// @vitest-environment happy-dom
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { act } from 'react'
import { createRoot } from 'react-dom/client'
import { expect, it, vi } from 'vitest'
import { FakeSource } from './fake-event-source'
import { subscribeThreadStream } from './thread-registry'
import { useThreadStream } from './use-thread-stream'

Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true })

it('shares a thread across sibling subscribers, delivers to both, resumes and closes at the last departure', () => {
  vi.useFakeTimers()
  const sources: FakeSource[] = []
  vi.stubGlobal('EventSource', class extends FakeSource {
    constructor(url: string) { super(url); sources.push(this) }
  })
  const client = new QueryClient()
  const root = createRoot(document.createElement('div'))
  const events: number[][] = [[], []]
  const notices: string[][] = [[], []]
  function Watcher({ id }: { id: number }) {
    useThreadStream('t1', event => events[id].push(event.seq), notice => notices[id].push(notice.type))
    return null
  }
  const render = (first: boolean) => root.render(<QueryClientProvider client={client}>
    {first && <Watcher key="first" id={0} />}<Watcher key="second" id={1} />
  </QueryClientProvider>)
  try {
    act(() => render(true))
    expect(sources.filter(s => !s.closed)).toHaveLength(1)
    act(() => {
      sources[0].durable(4)
      sources[0].caughtUp(4)
      sources[0].emit('commands', JSON.stringify({ thread_id: 't1', commands: [] }))
    })
    expect(events).toEqual([[4], [4]])
    expect(notices).toEqual([['commands'], ['commands']])
    act(() => render(false))
    expect(sources[0].closed).toBe(false)
    act(() => { sources[0].fail(); vi.runOnlyPendingTimers() })
    expect(sources[1].param('after')).toBe('4')
    act(() => { sources[1].durable(4); sources[1].durable(5) })
    expect(events).toEqual([[4], [4, 5]])
    act(() => root.unmount())
    expect(sources.every(s => s.closed)).toBe(true)
    act(() => vi.runOnlyPendingTimers())
    expect(sources).toHaveLength(2)
  } finally { client.clear(); vi.unstubAllGlobals(); vi.useRealTimers() }
})

it('a repeated departure never closes a channel opened after it', () => {
  const sources: FakeSource[] = []
  vi.stubGlobal('EventSource', class extends FakeSource {
    constructor(url: string) { super(url); sources.push(this) }
  })
  const client = new QueryClient()
  try {
    const leave = subscribeThreadStream(client, 't1', {})
    leave()
    const stay = subscribeThreadStream(client, 't1', {})
    leave()
    expect(sources.map(s => s.closed)).toEqual([true, false])
    const again = subscribeThreadStream(client, 't1', {})
    expect(sources).toHaveLength(2)
    again(); stay()
    expect(sources.every(s => s.closed)).toBe(true)
  } finally { client.clear(); vi.unstubAllGlobals() }
})
