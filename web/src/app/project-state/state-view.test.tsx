// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, until } from '@/test/test-app'
import type { DesignElement, ElementView, PartState, PlanProgress, ProjectStateView } from '@/api/project-state'

let app: TestApp | null = null
afterEach(() => { app?.unmount(); app = null })

const plan = (over: Partial<PlanProgress> = {}): PlanProgress => ({
  plan_id: 'plan1', title: 'Login plan', version: 1, workflow_id: 'w1', draft: null,
  tasks: { total: 4, done: 1, started: 2, not_started: 1 }, ...over,
})
const part = (over: Partial<PartState> = {}): PartState => ({
  part_id: 'web', title: 'Web client', kind: 'frontend', parent: null,
  elements: { planned: 1, built: 2, gap: 3, unchecked: 4 }, plans: [plan()], plans_more: 0, ...over,
})
const empty: ProjectStateView = {
  project_id: 'p1', stage: 'plans', map_revision: 5, missing: [], parts: [], parts_more: 0,
  unassigned: [], unassigned_more: 0, gaps: [], gaps_more: 0,
}
const view = (id: string, name: string, state: ElementView['state']['state'], behind = false): ElementView => {
  const element: DesignElement = {
    id, revision: 1, project_id: 'p1', part_id: 'web', outcomes: [], deleted: false, written_by: 't1',
    created_at: '2026-11-01T00:00:00Z', content: { kind: 'table', name, purpose: 'Stores rows', columns: [] },
  }
  return { element, state: { element_id: id, state, behind, built_revision: null, done: [], planned: [] } }
}
const stateOf = (over: Partial<ProjectStateView>): ProjectStateView => ({ ...empty, ...over })
const open = (url: string, state: (request: Request) => Response | ProjectStateView) =>
  startApp(url, { ...answers(), 'GET /api/projects/p1/state': (request: Request) => {
    const out = state(request)
    return out instanceof Response ? out : Response.json(out)
  } })
const project = (a: TestApp) => a.sources.filter(source => source.url.includes('/projects/p1/events'))

it('the_state_view_shows_stage_parts_plans_and_gaps', async () => {
  const a = app = await open('/projects/p1/state', () => stateOf({
    missing: ['roadmap'],
    parts: [part(), part({ part_id: 'api', title: 'Api', kind: null, parent: 'web', plans: [] })],
    unassigned: [plan({ plan_id: 'loose', title: 'Loose draft', workflow_id: null, draft: 'w9' })],
    gaps: [{ element_id: 'E1', kind: 'table', name: 'users', part_id: 'web', state: 'gap', why: 'no table found' }],
  }))
  await until(() => a.text().includes('Stage: plans'))
  expect(a.text()).toContain('Map revision 5')
  expect(a.text()).toContain('Missing: roadmap')
  const web = a.container.querySelector('[data-state-part="web"]')!
  expect(web.textContent).toContain('Web client')
  expect(web.textContent).toContain('1 planned · 2 built · 3 gap · 4 unchecked')
  expect(web.textContent).toContain('1 of 4 done · 2 started · 1 not started')
  expect(web.querySelector('a[href="/projects/p1/workflows/w1"]')?.textContent).toBe('Login plan')
  const api = a.container.querySelector('[data-state-part="api"]')!
  expect(api.textContent).toContain('No kind')
  expect(api.textContent).toContain('in Web client')
  const loose = a.container.querySelector('section[aria-label="Plans with no part"]')!
  expect(loose.querySelector('a[href="/projects/p1/workflows/w9"]')?.textContent).toBe('Loose draft (draft)')
  const gaps = a.container.querySelector('section[aria-label="Gaps"]')!
  expect(gaps.textContent).toContain('E1 · table · users')
  expect(gaps.textContent).toContain('no table found')
  expect(gaps.querySelector('[data-element-state="gap"]')).not.toBeNull()
  expect(gaps.querySelector('a[href*="view=tables"]')).not.toBeNull()
})

it('remainders_are_shown_as_cores_counts', async () => {
  const a = app = await open('/projects/p1/state', () => stateOf({
    parts: [part({ plans_more: 2 })], parts_more: 3, gaps_more: 1,
    gaps: [{ element_id: 'E1', kind: 'screen', name: 'Login', part_id: 'web', state: 'unchecked', why: 'not read' }],
  }))
  await until(() => a.text().includes('Stage: plans'))
  expect(a.text()).toContain('3 more parts')
  expect(a.text()).toContain('2 more plans')
  expect(a.text()).toContain('1 more gaps')
  expect(a.text()).not.toContain('No gaps.')
})

it('an_empty_project_says_the_map_comes_from_the_planner', async () => {
  const a = app = await open('/projects/p1/state', () => empty)
  await until(() => a.text().includes('No parts yet. The map comes from the Planner.'))
  expect(a.text()).toContain('No gaps.')
})

const detail = (elements: ElementView[]): ProjectStateView => stateOf({
  parts: [part()],
  part: {
    part_id: 'web', plans_more: 0, elements,
    plans: [{ plan_id: 'plan1', title: 'Login plan', version: 2, workflow_id: 'w1', tasks: [
      { number: 1, title: 'Add form', done: true, attempt: 'committed' },
      { number: 2, title: 'Wire it', done: false, attempt: 'failed' },
      { number: 3, title: 'Polish', done: false, attempt: null },
    ] }],
  },
})

