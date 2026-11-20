// @vitest-environment happy-dom
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { act } from 'react'
import { createRoot } from 'react-dom/client'
import { expect, it, vi } from 'vitest'
import { classifyRefresh, dispatchRefresh } from './refresh'
import { groupKeys } from '@/api/groups'
import { FakeSource } from '@/test/fake-event-source'
import { usePlanEvents } from './use-plan-events'
import { useProjectEvents } from './use-project-events'
import { useThreadStream } from './use-thread-stream'
import { waitingPermissionsQuery } from '@/api/permissions'
import { threadKeys } from '@/api/threads'
import { designKeys } from '@/api/design'
import { attemptsQuery, taskRunQuery } from '@/api/runs'
import { elementRevisionsQuery, elementsQuery, projectStateQuery } from '@/api/project-state'

Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true })

it('refreshes task choices on TaskExecutorChosen without reading attempt history again', () => {
  const client = new QueryClient()
  const taskKey = taskRunQuery('w1', 1, 1).queryKey
  const attemptsKey = attemptsQuery('w1', 1, 1).queryKey
  for (const key of [taskKey, attemptsKey]) client.getQueryCache().build(client, { queryKey: key })
  dispatchRefresh(client, classifyRefresh({
    kind: 'TaskExecutorChosen', seq: 1, threadId: null, operationId: null,
    payload: { project_id: 'p1', plan_id: 'p1', task: 1 },
  }))
  expect(client.getQueryState(taskKey)?.isInvalidated).toBe(true)
  expect(client.getQueryState(attemptsKey)?.isInvalidated).toBe(false)
  client.clear()
})

it.each(['AgentMessage', 'UserMessage', 'ToolCall'])('only assistant entries refresh conversation order: %s', kind => {
  const client = new QueryClient()
  const key = threadKeys.list('p1')
  client.getQueryCache().build(client, { queryKey: key })
  dispatchRefresh(client, classifyRefresh({
    kind: 'ThreadEntryAppended', seq: 1, threadId: 't1', operationId: null,
    payload: { ordinal: 1, kind },
  }))
  expect(client.getQueryState(key)?.isInvalidated).toBe(kind === 'AgentMessage')
  client.clear()
})

it.each(['DecisionProposed', 'DecisionAccepted'])('decision_events_refetch_the_decisions: %s', kind => {
  const client = new QueryClient()
  const key = designKeys.decisions('p1')
  client.getQueryCache().build(client, { queryKey: key })
  dispatchRefresh(client, classifyRefresh({
    kind, seq: 1, threadId: null, operationId: null, projectId: 'p1', payload: { project_id: 'p1' },
  }))
  expect(client.getQueryState(key)?.isInvalidated).toBe(true)
  client.clear()
})

it('a MessageAdded refreshes its group, its log and the list', async () => {
  const client = new QueryClient()
  const keys = [groupKeys.detail('g1'), groupKeys.messages('g1'), groupKeys.list, groupKeys.detail('other')]
  for (const key of keys) client.getQueryCache().build(client, { queryKey: key })
  dispatchRefresh(client, classifyRefresh({
    kind: 'MessageAdded', seq: 12, threadId: null, operationId: null,
    projectId: 'p1', payload: { group_id: 'g1', seq: 2 },
  }))
  expect(keys.map(key => client.getQueryState(key)?.isInvalidated)).toEqual([true, true, true, false])
  client.clear()
})

it('batches replayed group messages until the project is caught up', () => {
  const client = new QueryClient()
  const sources: FakeSource[] = []
  const root = createRoot(document.createElement('div'))
  vi.stubGlobal('EventSource', class extends FakeSource { constructor(url: string) { super(url); sources.push(this) } })
  const keys = [groupKeys.detail('g1'), groupKeys.messages('g1'), groupKeys.list]
  const invalidated = () => keys.map(key => client.getQueryState(key)?.isInvalidated)
  function View() { useProjectEvents('p1'); return null }
  try {
    for (const key of keys) client.getQueryCache().build(client, { queryKey: key })
    act(() => root.render(<QueryClientProvider client={client}><View /></QueryClientProvider>))
    act(() => {
      sources[0].durable(1, 'MessageAdded', null, { group_id: 'g1', seq: 1 })
      sources[0].durable(2, 'MessageAdded', null, { group_id: 'g1', seq: 2 })
    })
    expect(invalidated()).toEqual([false, false, false])
    act(() => sources[0].caughtUp(2))
    expect(invalidated()).toEqual([true, true, true])
  } finally { act(() => root.unmount()); client.clear(); vi.unstubAllGlobals() }
})

