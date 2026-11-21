// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, until } from '@/test/test-app'
import type { BuildState, DesignElement, ElementTask, ElementView, ProjectStateView } from '@/api/project-state'

let app: TestApp | null = null
afterEach(() => { app?.unmount(); app = null })

const base = { revision: 1, project_id: 'p1', part_id: 'web', outcomes: [], deleted: false, written_by: 't1', created_at: '2026-11-01T00:00:00Z' }
const task = (over: Partial<ElementTask> = {}): ElementTask => ({
  workflow_id: 'w1', version: 2, task: 3, revision: 1, action: 'create', file: 'src/db/sessions.rs', symbol_id: 'Session', symbol_state: 'found', ...over,
})
function table(id: string, name: string, state: BuildState, over: Partial<DesignElement> = {}, extra: Partial<ElementView['state']> = {}): ElementView {
  const element: DesignElement = { ...base, id, content: { kind: 'table', name, purpose: `${name} purpose`, columns: [
    { name: 'id', type_name: 'uuid', primary_key: true },
    ...(id === 'E2' ? [{ name: 'user_id', type_name: 'uuid', meaning: 'the owner', references: { table: 'E1', column: 'id' } }] : []),
  ] }, ...over }
  return { element, state: { element_id: id, state, behind: false, built_revision: null, done: [], planned: [], ...extra } }
}
const screen = (): ElementView => ({
  element: { ...base, id: 'E3', content: { kind: 'screen', name: 'Login', purpose: 'Sign in', route: '/login', leads_to: [] } },
  state: { element_id: 'E3', state: 'planned', behind: false, built_revision: null, done: [], planned: [] },
})
const state: ProjectStateView = {
  project_id: 'p1', stage: 'plans', map_revision: 1, missing: [], parts_more: 0, unassigned: [], unassigned_more: 0, gaps: [], gaps_more: 0,
  parts: [{ part_id: 'web', title: 'Web client', kind: 'frontend', parent: null, elements: { planned: 0, built: 0, gap: 0, unchecked: 0 }, plans: [], plans_more: 0 }],
}
const route = (url: string, elements: ElementView[], extra: Record<string, unknown> = {}) =>
  startApp(url, { ...answers(), 'GET /api/projects/p1/elements': elements, 'GET /api/projects/p1/state': state, ...extra })
const project = (a: TestApp) => a.sources.filter(source => source.url.includes('/projects/p1/events'))
const count = (a: TestApp, call: string) => a.calls.filter(item => item === call).length

it('the_er_diagram_draws_one_node_per_table_and_one_edge_per_foreign_key', async () => {
  const a = app = await route('/projects/p1/state?view=tables', [table('E1', 'users', 'built'), table('E2', 'sessions', 'planned'), screen()])
  await until(() => a.container.querySelectorAll('[data-element]').length === 2)
  expect(a.container.querySelectorAll('.react-flow__edge')).toHaveLength(1)
  const list = a.container.querySelector('nav[aria-label="Tables list"]')!
  expect([...list.querySelectorAll('li')].map(li => li.textContent)).toEqual(['BuiltE1 users', 'PlannedE2 sessions'])
})

it('a_drawn_node_is_exactly_as_tall_as_its_lines', async () => {
  const people = table('E1', 'people', 'built', { content: { kind: 'table', name: 'people', purpose: 'p', columns: [
    { name: 'id', type_name: 'INTEGER', primary_key: true }, { name: 'email', type_name: 'TEXT' },
    { name: 'password_hash', type_name: 'TEXT' }, { name: 'created_at', type_name: 'TEXT NOT NULL' },
  ] } })
  const a = app = await route('/projects/p1/state?view=tables', [people])
  await until(() => a.container.querySelectorAll('[data-element]').length === 1)
  const article = a.container.querySelector<HTMLElement>('[data-element="E1"]')!
  const wrapper = article.closest<HTMLElement>('.react-flow__node')!
  expect(article.querySelectorAll('li')).toHaveLength(4)
  // The frame (1px border and 8px padding, twice) plus every line at its drawn height.
  const px = (value: string) => Number.parseFloat(value || '0')
  const lines = [...article.children].filter(child => !child.classList.contains('react-flow__handle')) as HTMLElement[]
  const drawn = lines.reduce((sum, child) => sum + px(child.style.height) + px(child.style.marginTop), 0)
    + [...article.querySelectorAll<HTMLElement>('li')].reduce((sum, li) => sum + px(li.style.height), 0)
  expect(px(wrapper.style.height)).toBe(18 + drawn)
})

it('a_removed_table_is_hidden_until_shown', async () => {
  const a = app = await route('/projects/p1/state?view=tables', [table('E1', 'users', 'built'), table('E2', 'old_sessions', 'removed')])
  await until(() => a.container.querySelectorAll('[data-element]').length === 1)
  expect(a.container.querySelector('nav[aria-label="Tables list"]')!.textContent).not.toContain('old_sessions')
  act(() => a.button('Show removed')!.click())
  await until(() => a.container.querySelectorAll('[data-element]').length === 2)
  expect(a.container.querySelector('nav[aria-label="Tables list"]')!.textContent).toContain('old_sessions')
  expect(a.button('Hide removed')).toBeDefined()
})

