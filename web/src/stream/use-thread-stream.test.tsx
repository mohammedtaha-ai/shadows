// @vitest-environment happy-dom
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { StrictMode, act } from 'react'
import { type Root, createRoot } from 'react-dom/client'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { threadEntriesKey } from '@/api/queries'
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

  it('refetches entries at each caught-up and per live entry, never per replayed one', () => {
    const invalidate = vi.spyOn(queryClient, 'invalidateQueries')
    const refetches = () => invalidate.mock.calls.length
    show('t1')

    act(() => {
      current().durable(1)
      current().durable(2)
    })
    expect(refetches()).toBe(0)
    act(() => current().emit('caught-up', '2'))
    expect(refetches()).toBe(1)

    act(() => current().durable(3))
    expect(refetches()).toBe(2)
    act(() => current().durable(4, 'OperationStarted'))
    expect(refetches()).toBe(2)

    // A break: the events missed meanwhile are replayed, then one caught-up.
    act(() => current().fail())
    act(() => vi.runOnlyPendingTimers())
    expect(current().param('after')).toBe('4')
    act(() => {
      current().durable(5)
      current().durable(6)
      current().durable(7)
    })
    expect(refetches()).toBe(2)
    act(() => current().emit('caught-up', '7'))
    expect(refetches()).toBe(3)

    for (const [filters] of invalidate.mock.calls) {
      expect(filters).toEqual({ queryKey: threadEntriesKey('t1') })
    }
  })
})
