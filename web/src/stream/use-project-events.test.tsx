// @vitest-environment happy-dom
import { designKeys } from '@/api/design'
import { QueryClient, QueryClientProvider, useQuery } from '@tanstack/react-query'
import { act } from 'react'
import { createRoot } from 'react-dom/client'
import { expect, it, vi } from 'vitest'
import { FakeSource } from '@/test/fake-event-source'
import { useProjectEvents } from './use-project-events'
import { until } from '@/test/test-app'
import { startApp, typeInto } from '@/test/test-app'
import { answers } from '@/test/fake-daemon'

it.each(['WorkflowDraftStarted', 'WorkflowEdited', 'WorkflowFrozen', 'PlanArchived', 'PlanUnarchived'])('pins project refresh for %s', async kind => {
  const client = new QueryClient()
  const keys = [
    ['projects', 'p1', 'design', 'vision'], ['projects', 'p1', 'agreement', 'a1', 1],
    ['workflows', 'map', 'p1'], ['workflows', 'list', 'p1', { archived: false }],
    ['workflows', 'list', 'p1', { archived: true }], ['workflows', 'versions', 'plan1'],
    ['workflows', 'plan', 'w1'], ['workflows', 'plan', 'w0'], ['workflows', 'plan', 'neighbour'],
    ['workflows', 'map', 'p2'], ['workflows', 'plan', 'foreign'],
  ]
  const sources: FakeSource[] = []
  vi.stubGlobal('EventSource', class extends FakeSource {
    constructor(url: string) { super(url); sources.push(this) }
  })
  const root = createRoot(document.createElement('div'))
  function Watcher() { useProjectEvents('p1'); return null }
  try {
    act(() => root.render(<QueryClientProvider client={client}><Watcher /></QueryClientProvider>))
    act(() => sources[0].caughtUp(0))
    for (const queryKey of keys) await client.fetchQuery({ queryKey, staleTime: Infinity, queryFn: async () => ({ project_id: 'p1', plan_id: 'plan1' }) })
    act(() => sources[0].durable(1, kind, null, { plan_id: 'plan1', workflow_id: 'w1' }))
    expect(keys.map(key => client.getQueryState(key)?.isInvalidated)).toEqual(keys.map(() => true))
  } finally { act(() => root.unmount()); client.clear(); vi.unstubAllGlobals() }
})

it('independent tabs refresh every design slice, deduplicate and resume after disconnect', async () => {
  const sources: FakeSource[] = []
  vi.stubGlobal('EventSource', class extends FakeSource {
    constructor(url: string) { super(url); sources.push(this) }
  })
  const tabs = [document.createElement('div'), document.createElement('div')]
  const roots = tabs.map(tab => { document.body.append(tab); return createRoot(tab) })
  const clients = tabs.map(() => new QueryClient({ defaultOptions: { queries: { retry: false } } }))
  let revision = 0
  const reads = [0, 0]
  function Workspace({ tab, project = 'p1' }: { tab: number; project?: string }) {
    useProjectEvents(project)
    const vision = useQuery({ queryKey: designKeys.vision(project), queryFn: async () => { reads[tab]++; return revision } })
    const parts = useQuery({ queryKey: designKeys.collection(project, 'parts'), queryFn: async () => revision })
    const outcomes = useQuery({ queryKey: designKeys.collection(project, 'outcomes'), queryFn: async () => revision })
    return <p>{vision.data}:{parts.data}:{outcomes.data}</p>
  }
  const render = (tab: number, project = 'p1') => roots[tab].render(<QueryClientProvider client={clients[tab]}><Workspace tab={tab} project={project} /></QueryClientProvider>)
  try {
    clients.forEach(client => client.setQueryData(designKeys.vision('other'), 99))
    await act(async () => { render(0); render(1) })
    await until(() => sources.length === 2 && tabs.every(tab => tab.textContent === '0:0:0'))
    act(() => sources.forEach(source => source.caughtUp(0)))
    await until(() => reads.every(count => count === 2))
    revision = 1
    act(() => sources.forEach(source => source.durable(7, 'ProjectDesignChanged', null,
      { project_id: 'p1', revision, changed_parts: ['part'], changed_outcomes: ['outcome'], vision_changed: true })))
    await until(() => tabs.every(tab => tab.textContent === '1:1:1'))
    revision = 4
    act(() => sources.forEach(source => source.durable(8, 'ProjectStandardsSaved')))
    await until(() => tabs.every(tab => tab.textContent === '4:4:4'))
    const beforeProposal = [...reads]
    act(() => sources.forEach(source => source.durable(9, 'VisionProposed', null,
      { project_id: 'p1', proposal_id: 'proposal' })))
    await until(() => reads.every((count, tab) => count > beforeProposal[tab]))
    const beforeMap = [...reads]
    act(() => sources.forEach(source => source.durable(10, 'MapProposed', null,
      { project_id: 'p1', proposal_id: 'map' })))
    await until(() => reads.every((count, tab) => count > beforeMap[tab]))
    const beforeStructure = [...reads]
    act(() => sources.forEach(source => source.durable(11, 'ProjectStructureWritten', null,
      {project_id:'p1',map_version:2,paths:['SHADOWS.md']})))
    await until(() => reads.every((count, tab) => count > beforeStructure[tab]))
    const afterWrite = [...reads]
    act(() => sources.forEach(source => source.durable(7, 'ProjectDesignChanged')))
    await act(async () => { await new Promise(resolve => setTimeout(resolve, 10)) })
    expect(reads).toEqual(afterWrite)
    clients.forEach(client => expect(client.getQueryState(designKeys.vision('other'))?.isInvalidated).toBe(false))
    act(() => sources[0].fail())
    expect(sources[0].closed).toBe(true)
    await until(() => sources.length === 3)
    expect(sources[2].param('after')).toBe('11')
    revision = 2
    // A replayed event refreshes the disconnected tab before caught-up; duplicates stay ignored.
    act(() => { sources[2].durable(12, 'ProjectDesignChanged'); sources[2].caughtUp(12) })
    await until(() => tabs[0].textContent === '2:2:2')
    expect(tabs[1].textContent).toBe('4:4:4')
    revision = 3
    act(() => sources[2].caughtUp(12))
    await until(() => tabs[0].textContent === '3:3:3')
    await act(async () => render(0, 'p2'))
    await until(() => sources.length === 4 && tabs[0].textContent === '3:3:3')
    expect(sources[2].closed).toBe(true)
    expect(sources[3].url).toContain('/projects/p2/events')
    expect(sources[3].param('after')).toBe('0')
    expect(sources[1].closed).toBe(false)
  } finally {
    act(() => roots.forEach(root => root.unmount()))
    expect(sources.every(source => source.closed)).toBe(true)
    clients.forEach(client => client.clear()); tabs.forEach(tab => tab.remove()); vi.unstubAllGlobals()
  }
})

