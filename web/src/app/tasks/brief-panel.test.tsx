// @vitest-environment happy-dom

import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import type { Attempt, AttemptBrief } from '@/api/runs'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, until } from '@/test/test-app'

const PAGE = '/projects/p1/workflows/w1/versions/1/tasks/1'
let app: TestApp | null = null
afterEach(() => { app?.unmount(); app = null })

const attempt = (id: string, number: number, extra: Partial<Attempt> = {}): Attempt => ({
  id, project_id: 'p1', workflow_id: 'w1', version: 1, task: 1, number, thread_id: 't1', operation_id: null,
  model: 'fake-large', state: 'committed', started_at: '2026-11-08T10:00:00Z', ended_at: null, commit: null,
  reason: null, report: null, addition: null, ...extra,
})

const brief = (a: Attempt, extra: Partial<AttemptBrief> = {}): AttemptBrief => ({
  attempt: a,
  captured_at: '2026-11-08T10:00:01Z',
  consumers: [{ task: 3, title: 'Mail sender', output: 'send', use_index: 0, purpose: 'Calls send' }],
  consumers_more: true,
  context: {
    map_revision: 4,
    parts: [{ part: { name: 'api' }, paths: [{ path: 'src/api.rs', via: { kind: 'direct' } }] }],
    plan: { plan_id: 'plan1', title: 'Original plan', version: 1, workflow_id: 'w1' },
    standards: { base: 7, additions: 2 },
    task: { acceptance: [], goal: 'Original goal', number: 1, reads: [], title: 'Login', writes: ['src/api.rs'] },
    unowned_paths: ['docs/x.md'],
  } as unknown as AttemptBrief['context'],
  ...extra,
})

const click = (element: HTMLElement | null | undefined) => act(async () => { element?.click() })
const heading = (n: number) => [...document.querySelectorAll<HTMLButtonElement>('button[aria-expanded]')]
  .find((b) => b.textContent.includes(`Attempt ${n}`))

const reportTab = async () => {
  await until(() => [...document.querySelectorAll<HTMLElement>('[role="tab"]')].find((t) => t.textContent === 'Report') !== undefined)
  await act(async () => { [...document.querySelectorAll<HTMLElement>('[role="tab"]')].find((t) => t.textContent === 'Report')?.click() })
}

it('brief_view_keeps_original_version', async () => {
  const a1 = attempt('a1', 1, { addition: 'Try again carefully' })
  app = await startApp(PAGE, {
    ...answers({ attempts: [a1] }),
    'GET /api/attempts/a1/brief': brief(a1),
  })
  await reportTab()
  await until(() => heading(1) !== undefined)
  await click(heading(1))
  await until(() => document.querySelector('[aria-label="Attempt brief"]') !== null)
  const text = document.querySelector('[aria-label="Attempt brief"]')!.textContent
  expect(text).toContain('Original plan')
  expect(text).toContain('version 1')
  expect(text).toContain('Original goal')
  expect(text).toContain('map revision 4')
  expect(text).toContain('standards 7')
  expect(text).toContain('fake-large')
  expect(text).toContain('Try again carefully')
  expect(text).toContain('Mail sender')
  expect(text).toContain('Planned consumers of the original version')
  expect(text).toContain('more')
  expect(text).not.toContain('every consumer')
})

it('historical_context_is_labelled', async () => {
  const a1 = attempt('a1', 1)
  app = await startApp(PAGE, {
    ...answers({ attempts: [a1] }),
    'GET /api/attempts/a1/brief': () => Response.json({ code: 'context_not_recorded', reason: 'old' }, { status: 409 }),
  })
  await reportTab()
  await until(() => heading(1) !== undefined)
  await click(heading(1))
  await until(() => document.body.textContent.includes('began before its context was saved'))
  expect(document.querySelector('[aria-label="Attempt brief"]')).toBeNull()
})

it('report_opens_the_selected_attempt_brief', async () => {
  const a2 = attempt('a2', 2)
  const a1 = attempt('a1', 1)
  app = await startApp(PAGE, {
    ...answers({ attempts: [a2, a1] }),
    'GET /api/attempts/a1/brief': brief(a1, { consumers: [], consumers_more: false }),
    'GET /api/attempts/a2/brief': brief(a2, { consumers: [], consumers_more: false }),
  })
  await reportTab()
  await until(() => heading(1) !== undefined)
  await click(heading(1))
  await until(() => app!.calls.includes('GET /api/attempts/a1/brief'))
  expect(app.calls).not.toContain('GET /api/attempts/a2/brief')
})

it('the_attempt_hash_opens_report_and_that_attempt', async () => {
  const a2 = attempt('a2', 2)
  const a1 = attempt('a1', 1)
  app = await startApp(`${PAGE}#attempt-a1`, {
    ...answers({ attempts: [a2, a1] }),
    'GET /api/attempts/a1/brief': brief(a1, { consumers: [], consumers_more: false }),
  })
  await until(() => document.querySelector('[aria-label="Attempt brief"]') !== null)
  expect(heading(1)?.getAttribute('aria-expanded')).toBe('true')
  expect(heading(2)?.getAttribute('aria-expanded')).toBe('false')
})
