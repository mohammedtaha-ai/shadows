// Tests and their app harness drive frames by hand without opening a real connection.

import type { EventSourceLike } from '@/stream/thread-stream'

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

  durable(
    seq: number,
    kind = 'ThreadEntryAppended',
    operationId: string | null = null,
    payload: unknown = { ordinal: seq, kind: 'UserMessage' },
  ): void {
    this.emit(
      'durable',
      JSON.stringify({ seq, kind, operation_id: operationId, thread_id: 't', payload }),
    )
  }

  caughtUp(seq: number): void {
    this.emit('caught-up', JSON.stringify({ seq }))
  }

  /** The value of a query parameter of the URL this connection was opened with. */
  param(name: string): string | null {
    return new URL(this.url).searchParams.get(name)
  }
}
