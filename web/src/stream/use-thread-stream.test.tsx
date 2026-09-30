// @vitest-environment happy-dom
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { StrictMode, act } from 'react'
import { type Root, createRoot } from 'react-dom/client'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { threadEntriesKey, threadOperationsKey } from '@/api/queries'
import { FakeSource } from './fake-event-source'
import { useThreadStream } from './use-thread-stream'

Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true })

let sources: FakeSource[] = []
let queryClient: QueryClient
let root: Root | null = null

const open = () => sources.filter((source) => !source.closed)
const current = (): FakeSource => {
  const [source, ...more] = open()
  if (source === undefined || more.length > 0) {
    throw new Error(`expected one open connection, found ${open().length}`)
  }
  return source
}

function Probe({ threadId }: { threadId: string }) {
  useThreadStream(threadId)
  return null
}

/** Renders the hook for `threadId` under StrictMode, as `main.tsx` does. */
function show(threadId: string): void {
  root ??= createRoot(document.createElement('div'))
  const target = root
  act(() =>
    target.render(
      <StrictMode>
        <QueryClientProvider client={queryClient}>
          <Probe threadId={threadId} />
        </QueryClientProvider>
      </StrictMode>,
    ),
  )
}

beforeEach(() => {
  vi.useFakeTimers()
  sources = []
  queryClient = new QueryClient()
  vi.stubGlobal(
    'EventSource',
    class extends FakeSource {
      constructor(url: string) {
        super(url)
        sources.push(this)
      }
    },
  )
})

afterEach(() => {
  act(() => root?.unmount())
  root = null
  vi.unstubAllGlobals()
  vi.useRealTimers()
})

describe('useThreadStream', () => {
  it('holds one connection under StrictMode and closes it on unmount', () => {
    show('t1')
    expect(current().param('thread_id')).toBe('t1')

    act(() => root?.unmount())
    root = null
    expect(open()).toEqual([])
  })

  it("switching thread closes the old one's connection and its pending reconnect", () => {
    show('t1')
    act(() => current().fail())
    expect(open()).toEqual([])

    show('t2')
    act(() => vi.advanceTimersByTime(60_000))

    // One open connection, for t2, and the last one opened: t1's timer is gone.
    expect(current().param('thread_id')).toBe('t2')
    expect(sources.at(-1)?.param('thread_id')).toBe('t2')
  })

  it('refetches entries at each caught-up, per live entry and live turn end, never per replayed one', () => {
    const invalidate = vi.spyOn(queryClient, 'invalidateQueries')
    const refetches = (key: readonly unknown[]) =>
      invalidate.mock.calls.filter(([filters]) => JSON.stringify(filters?.queryKey) === JSON.stringify(key)).length
    const entries = () => refetches(threadEntriesKey('t1'))
    const operations = () => refetches(threadOperationsKey('t1'))
    show('t1')

    act(() => {
      current().durable(1)
      current().durable(2, 'OperationCompleted', 'op')
    })
    expect(entries()).toBe(0)
    act(() => current().caughtUp(2))
    expect([entries(), operations()]).toEqual([1, 1])

    act(() => current().durable(3))
    expect(entries()).toBe(2)
    act(() => current().durable(4, 'OperationStarted', 'op'))
    expect(entries()).toBe(2)
    act(() => current().durable(5, 'OperationCompleted', 'op'))
    // A live turn end also refetches the operations, for its invocation.
    expect([entries(), operations()]).toEqual([3, 2])

    // A break: the events missed meanwhile are replayed, then one caught-up.
    act(() => current().fail())
    act(() => vi.runOnlyPendingTimers())
    expect(current().param('after')).toBe('5')
    act(() => {
      current().durable(6)
      current().durable(7, 'OperationCancelled', 'op')
      current().durable(8)
    })
    expect(entries()).toBe(3)
    act(() => current().caughtUp(8))
    expect([entries(), operations()]).toEqual([4, 3])
    expect(invalidate).toHaveBeenCalledTimes(7)
  })

  it('hands every durable event to its caller once, saying whether it was live', () => {
    const seen: [number, boolean][] = []
    function Watcher() {
      useThreadStream('t1', (event, live) => seen.push([event.seq, live]))
      return null
    }
    root ??= createRoot(document.createElement('div'))
    const target = root
    act(() =>
      target.render(
        <QueryClientProvider client={queryClient}>
          <Watcher />
        </QueryClientProvider>,
      ),
    )
    act(() => {
      current().durable(1)
      current().caughtUp(1)
      current().durable(2, 'OperationStarted', 'op')
      current().durable(2, 'OperationStarted', 'op')
    })
    expect(seen).toEqual([
      [1, false],
      [2, true],
    ])
  })

  it('reads the thread lists again on a retitle: live at once, replayed at the caught-up', () => {
    const invalidate = vi.spyOn(queryClient, 'invalidateQueries')
    const lists = () =>
      invalidate.mock.calls.filter(([filters]) => filters?.predicate !== undefined).length
    queryClient.setQueryData(['projects', 'p1', 'threads'], [])
    queryClient.setQueryData(['projects', 'p1'], {})
    show('t1')

    act(() => current().durable(1, 'ThreadRetitled', null, { title: 'A', source: 'harness' }))
    expect(lists()).toBe(0)
    act(() => current().caughtUp(1))
    expect(lists()).toBe(1)
    act(() => current().caughtUp(1))
    expect(lists()).toBe(1)
    act(() => current().durable(2, 'ThreadRetitled', null, { title: 'B', source: 'harness' }))
    expect(lists()).toBe(2)
    expect(queryClient.getQueryState(['projects', 'p1', 'threads'])?.isInvalidated).toBe(true)
    expect(queryClient.getQueryState(['projects', 'p1'])?.isInvalidated).toBe(false)
  })
})