it('a_part_shows_its_plans_tasks_and_elements', async () => {
  const urls: string[] = []
  const a = app = await open('/projects/p1/state?part=web', request => {
    urls.push(new URL(request.url).search)
    return detail([view('E1', 'users', 'built'), view('E2', 'sessions', 'planned', true)])
  })
  await until(() => a.container.querySelector('section[aria-label="Part detail"]') !== null)
  expect(urls[0]).toBe('?part_id=web')
  const section = a.container.querySelector('section[aria-label="Part detail"]')!
  const lines = [...section.querySelectorAll('li')].map(li => li.textContent)
  expect(lines).toContain('T1 Add formDone')
  expect(lines).toContain('T2 Wire itFailed')
  expect(lines).toContain('T3 PolishNot started')
  expect(section.querySelector('a[href="/projects/p1/workflows/w1/versions/2/tasks/1"]')).not.toBeNull()
  expect(section.querySelector('[data-element-state="built"]')).not.toBeNull()
  expect(section.querySelector('[data-element-state="planned"]')).not.toBeNull()
  expect(section.textContent).toContain('newer revision planned')
  expect(section.textContent).not.toContain('Show removed')
})

it('removed_elements_are_hidden_until_asked', async () => {
  const a = app = await open('/projects/p1/state?part=web', () => detail([view('E1', 'users', 'built'), view('E2', 'old', 'removed')]))
  await until(() => a.container.querySelector('section[aria-label="Part detail"]') !== null)
  const section = () => a.container.querySelector('section[aria-label="Part detail"]')!
  expect(section().textContent).toContain('users')
  expect(section().textContent).not.toContain('E2 · table · old')
  act(() => a.button('Show removed')!.click())
  await until(() => section().textContent!.includes('E2 · table · old'))
  expect(section().querySelector('[data-element-state="removed"]')).not.toBeNull()
  expect(a.button('Hide removed')).toBeDefined()
})

it('an_unknown_part_shows_cores_refusal', async () => {
  const a = app = await open('/projects/p1/state?part=nope', () =>
    Response.json({ code: 'INVALID_COMMAND', message: 'unknown part: nope' }, { status: 422 }))
  await until(() => a.text().includes('unknown part: nope'))
  expect(a.text()).toContain('INVALID_COMMAND')
  expect([...a.container.querySelectorAll('a')].some(link => link.textContent === 'All parts')).toBe(true)
})

it('an_unknown_project_shows_cores_not_found', async () => {
  const a = app = await open('/projects/p1/state', () =>
    Response.json({ code: 'INVALID_COMMAND', message: 'not found: project' }, { status: 404 }))
  await until(() => a.text().includes('not found: project'))
  expect(a.text()).toContain('INVALID_COMMAND')
})

const stateReads = (a: TestApp) => a.calls.filter(call => call === 'GET /api/projects/p1/state').length

it('task_progress_on_the_project_stream_moves_a_task_to_started_without_reload', async () => {
  let started = false
  const a = app = await open('/projects/p1/state', () => stateOf({
    parts: [part({ plans: [plan({ tasks: started ? { total: 1, done: 0, started: 1, not_started: 0 } : { total: 1, done: 0, started: 0, not_started: 1 } })] })],
  }))
  await until(() => a.text().includes('1 not started') && project(a).length > 0)
  act(() => project(a)[0].caughtUp(0))
  await until(() => stateReads(a) >= 2)
  await act(async () => { await new Promise(resolve => setTimeout(resolve, 20)) })
  const before = stateReads(a)
  started = true
  act(() => project(a)[0].durable(1, 'TaskProgressChanged', null,
    { project_id: 'p1', workflow_id: 'w1', version: 1, task: 1, attempt_id: 'a1' }))
  await until(() => a.text().includes('1 started'))
  expect(stateReads(a)).toBe(before + 1)
})

it('replayed_events_reconcile_the_state_once', async () => {
  const a = app = await open('/projects/p1/state', () => stateOf({ parts: [part()] }))
  await until(() => a.text().includes('Web client') && project(a).length > 0)
  const before = stateReads(a)
  act(() => {
    const source = project(a)[0]
    source.durable(1, 'TaskProgressChanged', null, { project_id: 'p1', workflow_id: 'w1', version: 1, task: 1, attempt_id: null })
    source.durable(2, 'CodeRecordChanged', null, { project_id: 'p1' })
    source.durable(3, 'DesignElementChanged', null, { project_id: 'p1', element_id: 'E1', revision: 1 })
  })
  await act(async () => { await new Promise(resolve => setTimeout(resolve, 20)) })
  expect(stateReads(a)).toBe(before)
  act(() => project(a)[0].caughtUp(3))
  await until(() => stateReads(a) === before + 1)
  await act(async () => { await new Promise(resolve => setTimeout(resolve, 20)) })
  expect(stateReads(a)).toBe(before + 1)
})

it('the_workspace_links_to_the_state', async () => {
  const a = app = await startApp('/projects/p1/workspace', answers())
  await until(() => [...a.container.querySelectorAll('a')].some(link => link.textContent === 'State'))
  const link = [...a.container.querySelectorAll('a')].find(item => item.textContent === 'State')!
  expect(link.getAttribute('href')!.endsWith('/projects/p1/state')).toBe(true)
})
