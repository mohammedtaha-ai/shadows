// @vitest-environment happy-dom

import { createMemoryHistory, createRootRoute, createRoute, createRouter, RouterProvider } from '@tanstack/react-router'
import { act } from 'react'
import { createRoot } from 'react-dom/client'
import { afterEach, expect, it } from 'vitest'
import type { TaskConflict } from '@/api/runs'
import { ConflictNotice } from './conflict-notice'

;(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true
afterEach(() => { document.body.innerHTML = '' })

const conflict: TaskConflict = {
  attempt_id: 'a9', project_id: 'p1', workflow_id: 'w1', version: 2, task: 3, expected: 'one', observed: 'two',
  evidence: [{ path: 'a.rs', line: 4, summary: 'why' }, { path: 'b.rs', line: null, summary: 'no line' }],
  attempt_state: 'running', scope_held: true, delivery: 'pending', created_at: '2026-11-14T10:00:00Z',
}

async function draw(value: TaskConflict): Promise<HTMLElement> {
  const root = createRootRoute({ component: () => <ConflictNotice conflict={value} attemptId="a9" /> })
  const task = createRoute({ getParentRoute: () => root, path: '/projects/$projectId/workflows/$workflowId/versions/$version/tasks/$task' })
  const router = createRouter({ routeTree: root.addChildren([task]), history: createMemoryHistory({ initialEntries: ['/'] }) })
  const host = document.createElement('div')
  document.body.append(host)
  await act(async () => createRoot(host).render(<RouterProvider router={router} />))
  return host
}

it('conflict_notice_keeps_executor_attribution', async () => {
  const host = await draw(conflict)
  const text = host.textContent
  expect(text).toContain("Executor")
  expect(text).toContain('T3')
  expect(text).toContain('one')
  expect(text).toContain('two')
  expect(text).toContain('a.rs:4')
  expect(text).toContain('b.rs')
  expect(host.querySelector('button, input')).toBeNull()
})

it('conflict_notice_links_to_the_attempt_in_its_report', async () => {
  const host = await draw(conflict)
  const link = host.querySelector('a')!
  expect(link.getAttribute('href')).toBe('/projects/p1/workflows/w1/versions/2/tasks/3#attempt-a9')
})
