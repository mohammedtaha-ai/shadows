// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { answers } from '@/test/fake-daemon'
import { planFixture } from '@/test/contract-fixtures'
import type { DesignEdit } from '@/api/design'
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

it('outcome save retries the committed command when its follow-up read fails', async () => {
  let saved = { revision: 1, outcome, ancestors: [], parts: [], plans: [] }
  let failReads = false
  let committed: DesignEdit | undefined
  const a = app = await startApp('/projects/p1/workspace?view=roadmap&outcome=o1', {
    ...answers(),
    'GET /api/projects/p1/design/outcomes': { revision: 1, items: [outcome], next: null },
    'GET /api/projects/p1/design/outcomes/o1': () => failReads
      ? Response.json({ code: 'STORAGE_UNAVAILABLE', message: 'read unavailable' }, { status: 503 })
      : Response.json(saved),
    'GET /api/projects/p1/design/parts': { revision: 1, items: [], next: null },
    'POST /api/projects/p1/design/edits': async (request: Request) => {
      const body = await request.json() as DesignEdit
      if (committed) return JSON.stringify(body) === JSON.stringify(committed)
        ? Response.json({ revision: 2 })
        : Response.json({ code: 'REVISION_CONFLICT', message: 'already committed', current_revision: 2 }, { status: 409 })
      committed = body
      const put = body.ops.find(op => op.kind === 'OutcomePut')
      if (put?.kind !== 'OutcomePut') throw new Error('expected an outcome edit')
      saved = { ...saved, revision: 2, outcome: { ...outcome, revision: 2, content: put.content } }
      failReads = true
      return Response.json({ revision: 2 })
    },
  })
  const title = () => a.container.querySelector<HTMLInputElement>('input[aria-label="Outcome title"]')
  await until(() => title()?.value === outcome.content.title)
  typeInto(title()!, 'تعديل محفوظ')
  act(() => a.button('Save outcome')?.click())
  await until(() => a.text().includes('read unavailable') && a.button('Save outcome')?.disabled === false)
  expect(title()?.value).toBe('تعديل محفوظ')
  failReads = false
  act(() => a.button('Save outcome')?.click())
  await until(() => a.bodies.length === 2)
  expect(a.bodies[1]).toEqual(a.bodies[0])
  await until(() => a.button('Save outcome')?.disabled === true && !a.text().includes('read unavailable'))
  expect(title()?.value).toBe('تعديل محفوظ')
  expect(a.text()).not.toContain('already committed')
})

it('an outcome create retry preserves its identity and basis through a refetch', async () => {
  let revision = 0
  let writes = 0
  const a = app = await startApp('/projects/p1/workspace?view=roadmap', {
    ...answers(),
    'GET /api/projects/p1/design/outcomes': () => Response.json({ revision, items: [], next: null }),
    'GET /api/projects/p1/design/parts': { revision: 0, items: [], next: null },
    'POST /api/projects/p1/design/edits': () => {
      writes++; revision = 1
      return writes === 1
        ? Response.json({ code: 'STORAGE_UNAVAILABLE', message: 'response lost' }, { status: 503 })
        : Response.json({ revision: 1 })
    },
  })
  act(() => a.button('Create outcome')?.click())
  const title = () => a.container.querySelector<HTMLInputElement>('input[aria-label="Outcome title"]')
  await until(() => title() !== null)
  typeInto(title()!, 'نتيجة جديدة')
  act(() => a.button('Create')?.click())
  await until(() => a.text().includes('response lost'))
  await act(async () => { await a.queryClient.invalidateQueries({ queryKey: ['projects', 'p1', 'design'] }) })
  expect(title()?.value).toBe('نتيجة جديدة')
  act(() => a.button('Create')?.click())
  await until(() => a.bodies.length === 2)
  const edits = a.bodies as DesignEdit[]
  expect(edits[0].expected_revision).toBe(0)
  expect(edits[0].ops[0].kind).toBe('OutcomeCreate')
  expect(edits[1]).toEqual(edits[0])
  await until(() => title() === null)
})

it('outcome selection and project changes isolate local editor state', async () => {
  const other = { ...outcome, id: 'o2', content: { ...outcome.content, title: 'نتيجة ثانية' } }
  const foreign = { ...outcome, content: { ...outcome.content, title: 'مشروع آخر' } }
  const a = app = await startApp('/projects/p1/workspace?view=roadmap&outcome=o1', {
    ...answers(),
    'GET /api/projects/p1/design/outcomes': { revision: 1, items: [outcome, other], next: null },
    'GET /api/projects/p1/design/outcomes/o1': { revision: 1, outcome, ancestors: [], parts: [], plans: [] },
    'GET /api/projects/p1/design/outcomes/o2': { revision: 1, outcome: other, ancestors: [], parts: [], plans: [] },
    'GET /api/projects/p1/design/parts': { revision: 1, items: [], next: null },
    'GET /api/projects/p2/design/outcomes': { revision: 1, items: [foreign], next: null },
    'GET /api/projects/p2/design/outcomes/o1': { revision: 1, outcome: foreign, ancestors: [], parts: [], plans: [] },
    'GET /api/projects/p2/design/parts': { revision: 1, items: [], next: null },
    'GET /api/projects/p2/workflows': [],
  })
  const title = () => a.container.querySelector<HTMLInputElement>('input[aria-label="Outcome title"]')
  await until(() => title()?.value === outcome.content.title)
  typeInto(title()!, 'تعديل محلي أول')
  const { router } = await import('@/router')
  await act(async () => { await router.navigate({ to: '/projects/$projectId/workspace', params: { projectId: 'p1' }, search: { view: 'roadmap', outcome: 'o2' } }) })
  await until(() => title()?.value === other.content.title)
  typeInto(title()!, 'تعديل محلي ثان')
  await act(async () => { await router.navigate({ to: '/projects/$projectId/workspace', params: { projectId: 'p2' }, search: { view: 'roadmap', outcome: 'o1' } }) })
  await until(() => title()?.value === foreign.content.title)
  expect(a.text()).not.toContain('تعديل محلي أول')
  expect(a.text()).not.toContain('تعديل محلي ثان')
})
