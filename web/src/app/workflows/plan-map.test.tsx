// @vitest-environment happy-dom
// The map route renders current plans with navigable foreign references.

import { afterEach, expect, it, vi } from 'vitest'
import { act } from 'react'
import type { PlanMap } from '@/api/workflows'
import { answers } from '@/test/fake-daemon'
import { planFixture, planListing, planTask, planVersionsFixture, projectFixture, taskProgress } from '@/test/contract-fixtures'
import { type TestApp, startApp, until } from '@/test/test-app'

let app: TestApp | undefined
afterEach(() => { app?.unmount(); app = undefined; vi.unstubAllGlobals() })

it('opens the plan map from the sidebar with foreign plan navigation', async () => {
  const plan = { plan_id: 'plan1', project_id: 'p1', project_name: 'Demo', plan_state: 'Active' as const,
    workflow_id: 'w1', title: 'Web', goal: 'Login screen', version: 1, state: 'Draft' as const, task_count: 2, removed: false }
  const map: PlanMap = { project_id: 'p1', plans: [plan, { ...plan, plan_id: 'backend', project_id: 'p2',
    project_name: 'API project', workflow_id: 'backend-v2', title: 'Backend', version: 2 }],
    links: [{ plan_id: 'plan1', after: 'backend', count: 2, broken: true }] }
  app = await startApp('/projects/p1/map', { ...answers(), 'GET /api/projects/p1/plan-map': map })
  await until(() => app!.container.querySelector('[data-plan-map]') !== null)
  const foreign = app.container.querySelector<HTMLAnchorElement>('a[aria-label="Open plan Backend"]')
  expect(foreign?.getAttribute('href')).toBe('/projects/p2/workflows/backend-v2')
  expect(foreign?.textContent).toContain('API project')
  expect(app.text()).toContain('2 task links · broken')
  expect(app.container.querySelector('a[href="/projects/p1/map"]')?.textContent).toContain('Plan map')
})

it('refreshes the rendered map after dependency and reach notifications', async () => {
  let count = 1
  const node = { plan_id: 'plan1', project_id: 'p1', project_name: 'Demo', plan_state: 'Active' as const,
    workflow_id: 'w1', title: 'Web', goal: '', version: 1, state: 'Draft' as const, task_count: 1, removed: false }
  app = await startApp('/projects/p1/map', { ...answers(), 'GET /api/projects/p1/plan-map': () =>
    Response.json({ project_id: 'p1', plans: [node, { ...node, plan_id: 'backend', workflow_id: 'w2', title: 'Backend' }],
      links: [{ plan_id: 'plan1', after: 'backend', count, broken: count === 3 }] }) })
  await until(() => app!.text().includes('1 task link'))
  const stream = app.sources.find(source => source.url.includes('/projects/p1/events'))!
  act(() => stream.caughtUp(0))
  count = 2
  act(() => stream.durable(10, 'PlanDependenciesChanged', null, { plan_id: 'backend' }))
  await until(() => app!.text().includes('2 task links'))
  count = 3
  act(() => stream.durable(11, 'ProjectUnlinked', null, { linked: 'other' }))
  await until(() => app!.text().includes('3 task links · broken'))
})

it('opens a linked task in its own plan without selecting a local task with the same number', async () => {
  const link = { task: 4, after: { plan_id: 'backend', task: 3 }, kind: 'needs' as const, label: 'API', waiting_items: [] }
  const target = planFixture({ id: 'w2', plan_id: 'backend', title: 'Backend', tasks: [planTask(3, 'Login API')] })
  const source = planFixture({ tasks: [planTask(4, 'Login screen'), planTask(3, 'Local T3')], links: [link], linked_tasks: [{
    link, incoming: false, plan_id: 'backend', project_id: 'p1', project_name: 'Demo', workflow_id: 'w2',
    version: 1, plan_title: 'Backend', plan_state: 'Active', state: 'Draft', broken: null,
    task: { number: 3, title: 'Login API', goal: 'Authenticate', acceptance: [], state: 'Pending' },
  }] })
  app = await startApp('/projects/p1/workflows/w1', { ...answers({ plan: source }),
    'GET /api/workflows/w2': target, 'GET /api/plans/backend': planVersionsFixture(target) })
  await until(() => app!.container.querySelector('a[aria-label="Open Backend · T3"]') !== null)
  const linked = app.container.querySelector<HTMLAnchorElement>('a[aria-label="Open Backend · T3"]')!
  await act(async () => linked.click())
  await until(() => app!.path() === '/projects/p1/workflows/w2' &&
    app!.container.querySelector('.react-flow__node[data-id="t3"]')?.textContent?.includes('Login API') === true)
  expect(app.text()).not.toContain('Local T3')
})

