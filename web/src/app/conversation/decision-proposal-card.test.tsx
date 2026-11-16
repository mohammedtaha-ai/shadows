// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it, vi } from 'vitest'
import type { DecisionRecord } from '@/api/design'
import type { ThreadEntry } from '@/api/threads'
import { entryOfKind } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, until } from '@/test/test-app'

let app: TestApp | null = null
afterEach(() => { app?.unmount(); app = null; vi.restoreAllMocks() })

const record = (extra: Partial<DecisionRecord> = {}): DecisionRecord => ({
  id: 'D1', version: 1, project_id: 'p1', title: 'Use SQLite', decision: 'Store everything in SQLite.',
  rationale: 'One file, no server.', source: 'planner', state: 'proposed', approved_by: null,
  created_at: '2026-11-16T00:00:00Z', ...extra,
})
const entryOf = (card: DecisionRecord): ThreadEntry => ({
  ...entryOfKind('e1', 'AgentMessage', 'Proposed decision D1'), kind: 'DecisionProposal',
  card: { ...card } as unknown as ThreadEntry['card'],
})
const start = (card: DecisionRecord, list: unknown, extra: Record<string, unknown> = {}) =>
  startApp('/projects/p1/threads/t1', {
    ...answers({ entries: [entryOf(card)] }), 'GET /api/projects/p1/decisions': list, ...extra,
  } as never)

it('a_proposed_decision_can_be_accepted', async () => {
  let live = record()
  const a = app = await start(record(), () => Response.json([live]), {
    'POST /api/projects/p1/decisions/D1/accept': () => {
      live = record({ state: 'accepted', approved_by: { agent: 'human' } as never })
      return Response.json(live)
    },
  })
  await until(() => a.button('Accept') !== undefined)
  expect(a.text()).toContain('Decision D1 · version 1')
  expect(a.text()).toContain('Use SQLite')
  expect(a.text()).toContain('One file, no server.')
  act(() => a.button('Accept')?.click())
  await until(() => a.text().includes('Approved by you'))
  expect(a.button('Accept')).toBeUndefined()
  expect(a.bodies.at(-1)).toMatchObject({ command_id: expect.any(String), version: 1 })
})

it('an_accepted_decision_shows_who_accepted_it', async () => {
  const a = app = await start(record(), [record({ state: 'accepted', approved_by: { agent: 'human' } as never })])
  await until(() => a.text().includes('Approved by you'))
  expect(a.button('Accept')).toBeUndefined()
})

it('a_superseded_decision_has_no_accept', async () => {
  const a = app = await start(record(), [record({ state: 'superseded' }), record({ version: 2 })])
  await until(() => a.text().includes('Superseded'))
  expect(a.button('Accept')).toBeUndefined()
})

it('a_stale_accept_asks_to_reload', async () => {
  const a = app = await start(record(), [record()], {
    'POST /api/projects/p1/decisions/D1/accept': () =>
      Response.json({ code: 'DECISION_STALE', message: 'stale' }, { status: 409 }),
  })
  await until(() => a.button('Accept') !== undefined)
  act(() => a.button('Accept')?.click())
  await until(() => a.text().includes('This decision changed. Reload to see its latest version.'))
})
