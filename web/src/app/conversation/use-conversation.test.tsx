// @vitest-environment happy-dom
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { StrictMode, act, useEffect } from 'react'
import { type Root, createRoot } from 'react-dom/client'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { threadEntriesKey, threadOperationsKey } from '@/api/threads'
import { FakeSource } from '@/test/fake-event-source'
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
  it.each([
    ['WorkflowDraftStarted', true], ['WorkflowEdited', true], ['WorkflowFrozen', true],
    ['PlanArchived', true], ['PlanUnarchived', true],
  ] as const)('pins conversation refresh for %s', async (kind, refreshes) => {
    const client = new QueryClient()
    const keys = [
      ['workflows', 'map', 'p1'], ['workflows', 'list', 'p1', { archived: false }],
      ['workflows', 'versions', 'plan1'], ['workflows', 'plan', 'w1'], ['workflows', 'plan', 'other'],
    ['projects', 'p1', 'design', 'vision'], ['projects', 'p1', 'agreement', 'a1', 1],
    ]
    function Watcher() { useConversation('t1', undefined, 'p1'); return null }
    root = createRoot(document.createElement('div'))
    act(() => root!.render(<QueryClientProvider client={client}><Watcher /></QueryClientProvider>))
    act(() => sources[0].caughtUp(0))
    for (const queryKey of keys) await client.fetchQuery({ queryKey, staleTime: Infinity, queryFn: async () => ({ project_id: 'p1', plan_id: 'plan1' }) })
    act(() => sources[0].durable(1, kind, null, { plan_id: 'plan1', workflow_id: 'w1' }))
    expect(keys.map(key => client.getQueryState(key)?.isInvalidated)).toEqual(keys.map(() => refreshes))
    client.clear()
  })

  // A long reply must not trigger nested renders per delta and exceed React's update depth.
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
