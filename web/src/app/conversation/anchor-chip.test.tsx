// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import type { Outcome, Part } from '@/api/design'
import type { ThreadPlanningAnchor } from '@/api/planning-context'
import { threadFixture } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { startApp, until, type TestApp } from '@/test/test-app'

let open: TestApp | undefined
afterEach(() => { open?.unmount(); open = undefined })

const part = (id: string, title: string): Part =>
  ({ id, ordinal: 0, revision: 1, content: { title, responsibility: '', design: '' } })
const partView = { part: part('auth', 'Authentication'), ancestors: [], plans: [], revision: 1 }
const outcome: Outcome = {
  id: 'launch', ordinal: 0, revision: 1,
  content: { title: 'Launch', intended_result: '', acceptance: [] },
}

async function show(routes = answers()) {
  return open = await startApp('/projects/p1/threads/t1', routes)
}

async function click(app: TestApp, label: string) {
  await until(() => app.button(label)?.disabled === false)
  await act(async () => app.button(label)?.click())
}

it('shows_the_anchor_name', async () => {
  const routes = answers()
  routes['GET /api/threads/t1/anchor'] = { anchor: { kind: 'part', id: 'auth' } }
  routes['GET /api/projects/p1/design/parts/auth'] = partView
  const app = await show(routes)
  await until(() => app.text().includes('Anchored at Authentication'))
})

it('shows_a_removed_anchor', async () => {
  const routes = answers()
  routes['GET /api/threads/t1/anchor'] = { anchor: { kind: 'part', id: 'auth' } }
  routes['GET /api/projects/p1/design/parts/auth'] = () => new Response(null, { status: 404 })
  const app = await show(routes)
  await until(() => app.text().includes('Anchored part was removed'))
  expect(app.button('Clear')).toBeDefined()
  expect(app.button('Send context again')).toBeDefined()
})

it('clear_sends_null_with_a_command_id', async () => {
  const routes = answers()
  routes['GET /api/threads/t1/anchor'] = { anchor: { kind: 'part', id: 'auth' } }
  routes['GET /api/projects/p1/design/parts/auth'] = partView
  routes['PUT /api/threads/t1/anchor'] = { anchor: null }
  const app = await show(routes)
  await click(app, 'Clear')
  await until(() => app.text().includes('Not anchored'))
  expect(app.bodies).toContainEqual({ anchor: null, command_id: expect.any(String) })
})

it('picking_an_outcome_sets_it', async () => {
  const routes = answers()
  routes['GET /api/projects/p1/design/outcomes'] = { items: [outcome], revision: 1 }
  routes['GET /api/projects/p1/design/outcomes/launch'] = { outcome, parts: [], plans: [], ancestors: [], revision: 1 }
  routes['PUT /api/threads/t1/anchor'] = { anchor: { kind: 'outcome', id: 'launch' } }
  const app = await show(routes)
  await click(app, 'Anchor…')
  await click(app, 'Outcomes')
  await click(app, 'Select Launch')
  await until(() => app.text().includes('Anchored at Launch'))
  expect(app.bodies).toContainEqual({ anchor: { kind: 'outcome', id: 'launch' }, command_id: expect.any(String) })
})

it('moving_to_another_parent_starts_at_its_first_page', async () => {
  const routes = answers()
  routes['GET /api/projects/p1/design/parts'] = (request: Request) => {
    const { searchParams } = new URL(request.url)
    if (searchParams.get('parent') === 'a') {
      return Response.json(searchParams.has('after')
        ? { items: [part('a2', 'A second')], revision: 1 }
        : { items: [part('a1', 'A first')], next: 'a1', revision: 1 })
    }
    if (searchParams.get('parent') === 'b') return Response.json({ items: [part('b1', 'B first')], revision: 1 })
    return Response.json({ items: [part('a', 'A'), part('b', 'B')], revision: 1 })
  }
  const app = await show(routes)
  await click(app, 'Anchor…')
  await click(app, 'Browse A')
  await click(app, 'Load more')
  await until(() => app.button('Select A second') !== undefined)
  await click(app, 'Back')
  await click(app, 'Browse B')
  await until(() => app.button('Select B first') !== undefined)
  expect(app.button('Select A second')).toBeUndefined()
})

it('changing_the_selection_waits_for_an_unknown_previous_write', async () => {
  const routes = answers()
  const second = { ...outcome, id: 'verify', content: { ...outcome.content, title: 'Verify' } }
  routes['GET /api/projects/p1/design/outcomes'] = { items: [outcome, second], revision: 1 }
  routes['GET /api/projects/p1/design/outcomes/launch'] = { outcome, parts: [], plans: [], ancestors: [], revision: 1 }
  routes['GET /api/projects/p1/design/outcomes/verify'] = { outcome: second, parts: [], plans: [], ancestors: [], revision: 1 }
  const writes: { anchor: ThreadPlanningAnchor; command_id: string }[] = []
  let finish!: () => void
  const committed = new Promise<void>(resolve => { finish = resolve })
  routes['PUT /api/threads/t1/anchor'] = async (request: Request) => {
    const body = await request.json() as (typeof writes)[number]
    writes.push(body)
    if (writes.length === 1) throw new TypeError('Lost response before the write settled')
    if (body.anchor.id === 'launch') await committed
    return Response.json({ anchor: body.anchor })
  }
  const app = await show(routes)
  await click(app, 'Anchor…')
  await click(app, 'Outcomes')
  await click(app, 'Select Launch')
  await until(() => app.text().includes('The daemon is not reachable'))
  await click(app, 'Select Verify')
  await until(() => writes.length === 2)
  expect(writes[1]).toEqual(writes[0])
  await act(async () => finish())
  await until(() => app.text().includes('Anchored at Verify'))
  expect(writes[2]?.anchor.id).toBe('verify')
  expect(writes[2]?.command_id).not.toBe(writes[0]?.command_id)
})

it('send_again_says_the_next_message_carries_it', async () => {
  const routes = answers()
  routes['POST /api/threads/t1/context/resend'] = { owed: true }
  const app = await show(routes)
  await until(() => app.text().includes('Not anchored'))
  await click(app, 'Send context again')
  await until(() => app.text().includes('Your next normal message carries it again'))
  expect(app.bodies).toContainEqual({ command_id: expect.any(String) })
  expect(app.calls).not.toContain('POST /api/threads/t1/turns')
})

it('not_shown_for_aylul', async () => {
  const routes = answers()
  routes['GET /api/projects/p1/threads'] = [{ ...threadFixture, harness: 'aylul' }]
  const app = await show(routes)
  await until(() => app.text().includes(threadFixture.title))
  expect(app.button('Send context again')).toBeUndefined()
  expect(app.calls).not.toContain('GET /api/threads/t1/anchor')
})
