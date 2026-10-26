// @vitest-environment happy-dom
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { act } from 'react'
import { createRoot } from 'react-dom/client'
import { expect, it, vi } from 'vitest'
import { classifyRefresh } from './refresh'
import { FakeSource } from './fake-event-source'
import { usePlanEvents } from './use-plan-events'
import { useProjectEvents } from './use-project-events'
import { useThreadStream } from './use-thread-stream'

Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true })

it.each(['WorkflowDraftStarted', 'WorkflowEdited', 'WorkflowFrozen', 'PlanArchived', 'PlanUnarchived'])('classifies the full plan refresh union for %s', kind => {
  expect(classifyRefresh({ kind, seq: 1, threadId: 't1', operationId: null, payload: {}, projectId: 'p1' })).toEqual([
    { kind: 'workflows' }, { kind: 'design', projectId: 'p1' }, { kind: 'agreement-versions', projectId: 'p1' },
  ])
})

it('ignores unknown kinds and reads a supplied project identifier without cache access', () => {
  expect(classifyRefresh({ kind: 'unknown', seq: 1, threadId: null, operationId: null, payload: null })).toEqual([])
  expect(classifyRefresh({ kind: 'PlanArchived', seq: 1, threadId: 't1', operationId: null, payload: { project_id: 'p2' } })).toEqual([
    { kind: 'workflows' }, { kind: 'design', projectId: 'p2' }, { kind: 'agreement-versions', projectId: 'p2' },
  ])
})

it('preserves project and writer delivery with overlapping sequences, batching replayed archive and unarchive', async () => {
  const client = new QueryClient()
  const sources: FakeSource[] = []
  vi.stubGlobal('EventSource', class extends FakeSource {
    constructor(url: string) { super(url); sources.push(this) }
  })
  const root = createRoot(document.createElement('div'))
  const seen: number[] = []
  const keys = [
    ['workflows', 'plan', 'w1'], ['workflows', 'map', 'other'],
    ['projects', 'p1', 'design', 'vision'], ['projects', 'p1', 'agreement', 'a1', 1],
  ]
  const seed = async () => {
    for (const queryKey of keys) await client.fetchQuery({ queryKey, queryFn: async () => ({}), staleTime: Infinity })
  }
  const invalidated = () => keys.map(key => client.getQueryState(key)?.isInvalidated)
  function Watchers() {
    useProjectEvents('p1')
    usePlanEvents('t1', 'p1')
    useThreadStream('t1', event => seen.push(event.seq), undefined, 'p1')
    return null
  }
  try {
    act(() => root.render(<QueryClientProvider client={client}><Watchers /></QueryClientProvider>))
    expect(sources).toHaveLength(2)
    const project = sources.find(s => s.url.includes('/projects/'))!
    const thread = sources.find(s => s.param('thread_id') === 't1')!
    await seed()
    act(() => {
      thread.durable(7, 'PlanArchived', null, {})
      thread.durable(8, 'PlanUnarchived', null, {})
      project.durable(7, 'PlanArchived', null, { plan_id: 'plan1', workflow_id: 'w1' })
    })
    expect(invalidated()).toEqual([false, false, false, false])
    act(() => thread.caughtUp(8))
    expect(invalidated()).toEqual([true, true, true, true])
    expect(seen).toEqual([7, 8])
    await seed()
    act(() => project.caughtUp(7))
    expect(invalidated()).toEqual([true, true, true, true])
    await seed()
    act(() => project.durable(9, 'PlanUnarchived', null, { plan_id: 'plan1', workflow_id: 'w1' }))
    expect(invalidated()).toEqual([true, true, true, true])
    await seed()
    act(() => thread.durable(9, 'WorkflowFrozen', null, {}))
    expect(invalidated()).toEqual([true, true, true, true])
    expect(seen).toEqual([7, 8, 9])
  } finally { act(() => root.unmount()); client.clear(); vi.unstubAllGlobals() }
})
