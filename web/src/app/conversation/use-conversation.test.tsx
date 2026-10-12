// @vitest-environment happy-dom
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { StrictMode, act, useEffect } from 'react'
import { type Root, createRoot } from 'react-dom/client'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { threadEntriesKey, threadOperationsKey } from '@/api/queries'
import { FakeSource } from '@/stream/fake-event-source'
import { useConversation } from './use-conversation'

Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true })

let sources: FakeSource[] = []
let root: Root | null = null
let shown: string | undefined

function Probe() {
  const text = useConversation('t1').reply?.text
  // An effect after every commit, as `messages.tsx` keeps the list scrolled.
  useEffect(() => {
    shown = text
  })
  return null
}

beforeEach(() => {
  sources = []
  shown = undefined
  vi.stubGlobal('fetch', () => new Promise(() => {}))
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
})

describe('useConversation', () => {
  // Each delta used to reach the reply through an effect, one nested render
  // per delta: a long reply passed React's update depth limit and threw.
  it('streams a long reply without exceeding the update depth', async () => {
    const client = new QueryClient({ defaultOptions: { queries: { staleTime: Infinity } } })
    client.setQueryData(threadEntriesKey('t1'), [])
    client.setQueryData(threadOperationsKey('t1'), [
      { id: 'op1', status_kind: 'Running', cancel_requested_at: null },
    ])
    root = createRoot(document.createElement('div'))
    const target = root
    act(() =>
      target.render(
        <StrictMode>
          <QueryClientProvider client={client}>
            <Probe />
          </QueryClientProvider>
        </StrictMode>,
      ),
    )
    const source = sources.find((s) => !s.closed)
    if (source === undefined) throw new Error('no open stream')
    act(() => source.caughtUp(0))
    // Frames as the browser delivers them: outside `act`, each a task of its own.
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: false })
    const errors: unknown[] = []
    const onError = (event: ErrorEvent) => errors.push(event.error)
    window.addEventListener('error', onError)
    vi.spyOn(console, 'error').mockImplementation((...args) => errors.push(String(args[0])))
    for (let i = 0; i < 120; i += 1) {
      source.emit('delta', JSON.stringify({ op: 'op1', text: 'x' }))
      await new Promise((resolve) => setTimeout(resolve, 0))
    }
    window.removeEventListener('error', onError)
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true })
    expect(errors).toEqual([])
    expect(shown).toBe('x'.repeat(120))
  })
})
