// @vitest-environment happy-dom
// Deleting the open conversation through the real app's confirmation flow (§16.8).

import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import type { PlanningThread } from '@/api/client'
import { planFixture, planListing, threadFixture } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, choose, startApp, until } from '../test-app'

let app: TestApp | null = null
afterEach(() => {
  app?.unmount()
  app = null
})

it('deletes a conversation after asking, and leaves its page', async () => {
  let threads: PlanningThread[] = [threadFixture]
  const urls: string[] = []
  const table = answers()
  table['GET /api/projects/p1/threads'] = () => Response.json(threads)
  table['DELETE /api/threads/t1'] = (request: Request) => {
    urls.push(request.url)
    threads = []
    return Response.json({ ...threadFixture, removed_at: '2026-10-02T18:00:00Z' })
  }
  const a = (app = await startApp('/projects/p1/threads/t1', table))
  await until(() => a.button('Conversation options: Conversation 1') !== undefined)
  await choose(a, 'Conversation options: Conversation 1', 'Delete')
  await until(() => a.text().includes('Delete "Conversation 1"?'))
  expect(a.text()).toContain('It leaves the list for good. Its plans stay in the project, and their history still names it.')
  expect(urls).toHaveLength(0)
  await act(async () => a.button('Cancel')?.click())
  await until(() => document.querySelector('[role="dialog"]') === null)
  expect(urls).toHaveLength(0)

  const reads = a.calls.filter((c) => c === 'GET /api/projects/p1/threads').length
  await choose(a, 'Conversation options: Conversation 1', 'Delete')
  await until(() => a.button('Delete') !== undefined)
  await act(async () => a.button('Delete')?.click())
  await until(() => a.path() === '/projects/p1/new')
  expect(urls).toHaveLength(1)
  expect(new URL(urls[0]!).searchParams.get('command_id')).toMatch(/^[0-9a-f-]{36}$/)
  await until(() => a.calls.filter((c) => c === 'GET /api/projects/p1/threads').length > reads)
  expect([...a.container.querySelectorAll('a')].some((link) => link.textContent?.trim() === 'Conversation 1')).toBe(false)
})

it('refreshes retained plan attribution when its writer is deleted', async () => {
  let current = planFixture()
  let threads: PlanningThread[] = [threadFixture]
  const table = answers({ plan: () => Response.json(current), plans: [planListing(current)] })
  table['GET /api/projects/p1/threads'] = () => Response.json(threads)
  table['DELETE /api/threads/t1'] = () => {
    threads = []
    current = { ...current, written_by: { ...current.written_by, thread_removed: true } as typeof current.written_by }
    return Response.json({ ...threadFixture, removed_at: '2026-10-03T00:00:00Z' })
  }
  const a = (app = await startApp('/projects/p1/workflows/w1', table))
  await until(() => a.text().includes('from Login'))
  await until(() => a.button('Conversation options: Conversation 1') !== undefined)
  await choose(a, 'Conversation options: Conversation 1', 'Delete')
  await until(() => a.button('Delete') !== undefined)
  await act(async () => a.button('Delete')?.click())
  await until(() => a.text().includes('from Login (deleted)'))
  expect(a.path()).toBe('/projects/p1/workflows/w1')
})