it.each([
  { view: 'map', collection: 'parts', entity: 'part', label: 'Part title', save: 'Save part', content: { title: 'أصل', responsibility: '', design: '', kind: null } },
  { view: 'roadmap', collection: 'outcomes', entity: 'outcome', label: 'Outcome title', save: 'Save outcome', content: { title: 'أصل', intended_result: '', acceptance: [] } },
])('$entity keeps dirty Arabic input through real disconnect, replay and a stale save', async ({ view, collection, entity, label, save, content }) => {
  let revision = 1
  let remoteTitle = 'أصل'
  const node = () => ({ id: 'n1', revision, ordinal: 0, parent: null, content: { ...content, title: remoteTitle } })
  const app = await startApp(`/projects/p1/workspace?view=${view}&${entity}=n1`, {
    ...answers(),
    [`GET /api/projects/p1/design/${collection}`]: () => Response.json({ revision, items: [node()], next: null }),
    [`GET /api/projects/p1/design/${collection}/n1`]: () => Response.json({ revision, [entity]: node(), ancestors: [], parts: [], plans: [] }),
    'GET /api/projects/p1/design/parts': () => Response.json({ revision, items: entity === 'part' ? [node()] : [], next: null }),
    'POST /api/projects/p1/design/edits': () => Response.json({ code: 'REVISION_CONFLICT', message: 'changed', current_revision: revision }, { status: 409 }),
  })
  const title = () => app.container.querySelector<HTMLInputElement>(`input[aria-label="${label}"]`)
  const projectSources = () => app.sources.filter(source => source.url.includes('/projects/p1/events'))
  try {
    await until(() => title()?.value === 'أصل' && projectSources().length === 1)
    act(() => projectSources()[0].caughtUp(0))
    typeInto(title()!, 'محلي غير محفوظ  ')
    act(() => projectSources()[0].fail())
    await until(() => projectSources().length === 2)
    revision = 2; remoteTitle = 'من تبويب آخر'
    act(() => {
      projectSources()[1].durable(9, 'ProjectDesignChanged', null, { project_id: 'p1', revision, changed_parts: ['n1'], changed_outcomes: ['n1'], vision_changed: false })
      projectSources()[1].caughtUp(9)
    })
    await until(() => app.button('Reload') !== undefined)
    expect(title()?.value).toBe('محلي غير محفوظ  ')
    act(() => app.button(save)?.click())
    await until(() => app.text().includes('changed') && app.button(save)?.disabled === false)
    expect(app.bodies.at(-1)).toMatchObject({ expected_revision: 1 })
    expect(title()?.value).toBe('محلي غير محفوظ  ')
    act(() => app.button('Reload')?.click())
    await until(() => title()?.value === remoteTitle)
    expect(projectSources()[0].closed).toBe(true)
  } finally { app.unmount() }
})