it('PermissionsChanged on the project stream refetches the waiting list and reconnect reconciles it', async () => {
  const client = new QueryClient()
  const sources: FakeSource[] = []
  vi.stubGlobal('EventSource', class extends FakeSource { constructor(url: string) { super(url); sources.push(this) } })
  let reads = 0
  const options = { ...waitingPermissionsQuery, queryFn: async () => { reads++; return [] } }
  const root = createRoot(document.createElement('div'))
  function View() { useProjectEvents('p1'); return null }
  const observer = client.getQueryCache()
  try {
    await client.fetchQuery(options)
    const { QueryObserver } = await import('@tanstack/react-query')
    const watch = new QueryObserver(client, options)
    const release = watch.subscribe(() => {})
    act(() => root.render(<QueryClientProvider client={client}><View /></QueryClientProvider>))
    const project = sources[0]
    act(() => project.caughtUp(0)); await new Promise(resolve => setTimeout(resolve, 0))
    const before = reads
    act(() => project.durable(1, 'PermissionsChanged', null, { request_id: 'r1', state: 'asked' }))
    await new Promise(resolve => setTimeout(resolve, 0))
    expect(reads).toBeGreaterThan(before)
    const afterEvent = reads
    act(() => project.caughtUp(1)); await new Promise(resolve => setTimeout(resolve, 0))
    expect(reads).toBeGreaterThan(afterEvent)
    release()
  } finally { act(() => root.unmount()); observer.clear(); client.clear(); vi.unstubAllGlobals() }
})

it.each(['WorkflowDraftStarted', 'WorkflowEdited', 'WorkflowFrozen', 'PlanArchived', 'PlanUnarchived'])('classifies the full plan refresh union for %s', kind => {
  expect(classifyRefresh({ kind, seq: 1, threadId: 't1', operationId: null, payload: {}, projectId: 'p1' })).toEqual([
    { kind: 'workflows' }, { kind: 'design', projectId: 'p1' }, { kind: 'agreement-versions', projectId: 'p1' },
    { kind: 'project-state', projectId: 'p1' },
  ])
})

