// @vitest-environment happy-dom
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createMemoryHistory, createRootRoute, createRoute, createRouter, RouterProvider } from '@tanstack/react-router'
import { act } from 'react'
import { createRoot } from 'react-dom/client'
import { afterEach, expect, it, vi } from 'vitest'
import type { CheckResult, RecheckNotice } from '@/api/runs'
import { NoticeItem } from './notice-item'

;(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true

afterEach(() => {
  document.body.innerHTML = ''
  vi.unstubAllGlobals()
})

const check = (extra: Partial<CheckResult> = {}): CheckResult => ({
  for_task: 3, id: 'unit', by_shadows: false, run: ['cargo', 'test'], reported: 'passed', outcome: 'failed',
  ...extra,
} as CheckResult)

const recheck: RecheckNotice = {
  plan_id: 'pl1', cause_workflow_id: 'w1', cause_version: 4, cause_task: 5, recheck_id: 'r1',
  failed: [
    { workflow_id: 'w1', version: 2, task: 3, title: 'Login screen', attempt_id: 'a3',
      checks: [check({ reported_note: 'Executor note', matches: false })] },
    { workflow_id: 'w2', version: 1, task: 1, title: 'Header', attempt_id: 'a1', checks: [check({ for_task: 1, id: 'lint' })] },
  ],
}

/** Core's plan read answers each plan in `plans` by its title; any other plan's read never answers. */
async function draw(plans: Record<string, string>): Promise<HTMLElement> {
  vi.stubGlobal('fetch', async (request: Request) => {
    const id = new URL(request.url).pathname.replace('/api/workflows/', '')
    const title = plans[id]
    return title === undefined ? new Promise<Response>(() => {}) : Response.json({ id, title })
  })
  const queries = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  const root = createRootRoute({
    component: () => <QueryClientProvider client={queries}>
      <NoticeItem body="Planner text" card={null} recheck={recheck} projectId="p1" sender="Shadows" />
    </QueryClientProvider>,
  })
  const task = createRoute({ getParentRoute: () => root, path: '/projects/$projectId/workflows/$workflowId/versions/$version/tasks/$task' })
  const router = createRouter({ routeTree: root.addChildren([task]), history: createMemoryHistory({ initialEntries: ['/'] }) })
  const host = document.createElement('div')
  document.body.append(host)
  await act(async () => createRoot(host).render(<RouterProvider router={router} />))
  await act(async () => { await new Promise((resolve) => setTimeout(resolve, 0)) })
  return host
}

it('a_recheck_lists_each_failing_task_with_its_checks', async () => {
  const host = await draw({ w1: 'Notes service', w2: 'Web shell' })
  expect(host.textContent).toContain('recheck after T5 (version 4)')
  expect(host.textContent).toContain('These done tasks no longer pass:')
  expect(host.textContent).toContain('T3 · Login screen')
  expect(host.textContent).toContain('T3 · unit')
  expect(host.textContent).toContain('T1 · lint')
  expect(host.textContent).toContain('Confirmed: failed')
  expect(host.textContent).toContain('Their done marks stand.')
  expect(host.textContent).not.toContain('Planner text')
})

it('a_recheck_row_has_no_reported_line', async () => {
  const host = await draw({ w1: 'Notes service', w2: 'Web shell' })
  expect(host.textContent).not.toContain('Reported:')
  expect(host.textContent).not.toContain('Executor note')
  expect(host.textContent).not.toContain('Mismatch')
})

it('a_failing_task_names_its_plan_and_version', async () => {
  const host = await draw({ w1: 'Notes service', w2: 'Web shell' })
  expect(host.textContent).toContain('Notes service, version 2')
  expect(host.textContent).toContain('Web shell, version 1')
})

it('a_plan_not_yet_read_shows_its_version', async () => {
  const host = await draw({ w1: 'Notes service' })
  expect(host.textContent).toContain('Notes service, version 2')
  const items = [...host.querySelectorAll('section[aria-label="Recheck"] > div > ul > li')]
  expect(items[1]?.querySelector('p')?.textContent).toBe('version 1')
})

it('a_failing_task_links_to_its_own_version', async () => {
  const host = await draw({ w1: 'Notes service', w2: 'Web shell' })
  const links = [...host.querySelectorAll('a')].map(link => link.getAttribute('href'))
  expect(links).toContain('/projects/p1/workflows/w1/versions/2/tasks/3')
  expect(links).toContain('/projects/p1/workflows/w2/versions/1/tasks/1')
})
