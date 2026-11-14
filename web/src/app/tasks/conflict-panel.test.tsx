// @vitest-environment happy-dom

import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import type { Attempt, TaskConflict } from '@/api/runs'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, until } from '@/test/test-app'

const PAGE = '/projects/p1/workflows/w1/versions/1/tasks/1'
let app: TestApp | null = null
afterEach(() => { app?.unmount(); app = null })

const a1: Attempt = {
  id: 'a1', project_id: 'p1', workflow_id: 'w1', version: 1, task: 1, number: 1, thread_id: 't1', operation_id: null,
  model: 'fake-large', state: 'stopped', started_at: '2026-11-08T10:00:00Z', ended_at: null, commit: null,
  reason: null, report: null, addition: null,
}

const conflict = (extra: Partial<TaskConflict> = {}): TaskConflict => ({
  attempt_id: 'a1', project_id: 'p1', workflow_id: 'w1', version: 1, task: 1,
  expected: 'send(to: string)', observed: 'send(to: Address)',
  evidence: [{ path: 'src/mail.rs', line: 12, summary: 'takes Address' }, { path: 'src/x.rs', line: null, summary: 'no line' }],
  attempt_state: 'stopped', scope_held: false, delivery: 'delivered', created_at: '2026-11-14T10:00:00Z', ...extra,
})

const open = async () => {
  await until(() => document.querySelector('[role="tab"]') !== null)
  await act(async () => { [...document.querySelectorAll<HTMLElement>('[role="tab"]')].find((t) => t.textContent === 'Report')?.click() })
  await until(() => document.body.textContent.includes('Attempt 1'))
  const b = [...document.querySelectorAll<HTMLButtonElement>('button[aria-expanded]')].find((x) => x.textContent.includes('Attempt 1'))
  await act(async () => { b?.click() })
}

it('conflict_shows_expected_observed_and_attempt_state', async () => {
  app = await startApp(PAGE, {
    ...answers({ attempts: [a1] }),
    'GET /api/attempts/a1/brief': () => Response.json({ code: 'context_not_recorded', reason: 'old' }, { status: 409 }),
    'GET /api/attempts/a1/conflict': conflict({ scope_held: true, delivery: 'no_planner' }),
  })
  await open()
  await until(() => document.querySelector('[aria-label="Contract conflict"]') !== null)
  const text = document.querySelector('[aria-label="Contract conflict"]')!.textContent
  expect(text).toContain('send(to: string)')
  expect(text).toContain('send(to: Address)')
  expect(text).toContain('src/mail.rs:12')
  expect(text).toContain('takes Address')
  expect(text).toContain('src/x.rs')
  expect(text).toContain('stopped')
  expect(text).toContain('scope still held')
  expect(text).toContain('no Planner')
})

it('a_null_conflict_shows_nothing', async () => {
  app = await startApp(PAGE, {
    ...answers({ attempts: [a1] }),
    'GET /api/attempts/a1/brief': () => Response.json({ code: 'context_not_recorded', reason: 'old' }, { status: 409 }),
    'GET /api/attempts/a1/conflict': null,
  })
  await open()
  await until(() => app!.calls.includes('GET /api/attempts/a1/conflict'))
  expect(document.querySelector('[aria-label="Contract conflict"]')).toBeNull()
})
