// One job: an EventSource a test drives by hand, frame by frame. Test-only;
// nothing outside a `*.test.ts` imports it.

import type { EventSourceLike } from './thread-stream'

export class FakeSource implements EventSourceLike {
  readonly url: string
  closed = false
  readonly #listeners = new Map<string, ((event: Event) => void)[]>()

  constructor(url: string) {
    this.url = url
  }

  addEventListener(type: string, listener: (event: Event) => void): void {
    this.#listeners.set(type, [...(this.#listeners.get(type) ?? []), listener])
  }

  close(): void {
    this.closed = true
  }

  emit(type: string, data = ''): void {
    for (const listener of this.#listeners.get(type) ?? []) {
      listener(new MessageEvent(type, { data }))
    }
  }

  fail(): void {
    for (const listener of this.#listeners.get('error') ?? []) listener(new Event('error'))
  }

  durable(seq: number, kind = 'ThreadEntryAppended'): void {
    this.emit(
      'durable',
      JSON.stringify({
        seq,
        kind,
        payload: JSON.stringify({ ordinal: seq, kind: 'UserMessage' }),
      }),
    )
  }

  /** The value of a query parameter of the URL this connection was opened with. */
  param(name: string): string | null {
    return new URL(this.url).searchParams.get(name)
  }
}
