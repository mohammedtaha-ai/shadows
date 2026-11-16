// @vitest-environment happy-dom
import { createMemoryHistory, createRootRoute, createRoute, createRouter, RouterProvider } from '@tanstack/react-router'
import { act } from 'react'
import { createRoot } from 'react-dom/client'
import { afterEach, expect, it } from 'vitest'
import type { CheckResult, RecheckNotice } from '@/api/runs'
import { NoticeItem } from './notice-item'

;(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true

afterEach(() => { document.body.innerHTML = '' })

const check = (extra: Partial<CheckResult> = {}): CheckResult => ({
  for_task: 3, id: 'unit', run: ['cargo', 'test'], reported: 'passed', outcome: 'failed',
  ...extra,
} as CheckResult)

const recheck: RecheckNotice = {
  plan_id: 'pl1', cause_workflow_id: 'w1', cause_version: 4, cause_task: 5, recheck_id: 'r1',
  failed: [
    { workflow_id: 'w1', version: 2, task: 3, title: 'Login screen', attempt_id: 'a3', checks: [check()] },
    { workflow_id: 'w1', version: 4, task: 1, title: 'Header', attempt_id: 'a1', checks: [check({ for_task: 1, id: 'lint' })] },
  ],
}

async function draw(): Promise<HTMLElement> {
  const root = createRootRoute({
    component: () => <NoticeItem body="Planner text" card={null} recheck={recheck} projectId="p1" sender="Shadows" />,
  })
  const task = createRoute({ getParentRoute: () => root, path: '/projects/$projectId/workflows/$workflowId/versions/$version/tasks/$task' })
  const router = createRouter({ routeTree: root.addChildren([task]), history: createMemoryHistory({ initialEntries: ['/'] }) })
  const host = document.createElement('div')
  document.body.append(host)
  await act(async () => createRoot(host).render(<RouterProvider router={router} />))
  return host
}

it('a_recheck_lists_each_failing_task_with_its_checks', async () => {
  const host = await draw()
  expect(host.textContent).toContain('recheck after T5 (version 4)')
  expect(host.textContent).toContain('These done tasks no longer pass:')
  expect(host.textContent).toContain('T3 · Login screen')
  expect(host.textContent).toContain('T3 · unit')
  expect(host.textContent).toContain('T1 · lint')
  expect(host.textContent).toContain('Their done marks stand.')
  expect(host.textContent).not.toContain('Planner text')
})

it('a_failing_task_links_to_its_own_version', async () => {
  const host = await draw()
  const links = [...host.querySelectorAll('a')].map(link => link.getAttribute('href'))
  expect(links).toContain('/projects/p1/workflows/w1/versions/2/tasks/3')
  expect(links).toContain('/projects/p1/workflows/w1/versions/4/tasks/1')
})
