// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from 'vitest'
import { planFixture, planViewEntry, visionProposalEntry } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, until } from '../test-app'

let app: TestApp | null = null
afterEach(() => { app?.unmount(); app = null; vi.restoreAllMocks() })

const proposal = (agent: string) => ({ id: 'p1', thread_id: 't1', action: 'add' as const,
  field: 'goals' as const, item_id: 'G1', text: 'Ship', old_text: null,
  expected_item_revision: null, status: 'Approved' as const, outdated: false, created_at: 'today',
  approved_by: { principal: 'user:local', agent, request: agent === 'human' ? null :
    { thread_id: 't2', entry_id: 'request', ordinal: 7 } } })

it.each(['human', 'planner:claude-code'])('shows %s attribution with the correct request conversation', async agent => {
  const p = proposal(agent)
  const a = app = await startApp('/projects/p1/threads/t1', {
    ...answers({ entries: [visionProposalEntry('e1', p)] }),
    'GET /api/projects/p1/vision-proposals': [p],
  })
  const expected = agent === 'human' ? 'Approved by you' : 'Approved by the Planner at your request'
  await until(() => a.text().includes(expected))
  const link = a.container.querySelector('a[href="/projects/p1/threads/t2#entry-7"]')
  expect(link !== null).toBe(agent !== 'human')
})

it('lands on the requested entry after conversation history loads', async () => {
  const scroll = vi.spyOn(Element.prototype, 'scrollIntoView').mockImplementation(() => {})
  const p = proposal('human')
  const entry = { ...visionProposalEntry('e1', p), ordinal: 3 }
  const a = app = await startApp('/projects/p1/threads/t1#entry-3', {
    ...answers({ entries: [entry] }), 'GET /api/projects/p1/vision-proposals': [p],
  })
  await until(() => a.container.querySelector('#entry-3') !== null)
  await until(() => scroll.mock.calls.length > 0)
  expect(scroll.mock.instances.some(el => (el as HTMLElement).id === 'entry-3')).toBe(true)
})

it('shows approval attribution on a frozen plan card', async () => {
  const p = planFixture({ state:'Frozen', approved_by:proposal('planner:claude-code').approved_by })
  const a = app = await startApp('/projects/p1/threads/t1', {
    ...answers({ entries:[planViewEntry('e1','Plan v1',p.id)] }),
    [`GET /api/workflows/${p.id}`]:p,
  })
  await until(() => a.text().includes('Approved by the Planner at your request'))
  expect(a.container.querySelector('a[href="/projects/p1/threads/t2#entry-7"]')).not.toBeNull()
})
