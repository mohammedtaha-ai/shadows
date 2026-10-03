// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { answers } from '@/test/fake-daemon'
import { planFixture } from '@/test/contract-fixtures'
import { type TestApp, startApp, typeInto, until } from '../test-app'

let app: TestApp | null = null
afterEach(() => { app?.unmount(); app = null })
const outcome = { id: 'o1', revision: 1, ordinal: 0, parent: null,
  content: { title: 'تسجيل الدخول', intended_result: 'دخول آمن', acceptance: ['يمكن الدخول', 'يمكن الخروج'] } }
const listing = (id: string, title: string, plan: string) => ({ id, plan_id: plan, title, plan_state: 'Archived', state: 'Frozen', version: 1, updated_at: '2026-10-03T00:00:00Z' })

// Hiding unassigned/archived plans or deriving Done from lifecycle fails here.
it('roadmap_navigation_keeps_old_plans_accessible', async () => {
  const a = app = await startApp('/projects/p1/workspace?view=roadmap&outcome=o1', {
    ...answers({ plan: planFixture({ plan_id: 'linked', plan_state: 'Archived', state: 'Frozen', frozen_at: '2026-10-03T00:00:00Z' }) }),
    'GET /api/projects/p1/design/outcomes': { revision: 1, items: [outcome], next: null },
    'GET /api/projects/p1/design/outcomes/o1': { revision: 1, outcome, ancestors: [], parts: ['part1'], plans: ['linked'] },
    'GET /api/projects/p1/design/parts': { revision: 1, items: [{ id: 'part1', revision: 1, ordinal: 0, parent: null, content: { title: 'المصادقة', responsibility: '', design: '', kind: null } }], next: null },
    'GET /api/projects/p1/design/parts/part1': { revision: 1, part: { id: 'part1', revision: 1, ordinal: 0, parent: null, content: { title: 'المصادقة', responsibility: '', design: '', kind: null } }, ancestors: [], plans: [] },
    'GET /api/projects/p1/workflows': [listing('w1', 'خطة الدخول', 'linked'), listing('w2', 'خطة مستقلة', 'unassigned')],
  })
  await until(() => a.container.querySelector('input[aria-label="Outcome title"]') !== null)
  expect(a.text()).toContain('المصادقة')
  expect(a.text()).not.toMatch(/\bDone\b|\bCompleted\b/)
  const { router } = await import('@/router')
  await act(async () => { await router.navigate({ to: '/projects/$projectId/workspace', params: { projectId: 'p1' }, search: { view: 'plans' } }) })
  await until(() => a.text().includes('خطة مستقلة'))
  expect(a.text()).toContain('خطة الدخول')
  const old = a.container.querySelector<HTMLAnchorElement>('a[href="/projects/p1/workflows/w1"]')
  expect(old).not.toBeNull()
  await act(async () => { await router.navigate({ to: '/projects/$projectId/workflows/$workflowId', params: { projectId: 'p1', workflowId: 'w1' } }) })
  await until(() => a.calls.includes('GET /api/workflows/w1'))
  await until(() => a.text().includes('Archived · read only'))
  expect(a.button('Approve')).toBeUndefined()
})

// Stale saves and a reconnect may not erase locally edited acceptance/result.
it('roadmap editor keeps local content through a stale save until Reload', async () => {
  let saved = { revision: 1, outcome, ancestors: [], parts: [], plans: [] }
  const a = app = await startApp('/projects/p1/workspace?view=roadmap&outcome=o1', {
    ...answers(),
    'GET /api/projects/p1/design/outcomes': { revision: 1, items: [outcome], next: null },
    'GET /api/projects/p1/design/outcomes/o1': () => Response.json(saved),
    'GET /api/projects/p1/design/parts': { revision: 1, items: [], next: null },
    'POST /api/projects/p1/design/edits': () => {
      saved = { ...saved, revision: 2, outcome: { ...outcome, content: { ...outcome.content, title: 'عنوان أحدث' } } }
      return Response.json({ code: 'REVISION_CONFLICT', message: 'changed', current_revision: 2 }, { status: 409 })
    },
  })
  const title = () => a.container.querySelector<HTMLInputElement>('input[aria-label="Outcome title"]')
  await until(() => title()?.value === 'تسجيل الدخول')
  typeInto(title()!, 'نصي المحلي')
  act(() => a.button('Save outcome')?.click())
  await until(() => a.button('Reload') !== undefined)
  expect(title()?.value).toBe('نصي المحلي')
  act(() => a.button('Reload')?.click())
  await until(() => title()?.value === 'عنوان أحدث')
})
