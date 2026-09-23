// One job: the client half of spec §2.10 for one thread — durable replay,
// then live, each durable event applied at most once by `seq`, reconnecting
// from the last applied `seq` whenever the stream breaks.
//
// Plain TypeScript with the EventSource injected, so it is tested without a
// browser; `use-thread-stream.ts` is its React binding. Closing it closes the
// connection and nothing else: a client leaving never cancels work (spec §8.4
// case 7), and this module has no way to ask for that.

import {
  type DurableEvent,
  FrameError,
  type TurnEnd,
  parseCaughtUp,
  parseDelta,
  parseDurable,
  parseMeta,
  parseTurnEnd,
} from './frames'

/** `connecting` until the first `caught-up`; `live` after it; `reconnecting`
 * after a break until the next `caught-up`; `failed` once reconnecting has been
 * given up (see `Options.maxFailures`) or the daemon sent a frame this client
 * cannot read. Only `retry()` leaves `failed`. */
export type Connection = 'connecting' | 'live' | 'reconnecting' | 'failed'

export interface StreamState {
  readonly connection: Connection
  /** The durable replay has ended at least once: history is loaded. */
  readonly caughtUp: boolean
  /** The highest durable `seq` applied. The next connection asks for events after it. */
  readonly lastSeq: number
  /** Streamed text of each running turn, by operation id. Transient: text that
   * streamed while the connection was broken is never resent, so a break
   * clears it, and the turn's durable entry carries the whole reply. */
  readonly streaming: Readonly<Record<string, string>>
  /** The latest operational label of each running turn, by operation id. */
  readonly labels: Readonly<Record<string, string>>
  readonly lastTurnEnd: TurnEnd | null
  /** Why the last connection ended, for a person to read. */
  readonly problem: string | null
}

/** The part of the browser's `EventSource` this module uses. */
export interface EventSourceLike {
  addEventListener(type: string, listener: (event: Event) => void): void
  close(): void
}

export interface Options {
  /** The stream's URL for a connection resuming after `after`. */
  url: (after: number) => string
  /** Opens a connection. Defaults to the browser's `EventSource`. */
  connect?: (url: string) => EventSourceLike
  /** The last `seq` already applied; 0 replays the thread from its start. */
  after?: number
  /** Called once per durable event, in `seq` order, never twice for a `seq`. */
  onDurable?: (event: DurableEvent) => void
  /** Called at each `caught-up`, with the last applied `seq`. */
  onCaughtUp?: (lastSeq: number) => void
  /** Reconnect delay after the n-th consecutive failure: `baseDelayMs * 2^(n-1)`, capped. */
  baseDelayMs?: number
  maxDelayMs?: number
  /** Consecutive failures, without a `caught-up` between them, before giving up. */
  maxFailures?: number
}

export class ThreadStream {
  readonly #options: Required<Omit<Options, 'onDurable' | 'onCaughtUp'>> &
    Pick<Options, 'onDurable' | 'onCaughtUp'>
  readonly #listeners = new Set<() => void>()
  #state: StreamState
  #source: EventSourceLike | null = null
  #timer: ReturnType<typeof setTimeout> | null = null
  #failures = 0

