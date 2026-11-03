// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { answers } from '@/test/fake-daemon'
import { startApp, typeInto, until, type Answer, type TestApp } from '@/test/test-app'
import type { Message } from '@/api/groups'

const group = {
  id: 'g1', name: 'Team', needs_you: false, created_at: 'now',
  members: [{ slug: 'alpha', name: 'Alpha', thread_id: 'hidden1' }, { slug: 'gone', name: 'Gone', thread_id: 'hidden2' }],
}
const message = (seq: number, body: string, kind: Message['kind'] = 'chat'): Message => ({
  id: `m${seq}`, group_id: 'g1', seq, body, kind, created_at: 'now', to: [],
  speaker: kind === 'notice' ? { kind: 'shadows' } : { kind: 'agent', slug: 'gone', name: 'Gone' },
})
let app: TestApp | undefined
afterEach(() => { app?.unmount(); app = undefined })
async function open(overrides: Record<string, Answer> = {}) {
  const a = app = await startApp('/aylul/groups/g1', {
    ...answers(),
    'GET /api/aylul/agents': { project_id: 'p1', agents: [{ slug: 'alpha', name: 'Alpha' }] },
    'GET /api/groups': [group], 'GET /api/groups/g1': group,
    'GET /api/groups/g1/messages': [message(1, 'Earlier'), message(2, 'A system notice', 'notice')],
    'GET /api/threads/hidden1/operations': [], 'GET /api/threads/hidden2/operations': [],
    'GET /api/aylul/permissions': [],
    ...overrides,
  })
  await until(() => a.button('Send message') !== undefined)
  return a
}

it('renders ordered messages with their speaker and renders notices outside bubbles', async () => {
  const a = await open({ 'GET /api/groups/g1/messages': [message(2, 'A system notice', 'notice'), message(1, 'Earlier')] })
  await until(() => a.text().includes('A system notice'))
  const log = a.container.querySelector('[aria-label="Group messages"]')!
  expect(log.textContent).toMatch(/Gone.*Earlier.*A system notice/)
  expect(log.querySelector('[data-message-kind="notice"]')?.tagName).toBe('P')
})

it('offers only live group members in mention completion', async () => {
  const a = await open()
  typeInto(a.container.querySelector('textarea')!, '@')
  await until(() => a.button('@alpha') !== undefined)
  expect(a.button('@gone')).toBeUndefined()
  expect(a.button('@mohammed')).toBeUndefined()
})

it('retries a send whose committed response was lost with the same command id', async () => {
  let sends = 0
  const a = await open({
    'POST /api/groups/g1/messages': () => {
      if (++sends === 1) throw new TypeError('Failed to fetch')
      return Response.json(message(3, 'Hello'))
    },
  })
  const input = a.container.querySelector('textarea')!
  typeInto(input, 'Hello')
  act(() => a.button('Send message')!.click())
  await until(() => sends === 1 && a.button('Send message')?.disabled === false)
  expect(input.value).toBe('Hello')
  act(() => a.button('Send message')!.click())
  await until(() => input.value === '')
  expect(a.bodies[1]).toEqual(a.bodies[0])
})

it('shows and clears thinking on thread events without a group message', async () => {
  let state: { status_kind: string }[] = []
  const a = await open({ 'GET /api/threads/hidden1/operations': () => Response.json(state) })
  await until(() => a.sources.some(s => s.param('thread_id') === 'hidden1'))
  const source = a.sources.find(s => s.param('thread_id') === 'hidden1')!
  act(() => source.caughtUp(0))
  state = [{ status_kind: 'Running' }]
  act(() => source.durable(1, 'OperationStarted', 'hidden1', {}))
  await until(() => a.text().includes('Alpha is thinking'))
  state = [{ status_kind: 'Completed' }]
  act(() => source.durable(2, 'OperationCompleted', 'hidden1', {}))
  await until(() => !a.text().includes('Alpha is thinking'))
})

it('keeps a scrolled-up reader in place when two messages arrive', async () => {
  let messages = [message(1, 'Earlier')]
  const a = await open({ 'GET /api/groups/g1/messages': () => Response.json(messages) })
  await until(() => a.text().includes('Earlier'))
  const log = a.container.querySelector<HTMLElement>('[aria-label="Group messages"]')!
  Object.defineProperties(log, { clientHeight: { value: 200 }, scrollHeight: { value: 1000 } })
  log.scrollTop = 100
  act(() => log.dispatchEvent(new Event('scroll')))
  const project = a.sources.find(s => s.url.includes('/api/projects/p1/events'))!
  act(() => project.caughtUp(0))
  messages = [...messages, message(2, 'Second'), message(3, 'Third')]
  act(() => project.durable(1, 'MessageAdded', null, { group_id: 'g1', seq: 3 }))
  await until(() => a.button('New messages') !== undefined)
  expect(log.scrollTop).toBe(100)
  act(() => a.button('New messages')!.click())
  expect(a.button('New messages')).toBeUndefined()
})

it('shows the member and request title before answering in place', async () => {
  const permission = {
    thread_id: 'hidden1', request_id: 'r1', title: 'Run a command', agent: 'alpha', asked_at: 'now',
    options: [{ option_id: 'allow', name: 'Allow once', kind: 'allow_once' }],
  }
  let waiting = true
  const a = await open({
    'GET /api/aylul/permissions': () => Response.json(waiting ? [permission, { ...permission, thread_id: 'other' }] : []),
    'POST /api/threads/hidden1/session/permissions/r1': () => {
      waiting = false
      return Response.json({ request_id: 'r1', option_id: 'allow' })
    },
  })
  await until(() => a.button('Allow once') !== undefined)
  const requests = a.container.querySelector('[aria-label="Group permission requests"]')
  expect(requests?.textContent).toMatch(/Alpha.*Run a command.*Allow once/)
  expect(a.buttons('Allow once')).toHaveLength(1)
  act(() => a.button('Allow once')!.click())
  await until(() => a.button('Allow once') === undefined)
  expect(a.path()).toBe('/aylul/groups/g1')
  expect(a.bodies.at(-1)).toEqual({ command_id: expect.any(String), option_id: 'allow' })
})
