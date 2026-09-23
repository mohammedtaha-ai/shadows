import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { FakeSource } from './fake-event-source'
import { type Options, ThreadStream } from './thread-stream'

function harness(options: Partial<Options> = {}) {
  const sources: FakeSource[] = []
  const applied: number[] = []
  const stream = new ThreadStream({
    url: (after) => `http://daemon/api/subscribe?thread_id=t&after=${after}`,
    connect: (url) => {
      const source = new FakeSource(url)
      sources.push(source)
      return source
    },
    onDurable: (event) => applied.push(event.seq),
    ...options,
  })
  const current = (): FakeSource => {
    const source = sources[sources.length - 1]
    if (source === undefined) throw new Error('no connection was opened')
    return source
  }
  return { stream, sources, applied, current }
}

const afterOf = (source: FakeSource) => source.param('after')

beforeEach(() => vi.useFakeTimers())
afterEach(() => vi.useRealTimers())

describe('ThreadStream', () => {
  it('applies a durable event replayed after a reconnect only once', () => {
    const { stream, applied, current } = harness()
    stream.start()
    current().durable(1)
    current().durable(2)
    current().fail()
    vi.runOnlyPendingTimers()
    // A daemon that resends from an older cursor must not double-apply.
    current().durable(2)
    current().durable(3)

    expect(applied).toEqual([1, 2, 3])
    expect(stream.getState().lastSeq).toBe(3)
  })

  it('reconnects after the latest applied seq, not the one it started from', () => {
    const { stream, sources, current } = harness({ after: 4 })
    stream.start()
    expect(afterOf(current())).toBe('4')
    current().durable(5)
    current().durable(6)
    current().emit('fatal', 'journal read failed')
    expect(stream.getState().connection).toBe('reconnecting')
    vi.runOnlyPendingTimers()

    expect(sources).toHaveLength(2)
    expect(sources[0]?.closed).toBe(true)
    expect(afterOf(current())).toBe('6')
  })

  it('resubscribes at once from lastSeq when the daemon says it lagged', () => {
    const { stream, sources, current } = harness()
    stream.start()
    current().durable(1)
    current().emit('caught-up', '1')
    current().emit('delta', JSON.stringify({ op: 'a', text: 'partial' }))
    current().emit('lagged')

    // No timer ran: a lag is not a failure and is not backed off.
    expect(sources).toHaveLength(2)
    expect(sources[0]?.closed).toBe(true)
    expect(afterOf(current())).toBe('1')
    expect(stream.getState().connection).toBe('reconnecting')
    // Dropped deltas are never resent, so the gapped text is not kept.
    expect(stream.getState().streaming).toEqual({})
  })

  it("keeps each operation's streamed text apart", () => {
    const { stream, current } = harness()
    stream.start()
    current().emit('delta', JSON.stringify({ op: 'a', text: 'Hel' }))
    current().emit('delta', JSON.stringify({ op: 'b', text: 'other' }))
    current().emit('delta', JSON.stringify({ op: 'a', text: 'lo' }))

    expect(stream.getState().streaming).toEqual({ a: 'Hello', b: 'other' })
  })

  it("clears an operation's streamed text and label at its turn end", () => {
    const { stream, current } = harness()
    stream.start()
    current().emit('delta', JSON.stringify({ op: 'a', text: 'done' }))
    current().emit('meta', JSON.stringify({ op: 'a', label: 'system/init' }))
    current().emit('delta', JSON.stringify({ op: 'b', text: 'still going' }))
    current().emit(
      'turn-end',
      JSON.stringify({ op: 'a', subtype: 'success', stop_reason: 'end_turn' }),
    )

    const state = stream.getState()
    expect(state.streaming).toEqual({ b: 'still going' })
    expect(state.labels).toEqual({})
    expect(state.lastTurnEnd).toEqual({ op: 'a', subtype: 'success', stopReason: 'end_turn' })
  })

  it('is loading history until caught-up, then live', () => {
    const caughtUpAt: number[] = []
    const { stream, current } = harness({ onCaughtUp: (seq) => caughtUpAt.push(seq) })
    stream.start()
    current().durable(1)
    expect(stream.getState()).toMatchObject({ connection: 'connecting', caughtUp: false })
    current().emit('caught-up', '1')

    expect(stream.getState()).toMatchObject({ connection: 'live', caughtUp: true })
    expect(caughtUpAt).toEqual([1])
  })

  it('backs off with a cap, gives up after maxFailures, and retry() starts over', () => {
    const { stream, sources, current } = harness({
      baseDelayMs: 100,
      maxDelayMs: 300,
      maxFailures: 4,
    })
    stream.start()
    const waits: number[] = []
    for (let failure = 1; failure <= 4; failure += 1) {
      current().fail()
      const opened = sources.length
      let waited = 0
      while (sources.length === opened) {
        vi.advanceTimersByTime(50)
        waited += 50
      }
      waits.push(waited)
    }
    expect(waits).toEqual([100, 200, 300, 300])

    current().fail()
    expect(stream.getState().connection).toBe('failed')
    vi.advanceTimersByTime(60_000)
    expect(sources).toHaveLength(5)

    stream.retry()
    expect(sources).toHaveLength(6)
    expect(stream.getState().connection).toBe('connecting')
  })

  it('stops reconnecting once closed', () => {
    const { stream, sources, current } = harness()
    stream.start()
    current().fail()
    stream.close()
    vi.advanceTimersByTime(60_000)

    expect(sources).toHaveLength(1)
    expect(sources[0]?.closed).toBe(true)
  })

  it('fails without retrying on a frame it cannot read', () => {
    const { stream, sources, current } = harness()
    stream.start()
    current().emit('durable', '{"seq":"one"}')
    vi.advanceTimersByTime(60_000)

    expect(stream.getState().connection).toBe('failed')
    expect(stream.getState().problem).toMatch(/malformed `durable` frame/)
    expect(sources).toHaveLength(1)
  })
})
