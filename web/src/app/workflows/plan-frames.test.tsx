// @vitest-environment happy-dom
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { act } from 'react'
import { createRoot } from 'react-dom/client'
import { expect, it, vi } from 'vitest'
import { FakeSource } from '@/stream/fake-event-source'
import { usePlanFrames } from './plan-frames'

it.each([
  ['WorkflowDraftStarted', true], ['WorkflowEdited', true], ['WorkflowFrozen', true],
  ['PlanArchived', false], ['PlanUnarchived', false],
] as const)('pins writer-thread refresh for %s', async (kind, refreshes) => {
  const client = new QueryClient()
  const keys = [
    ['workflows', 'map', 'p1'], ['workflows', 'list', 'p1', { archived: false }],
    ['workflows', 'versions', 'plan1'], ['workflows', 'plan', 'w1'],
    ['workflows', 'plan', 'other'],
  ]
  const sources: FakeSource[] = []
  vi.stubGlobal('EventSource', class extends FakeSource {
    constructor(url: string) { super(url); sources.push(this) }
  })
  const root = createRoot(document.createElement('div'))
  function Watcher() { usePlanFrames('t1'); return null }
  try {
    act(() => root.render(<QueryClientProvider client={client}><Watcher /></QueryClientProvider>))
    act(() => sources[0].caughtUp(0))
    for (const queryKey of keys) await client.fetchQuery({ queryKey, queryFn: async () => ({ project_id: 'p1', plan_id: 'plan1' }), staleTime: Infinity })
    act(() => sources[0].durable(1, kind, null, { plan_id: 'plan1', workflow_id: 'w1', project_id: 'p1' }))
    expect(keys.map(key => client.getQueryState(key)?.isInvalidated)).toEqual(keys.map(() => refreshes))
  } finally { act(() => root.unmount()); client.clear(); vi.unstubAllGlobals() }
})
