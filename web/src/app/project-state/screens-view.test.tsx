// @vitest-environment happy-dom
import { afterEach, expect, it } from 'vitest'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, until } from '@/test/test-app'
import type { BuildState, DesignElement, ElementView } from '@/api/project-state'

let app: TestApp | null = null
afterEach(() => { app?.unmount(); app = null })

const screen = (id: string, name: string, state: BuildState, leads: [string, string][] = [], route: string | null = null): ElementView => {
  const element: DesignElement = {
    id, revision: 1, project_id: 'p1', part_id: 'web', outcomes: [], deleted: false, written_by: 't1', created_at: '2026-11-01T00:00:00Z',
    content: { kind: 'screen', name, purpose: `${name} purpose`, route, leads_to: leads.map(([target, when]) => ({ screen: target, when })) },
  }
  return { element, state: { element_id: id, state, behind: false, built_revision: null, done: [], planned: [] } }
}
const table = (): ElementView => ({
  element: { id: 'E9', revision: 1, project_id: 'p1', part_id: 'web', outcomes: [], deleted: false, written_by: 't1', created_at: '2026-11-01T00:00:00Z',
    content: { kind: 'table', name: 'users', purpose: 'p', columns: [] } },
  state: { element_id: 'E9', state: 'planned', behind: false, built_revision: null, done: [], planned: [] },
})
const route = (url: string, elements: ElementView[], extra: Record<string, unknown> = {}) =>
  startApp(url, {
    ...answers(), 'GET /api/projects/p1/elements': elements,
    'GET /api/projects/p1/state': { project_id: 'p1', stage: 'plans', map_revision: 1, missing: [], parts: [], parts_more: 0, unassigned: [], unassigned_more: 0, gaps: [], gaps_more: 0 },
    ...extra,
  })

it('the_screen_flow_draws_screens_and_arrows_labelled_when', async () => {
  const a = app = await route('/projects/p1/state?view=screens', [
    screen('E1', 'Login', 'planned', [['E2', 'signed in']], '/login'), screen('E2', 'Home', 'built'), table(),
  ])
  await until(() => a.container.querySelectorAll('[data-element]').length === 2)
  expect(a.container.querySelector('section[aria-label="Screen flow"]')).not.toBeNull()
  expect(a.container.querySelectorAll('.react-flow__edge')).toHaveLength(1)
  await until(() => a.container.querySelector('[data-link="link:E1:0"]')?.textContent === 'signed in')
  expect(a.container.querySelector('[data-link="link:E1:0"]')!.getAttribute('dir')).toBe('auto')
  expect(a.container.querySelector('nav[aria-label="Screens list"]')!.textContent).toContain('Login')
  expect(a.container.querySelector('[data-element="E1"]')!.textContent).toContain('/login')
})

it('links_both_ways_draw_two_arrows_on_two_paths', async () => {
  const a = app = await route('/projects/p1/state?view=screens', [
    screen('E2', 'Sign in', 'planned', [['E3', 'chooses to create an account']], '/sign-in'),
    screen('E3', 'Sign up', 'planned', [['E2', 'chooses to sign in']], '/sign-up'),
  ])
  await until(() => a.container.querySelectorAll('[data-link]').length === 2)
  const paths = [...a.container.querySelectorAll('.react-flow__edge path.react-flow__edge-path')]
  expect(paths).toHaveLength(2)
  expect(paths.every(path => path.getAttribute('marker-end')?.includes('arrowclosed') === true)).toBe(true)
  expect(new Set(paths.map(path => path.getAttribute('d'))).size).toBe(2)
  expect([...a.container.querySelectorAll('[data-link]')].map(label => label.textContent))
    .toEqual(['chooses to create an account', 'chooses to sign in'])
})

it('a_link_to_a_screen_not_shown_is_not_drawn', async () => {
  const a = app = await route('/projects/p1/state?view=screens', [
    screen('E1', 'Login', 'planned', [['E2', 'signed in']]), screen('E2', 'Old home', 'removed'),
  ])
  await until(() => a.container.querySelectorAll('[data-element]').length === 1)
  expect(a.container.querySelectorAll('.react-flow__edge')).toHaveLength(0)
  expect(a.container.querySelector('[data-element="E1"]')!.textContent).toContain('→ E2 when signed in')
})

it('no_screens_says_the_planner_writes_them', async () => {
  const a = app = await route('/projects/p1/state?view=screens', [table()])
  await until(() => a.text().includes('No screens yet. The Planner writes them when a plan needs them.'))
})

it('an_opened_screen_shows_its_route_and_links_in_the_panel', async () => {
  const login = screen('E1', 'Login', 'planned', [['E2', 'signed in']], '/login')
  const a = app = await route('/projects/p1/state?view=screens&element=E1', [login, screen('E2', 'Home', 'built')], {
    'GET /api/projects/p1/elements/E1/revisions': [login.element],
  })
  await until(() => a.container.querySelector('aside[aria-label="Element E1"] details') !== null)
  const panel = a.container.querySelector('aside[aria-label="Element E1"]')!
  expect(panel.textContent).toContain('E1 · screen · Login')
  expect(panel.textContent).toContain('Route: /login')
  expect(panel.textContent).toContain('Home when signed in')
})