it('an_opened_table_shows_its_part_outcomes_tasks_and_revisions', async () => {
  const sessions = table('E2', 'sessions', 'built', { outcomes: ['o1'] }, {
    why: 'matches the plan', confirmed: 'file_and_symbol', built_revision: 1, done: [task()],
  })
  const a = app = await route('/projects/p1/state?view=tables&element=E2', [table('E1', 'users', 'built'), sessions], {
    'GET /api/projects/p1/elements/E2/revisions': [sessions.element],
  })
  await until(() => a.container.querySelector('aside[aria-label="Element E2"] details') !== null)
  expect(count(a, 'GET /api/projects/p1/elements/E2/revisions')).toBe(1)
  const panel = a.container.querySelector('aside[aria-label="Element E2"]')!
  expect(panel.textContent).toContain('E2 · table · sessions')
  expect(panel.textContent).toContain('matches the plan')
  expect(panel.textContent).toContain('File and symbol confirmed')
  expect(panel.textContent).toContain('T3 · v2 · r1 · create · src/db/sessions.rs · Session (found)')
  expect(panel.querySelector('a[href="/projects/p1/workflows/w1/versions/2/tasks/3"]')).not.toBeNull()
  await until(() => panel.textContent!.includes('Web client'))
  expect(panel.querySelector('a[href*="view=roadmap"]')?.textContent).toBe('o1')
  expect(panel.querySelector('table[aria-label="sessions columns"]')?.textContent).toContain('→ users.id')
  expect(panel.textContent).toContain('r1 · 2026-11-01T00:00:00Z')
})

it('a_behind_table_shows_its_built_shape_and_the_planned_change', async () => {
  const v1 = table('E2', 'sessions', 'built').element
  const v2: DesignElement = { ...table('E2', 'sessions_v2', 'built').element, revision: 2 }
  const behind = { ...table('E2', 'sessions_v2', 'built', { revision: 2 }, { behind: true, built_revision: 1, done: [task()] }) }
  const a = app = await route('/projects/p1/state?view=tables&element=E2', [behind], {
    'GET /api/projects/p1/elements/E2/revisions': [v1, v2],
  })
  await until(() => a.container.querySelector('section[aria-label="Built revision"]') !== null)
  expect(a.container.querySelector('section[aria-label="Built revision"]')!.textContent).toContain('Built (r1)')
  expect(a.container.querySelector('section[aria-label="Planned change"]')!.textContent).toContain('Planned change (r2)')
  expect(a.container.querySelector('[data-element="E2"]')!.textContent).toContain('built r1, newer planned')
  expect(a.container.querySelectorAll('aside details')).toHaveLength(2)
})

it('an_element_without_revisions_shows_cores_error', async () => {
  const a = app = await route('/projects/p1/state?view=tables&element=E1', [table('E1', 'users', 'built')], {
    'GET /api/projects/p1/elements/E1/revisions': () => Response.json({ code: 'INVALID_COMMAND', message: 'not found: element' }, { status: 404 }),
  })
  await until(() => a.text().includes('not found: element'))
  expect(a.container.querySelector('aside[aria-label="Element E1"]')!.textContent).toContain('INVALID_COMMAND')
})

it('an_element_that_is_not_a_table_says_so', async () => {
  const a = app = await route('/projects/p1/state?view=tables&element=E3', [table('E1', 'users', 'built'), screen()])
  await until(() => a.text().includes('No table E3 in this project.'))
})

it('no_tables_says_the_planner_writes_them', async () => {
  const a = app = await route('/projects/p1/state?view=tables', [screen()])
  await until(() => a.text().includes('No tables yet. The Planner writes them when a plan needs them.'))
})

it('a_design_element_change_reads_the_elements_and_its_revisions_again', async () => {
  const sessions = table('E2', 'sessions', 'built', {}, { built_revision: 1 })
  const a = app = await route('/projects/p1/state?view=tables&element=E2', [sessions], {
    'GET /api/projects/p1/elements/E2/revisions': [sessions.element],
  })
  await until(() => a.container.querySelector('aside[aria-label="Element E2"] details') !== null && project(a).length > 0)
  act(() => project(a)[0].caughtUp(0))
  await act(async () => { await new Promise(resolve => setTimeout(resolve, 50)) })
  const elements = count(a, 'GET /api/projects/p1/elements')
  const revisions = count(a, 'GET /api/projects/p1/elements/E2/revisions')
  act(() => project(a)[0].durable(1, 'DesignElementChanged', null, { project_id: 'p1', element_id: 'E2', revision: 2 }))
  await until(() => count(a, 'GET /api/projects/p1/elements') === elements + 1 && count(a, 'GET /api/projects/p1/elements/E2/revisions') === revisions + 1)
  await act(async () => { await new Promise(resolve => setTimeout(resolve, 50)) })
  expect(count(a, 'GET /api/projects/p1/elements')).toBe(elements + 1)
  expect(count(a, 'GET /api/projects/p1/elements/E2/revisions')).toBe(revisions + 1)
})
