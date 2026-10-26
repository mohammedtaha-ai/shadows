// @vitest-environment happy-dom
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { act } from 'react'
import { createRoot } from 'react-dom/client'
import { expect, it, vi } from 'vitest'
import { FakeSource } from '@/test/fake-event-source'
import { usePlanFrames } from './plan-frames'
import { useThreadStream } from '@/stream'

Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true })

it('shares writer watchers with a sibling conversation stream until the last watcher leaves', () => {
  const client = new QueryClient()
  const sources: FakeSource[] = []
  vi.stubGlobal('EventSource', class extends FakeSource {
    constructor(url: string) { super(url); sources.push(this) }
  })
  const root = createRoot(document.createElement('div'))
  const seen: number[] = []
  function Conversation() { useThreadStream('t1', e => seen.push(e.seq)); return null }
  function Writer() { usePlanFrames('t1', 'p1'); return null }
  const render = (conversation: boolean) => root.render(<QueryClientProvider client={client}>
    {conversation && <Conversation />}<Writer /><Writer />
  </QueryClientProvider>)
  try {
    act(() => render(true))
    expect(sources.filter(s => !s.closed)).toHaveLength(1)
    act(() => { sources[0].caughtUp(0); sources[0].durable(1, 'WorkflowEdited') })
    expect(seen).toEqual([1])
    act(() => render(false))
    expect(sources[0].closed).toBe(false)
    act(() => root.unmount())
    expect(sources[0].closed).toBe(true)
  } finally { client.clear(); vi.unstubAllGlobals() }
})

it.each([
  ['WorkflowDraftStarted', true], ['WorkflowEdited', true], ['WorkflowFrozen', true],
  ['PlanArchived', true], ['PlanUnarchived', true],
] as const)('pins writer-thread refresh for %s', async (kind, refreshes) => {
  const client = new QueryClient()
  const keys = [
    ['workflows', 'map', 'p1'], ['workflows', 'list', 'p1', { archived: false }],
    ['workflows', 'versions', 'plan1'], ['workflows', 'plan', 'w1'],
    ['workflows', 'plan', 'other'],
    ['projects', 'p1', 'design', 'vision'], ['projects', 'p1', 'agreement', 'a1', 1],
  ]
  const sources: FakeSource[] = []
  vi.stubGlobal('EventSource', class extends FakeSource {
    constructor(url: string) { super(url); sources.push(this) }
  })
  const root = createRoot(document.createElement('div'))
  function Watcher() { usePlanFrames('t1', 'p1'); return null }
  try {
    act(() => root.render(<QueryClientProvider client={client}><Watcher /></QueryClientProvider>))
    act(() => sources[0].caughtUp(0))
    for (const queryKey of keys) await client.fetchQuery({ queryKey, queryFn: async () => ({ project_id: 'p1', plan_id: 'plan1' }), staleTime: Infinity })
    act(() => sources[0].durable(1, kind, null, { plan_id: 'plan1', workflow_id: 'w1' }))
    expect(keys.map(key => client.getQueryState(key)?.isInvalidated)).toEqual(keys.map(() => refreshes))
  } finally { act(() => root.unmount()); client.clear(); vi.unstubAllGlobals() }
})