  constructor(options: Options) {
    this.#options = {
      connect: (url) => new EventSource(url),
      after: 0,
      baseDelayMs: 500,
      maxDelayMs: 10_000,
      maxFailures: 10,
      ...options,
    }
    this.#state = {
      connection: 'connecting',
      caughtUp: false,
      lastSeq: this.#options.after,
      streaming: {},
      labels: {},
      lastTurnEnd: null,
      problem: null,
    }
  }

  /** Opens the stream. Calling it again after `close()` resumes from `lastSeq`. */
  start(): void {
    this.close()
    this.#open(this.#state.caughtUp ? 'reconnecting' : 'connecting')
  }

  /** Closes the connection and cancels any pending reconnect. Never cancels work. */
  close(): void {
    if (this.#timer !== null) {
      clearTimeout(this.#timer)
      this.#timer = null
    }
    this.#source?.close()
    this.#source = null
  }

  /** Leaves `failed` by reconnecting now, with a fresh failure budget. */
  readonly retry = (): void => {
    this.close()
    this.#failures = 0
    this.#open(this.#state.caughtUp ? 'reconnecting' : 'connecting')
  }

  readonly getState = (): StreamState => this.#state

  readonly subscribe = (listener: () => void): (() => void) => {
    this.#listeners.add(listener)
    return () => this.#listeners.delete(listener)
  }

  #open(connection: 'connecting' | 'reconnecting'): void {
    this.#set({ connection })
    const source = this.#options.connect(this.#options.url(this.#state.lastSeq))
    this.#source = source

    // Every handler ignores a source that is no longer current: frames can
    // still be queued on one this module has already closed.
    const on = (type: string, handle: (data: string) => void) => {
      source.addEventListener(type, (event) => {
        if (source !== this.#source) return
        if (!(event instanceof MessageEvent) || typeof event.data !== 'string') {
          this.#fail(`\`${type}\` arrived without text data`)
          return
        }
        try {
          handle(event.data)
        } catch (error) {
          if (!(error instanceof FrameError)) throw error
          this.#fail(error.message)
        }
      })
    }

    on('durable', (data) => this.#durable(parseDurable(data)))
    on('caught-up', (data) => {
      parseCaughtUp(data)
      this.#failures = 0
      this.#set({ connection: 'live', caughtUp: true, problem: null })
      this.#options.onCaughtUp?.(this.#state.lastSeq)
    })
    on('delta', (data) => {
      const { op, text } = parseDelta(data)
      const streaming = this.#state.streaming
      this.#set({ streaming: { ...streaming, [op]: (streaming[op] ?? '') + text } })
    })
    on('turn-end', (data) => {
      const turnEnd = parseTurnEnd(data)
      this.#set({
        streaming: without(this.#state.streaming, turnEnd.op),
        labels: without(this.#state.labels, turnEnd.op),
        lastTurnEnd: turnEnd,
      })
    })
    on('meta', (data) => {
      const { op, label } = parseMeta(data)
      this.#set({ labels: { ...this.#state.labels, [op]: label } })
    })
    // Transient frames were dropped, durable ones were not: resubscribe from
    // `lastSeq` at once. Not a failure, so no backoff.
    on('lagged', () => {
      this.close()
      this.#set({ streaming: {}, labels: {} })
      this.#open('reconnecting')
    })
    // The daemon could not read its journal and is ending the stream.
    on('fatal', (data) => this.#lost(`the daemon ended the stream: ${data}`))
    source.addEventListener('error', () => {
      if (source !== this.#source) return
      this.#lost('the connection to the daemon was lost')
    })
  }

  #durable(event: DurableEvent): void {
    if (event.seq <= this.#state.lastSeq) return
    this.#set({ lastSeq: event.seq })
    this.#options.onDurable?.(event)
  }

  /** The connection broke: reconnect from `lastSeq` after a capped backoff,
   * or give up once `maxFailures` breaks came without a `caught-up`. */
  #lost(problem: string): void {
    this.close()
    this.#failures += 1
    if (this.#failures > this.#options.maxFailures) {
      this.#set({ connection: 'failed', problem, streaming: {}, labels: {} })
      return
    }
    this.#set({ connection: 'reconnecting', problem, streaming: {}, labels: {} })
    const delay = Math.min(
      this.#options.baseDelayMs * 2 ** (this.#failures - 1),
      this.#options.maxDelayMs,
    )
    this.#timer = setTimeout(() => {
      this.#timer = null
      this.#open('reconnecting')
    }, delay)
  }

  /** A frame this client cannot read. Reconnecting would read it again, so stop. */
  #fail(problem: string): void {
    this.close()
    this.#set({ connection: 'failed', problem, streaming: {}, labels: {} })
  }

  #set(patch: Partial<StreamState>): void {
    this.#state = { ...this.#state, ...patch }
    for (const listener of this.#listeners) listener()
  }
}

function without(record: Readonly<Record<string, string>>, key: string): Record<string, string> {
  const rest = { ...record }
  delete rest[key]
  return rest
}