it('shows a linked task card the status Runs holds for its exact version, not its plan preview', async () => {
  const preview = (number: number) => ({ number, title: `Store T${number}`, goal: 'Save days', acceptance: [], state: 'Pending' })
  const card = (task: number, extra: object) => {
    const link = { task: 1, after: { plan_id: 'store', task }, kind: 'needs' as const, label: 'store', waiting_items: [] }
    return { link, incoming: false, plan_id: 'store', project_id: 'p1', project_name: 'Demo', workflow_id: 'store-v2',
      version: 2, plan_title: 'Storage', plan_state: 'Active' as const, state: 'Frozen' as const, broken: null,
      task: preview(task), ...extra }
  }
  const linked = [card(3, {}), card(4, {}), card(5, {}), card(6, {}),
    card(7, { plan_id: 'draft-plan', workflow_id: 'draft-v1', version: 1, plan_title: 'Sketch', state: 'Draft' }),
    card(8, { workflow_id: null, version: null, state: null, task: null, broken: 'the plan was removed' })]
  const source = planFixture({ tasks: [planTask(1, 'Report')], links: linked.map(l => l.link), linked_tasks: linked })
  let progressReads = 0
  app = await startApp('/projects/p1/workflows/w1', { ...answers({ plan: source }),
    'GET /api/workflows/store-v2/versions/2/progress': () => {
      progressReads += 1
      return Response.json([taskProgress(3, { done: true, state: 'committed' }),
        taskProgress(4, { state: 'committed' }), taskProgress(5), taskProgress(6, { state: 'running' })])
    } })
  const status = (label: string) => app!.container.querySelector(`a[aria-label="${label}"] [data-task-status]`)?.textContent
  await until(() => status('Open Storage · T3') === 'Done · v2')
  expect(status('Open Storage · T4')).toBe('Committed · v2')
  expect(status('Open Storage · T5')).toBe('Not started · v2')
  expect(status('Open Storage · T6')).toBe('Running · v2')
  expect(status('Open Sketch · T7')).toBe('Draft · v1')
  expect(app.text()).not.toContain('Pending')
  expect(app.text()).toContain('the plan was removed')
  expect(progressReads).toBe(1)
})

it('says a linked task status is unavailable when its progress read fails', async () => {
  const link = { task: 1, after: { plan_id: 'store', task: 3 }, kind: 'needs' as const, label: 'store', waiting_items: [] }
  const source = planFixture({ tasks: [planTask(1, 'Report')], links: [link], linked_tasks: [{
    link, incoming: false, plan_id: 'store', project_id: 'p1', project_name: 'Demo', workflow_id: 'store-v2',
    version: 2, plan_title: 'Storage', plan_state: 'Active', state: 'Frozen', broken: null,
    task: { number: 3, title: 'Store', goal: 'Save days', acceptance: [], state: 'Pending' },
  }] })
  app = await startApp('/projects/p1/workflows/w1', { ...answers({ plan: source }),
    'GET /api/workflows/store-v2/versions/2/progress': () => new Response('down', { status: 500 }) })
  await until(() => app!.container.querySelector('a[aria-label="Open Storage · T3"] [data-task-status]')
    ?.textContent === 'Status unavailable · v2')
})

it('keeps another open projects sidebar current without duplicating the active project stream', async () => {
  let version = 1
  const other = { ...projectFixture, id: 'p2', slug: 'api', name: 'API project' }
  app = await startApp('/projects/p1/map', { ...answers(),
    'GET /api/projects': [projectFixture, other],
    'GET /api/projects/p1/plan-map': { project_id: 'p1', plans: [], links: [] },
    'GET /api/projects/p2/threads': [],
    'GET /api/projects/p2/workflows': () => Response.json([planListing(planFixture({
      id: `w${version}`, plan_id: 'backend', title: 'Backend', version,
    }))]),
  })
  await until(() => app!.container.querySelector('button[aria-controls="project-p2-section"]') !== null)
  await act(async () => app!.container.querySelector<HTMLButtonElement>('button[aria-controls="project-p2-section"]')!.click())
  await until(() => app!.text().includes('Backend'))
  const sources = app.sources.filter(source => !source.closed)
  expect(sources.filter(source => source.url.includes('/projects/p1/events'))).toHaveLength(1)
  const foreign = sources.find(source => source.url.includes('/projects/p2/events'))
  expect(foreign).toBeDefined()
  act(() => foreign!.caughtUp(0))
  version = 2
  act(() => foreign!.durable(20, 'WorkflowDraftStarted', null, { plan_id: 'backend', workflow_id: 'w2' }))
  await until(() => app!.container.querySelector('a[href="/projects/p2/workflows/w2"]') !== null)
  expect(app.text()).toContain('Draft v2')
})