it('ignores unknown kinds and reads a supplied project identifier without cache access', () => {
  expect(classifyRefresh({ kind: 'unknown', seq: 1, threadId: null, operationId: null, payload: null })).toEqual([])
  expect(classifyRefresh({ kind: 'PlanArchived', seq: 1, threadId: 't1', operationId: null, payload: { project_id: 'p2' } })).toEqual([
    { kind: 'workflows' }, { kind: 'design', projectId: 'p2' }, { kind: 'agreement-versions', projectId: 'p2' },
    { kind: 'project-state', projectId: 'p2' },
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

it('refreshes once per event however many views share a stream, and a late view on a live stream refetches nothing it already watches', async () => {
  const client = new QueryClient()
  const sources: FakeSource[] = []
  vi.stubGlobal('EventSource', class extends FakeSource {
    constructor(url: string) { super(url); sources.push(this) }
  })
  const invalidate = vi.spyOn(client, 'invalidateQueries')
  const calls = (head: string) => invalidate.mock.calls.filter(([filters]) => filters?.queryKey?.[0] === head).length
  const root = createRoot(document.createElement('div'))
  function Card() { usePlanEvents('t1', 'p1'); return null }
  function View() { useProjectEvents('p1'); return null }
  const render = (cards: number) => root.render(<QueryClientProvider client={client}>
    {Array.from({ length: cards }, (_, i) => <Card key={i} />)}<View /><View />
  </QueryClientProvider>)
  try {
    act(() => render(3))
    const project = sources.find(s => s.url.includes('/projects/'))!
    const thread = sources.find(s => s.param('thread_id') === 't1')!
    act(() => { thread.caughtUp(0); project.caughtUp(0) })
    invalidate.mockClear()
    act(() => thread.durable(1, 'WorkflowEdited'))
    expect(calls('workflows')).toBe(1)
    invalidate.mockClear()
    act(() => project.durable(1, 'ProjectDesignChanged'))
    expect(invalidate.mock.calls.filter(([filters]) => filters?.queryKey?.[2] === 'design').length).toBe(1)
    invalidate.mockClear()
    act(() => render(6))
    expect(invalidate).not.toHaveBeenCalled()
  } finally { act(() => root.unmount()); client.clear(); vi.unstubAllGlobals() }
})

it('a conversation joining a live writer stream reads its entries and operations once', () => {
  const client = new QueryClient()
  const sources: FakeSource[] = []
  vi.stubGlobal('EventSource', class extends FakeSource {
    constructor(url: string) { super(url); sources.push(this) }
  })
  const invalidate = vi.spyOn(client, 'invalidateQueries')
  const heads = () => invalidate.mock.calls.map(([filters]) => (filters?.queryKey ?? []).join('/'))
  const root = createRoot(document.createElement('div'))
  function Card() { usePlanEvents('t1', 'p1'); return null }
  function Conversation() { useThreadStream('t1', undefined, undefined, 'p1'); return null }
  const render = (conversation: boolean) => root.render(<QueryClientProvider client={client}>
    <Card />{conversation && <Conversation />}
  </QueryClientProvider>)
  try {
    act(() => render(false))
    act(() => sources[0].caughtUp(0))
    invalidate.mockClear()
    act(() => render(true))
    expect(sources).toHaveLength(1)
    expect(heads()).toEqual(['threads/t1/entries', 'threads/t1/operations'])
  } finally { act(() => root.unmount()); client.clear(); vi.unstubAllGlobals() }
})

const STATE_KINDS = [
  'WorkflowDraftStarted', 'WorkflowEdited', 'WorkflowFrozen', 'PlanArchived', 'PlanUnarchived',
  'DesignElementChanged', 'CodeRecordChanged', 'TaskProgressChanged', 'ProjectDesignChanged',
  'MapProposed', 'RoadmapProposed', 'RoadmapApproved',
]

it.each(STATE_KINDS)('project_state_events_refetch_the_state_and_the_elements: %s', kind => {
  const client = new QueryClient()
  const keys = [projectStateQuery('p1').queryKey, projectStateQuery('p1', 'web').queryKey, elementsQuery('p1').queryKey]
  for (const queryKey of keys) client.getQueryCache().build(client, { queryKey })
  dispatchRefresh(client, classifyRefresh({
    kind, seq: 1, threadId: null, operationId: null, projectId: 'p1', payload: { project_id: 'p1' },
  }))
  expect(keys.map(key => client.getQueryState(key)?.isInvalidated)).toEqual([true, true, true])
  client.clear()
})

it('a_design_element_change_refetches_only_that_elements_revisions', () => {
  const client = new QueryClient()
  const keys = [elementRevisionsQuery('p1', 'E3').queryKey, elementRevisionsQuery('p1', 'E4').queryKey]
  for (const queryKey of keys) client.getQueryCache().build(client, { queryKey })
  dispatchRefresh(client, classifyRefresh({
    kind: 'DesignElementChanged', seq: 1, threadId: null, operationId: null,
    payload: { project_id: 'p1', element_id: 'E3', revision: 2 },
  }))
  expect(keys.map(key => client.getQueryState(key)?.isInvalidated)).toEqual([true, false])
  client.clear()
})

it('task_progress_and_record_changes_name_only_the_project_state', () => {
  const only = [{ kind: 'project-state', projectId: 'p1' }]
  expect(classifyRefresh({
    kind: 'TaskProgressChanged', seq: 1, threadId: null, operationId: null,
    payload: { project_id: 'p1', workflow_id: 'w1', version: 1, task: 1, attempt_id: null },
  })).toEqual(only)
  expect(classifyRefresh({
    kind: 'CodeRecordChanged', seq: 2, threadId: null, operationId: null, payload: { project_id: 'p1' },
  })).toEqual(only)
})

it('replayed_state_events_reconcile_the_state_once_at_caught_up', () => {
  const client = new QueryClient()
  const sources: FakeSource[] = []
  vi.stubGlobal('EventSource', class extends FakeSource { constructor(url: string) { super(url); sources.push(this) } })
  const root = createRoot(document.createElement('div'))
  const invalidate = vi.spyOn(client, 'invalidateQueries')
  const reads = () => invalidate.mock.calls.filter(([filters]) => filters?.queryKey?.[3] === 'reads').length
  function View() { useProjectEvents('p1'); return null }
  try {
    client.getQueryCache().build(client, { queryKey: projectStateQuery('p1').queryKey })
    act(() => root.render(<QueryClientProvider client={client}><View /></QueryClientProvider>))
    const project = sources.find(s => s.url.includes('/projects/'))!
    act(() => {
      project.durable(1, 'TaskProgressChanged', null, { project_id: 'p1', workflow_id: 'w1', version: 1, task: 1, attempt_id: null })
      project.durable(2, 'CodeRecordChanged', null, { project_id: 'p1' })
      project.durable(3, 'DesignElementChanged', null, { project_id: 'p1', element_id: 'E1', revision: 1 })
    })
    expect(reads()).toBe(0)
    act(() => project.caughtUp(3))
    expect(reads()).toBe(1)
    act(() => project.durable(4, 'TaskProgressChanged', null, { project_id: 'p1', workflow_id: 'w1', version: 1, task: 1, attempt_id: 'a1' }))
    expect(reads()).toBe(2)
  } finally { act(() => root.unmount()); client.clear(); vi.unstubAllGlobals() }
})
