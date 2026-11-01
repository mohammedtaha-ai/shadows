// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { answers } from '@/test/fake-daemon'
import { entryOfKind } from '@/test/contract-fixtures'
import { startApp, until, type TestApp, type Answer } from '@/test/test-app'

const options = [{ option_id: 'allow', name: 'Allow once', kind: 'allow_once' }, { option_id: 'reject', name: 'Reject', kind: 'reject_once' }]
const request = { request_id: 'r1', thread_id: 't1', agent: 'work', title: 'Run a command', options, asked_at: '2026-11-01T12:00:00Z' }
const asked = { ...entryOfKind('permission1', 'PermissionAsked', 'Run a command'), card: { request_id: 'r1', options } }
let app: TestApp | null = null
afterEach(() => { app?.unmount(); app = null })
async function open(overrides: Record<string, Answer> = {}) {
  const a = app = await startApp('/projects/p1/threads/t1', { ...answers(),
    'GET /api/threads/t1/entries': [asked],
    'GET /api/aylul/permissions': [request],
    ...overrides,
  })
  await until(() => a.text().includes('Run a command'))
  return a
}
it('only a live matching request offers option buttons and a click answers it', async () => {
  let waiting = true
  const a = await open({
    'GET /api/aylul/permissions': () => Response.json(waiting ? [request] : []),
    'POST /api/threads/t1/session/permissions/r1': () => { waiting = false; return Response.json({ request_id: 'r1', option_id: 'allow' }) },
  })
  await until(() => a.button('Allow once') !== undefined)
  expect(a.button('Reject')).toBeDefined()
  expect(document.querySelector('[aria-label="Permission answer"]')).toBeNull()
  act(() => a.button('Allow once')!.click())
  await until(() => a.calls.includes('POST /api/threads/t1/session/permissions/r1') && a.button('Allow once') === undefined)
  expect(a.bodies.at(-1)).toEqual({ command_id: expect.any(String), option_id: 'allow' })
})
it('a past card or a request in another thread cannot be answered', async () => {
  const a = await open({ 'GET /api/aylul/permissions': [{ ...request, thread_id: 'other' }] })
  await until(() => a.calls.includes('GET /api/aylul/permissions'))
  expect(a.button('Allow once')).toBeUndefined()
  expect(a.button('Reject')).toBeUndefined()
})
it('the project event removes a request settled by another surface', async () => {
  let waiting = true
  const a = await open({ 'GET /api/aylul/permissions': () => Response.json(waiting ? [request] : []) })
  await until(() => a.button('Allow once') !== undefined)
  const project = a.sources.find(source => new URL(source.url).pathname === '/api/projects/p1/events')!
  act(() => project.caughtUp(0))
  waiting = false
  act(() => project.durable(1, 'PermissionsChanged', null, { request_id: 'r1', state: 'answered' }))
  await until(() => a.button('Allow once') === undefined)
  expect(a.calls).not.toContain('POST /api/threads/t1/session/permissions/r1')
})
it('one answer stays in flight and every option is disabled until it settles', async () => {
  let release!: (value: Response) => void
  const a = await open({ 'POST /api/threads/t1/session/permissions/r1': () => new Promise<Response>(resolve => { release = resolve }) })
  await until(() => a.button('Allow once') !== undefined)
  act(() => { a.button('Allow once')!.click(); a.button('Allow once')!.click() })
  await until(() => a.bodies.length === 1)
  expect(a.button('Allow once')!.disabled).toBe(true)
  expect(a.button('Reject')!.disabled).toBe(true)
  await act(async () => release(Response.json({ request_id: 'r1', option_id: 'allow' })))
  await until(() => a.button('Allow once') === undefined)
})
it('an uncertain failed answer retries with the same command id', async () => {
  let count = 0
  const a = await open({ 'POST /api/threads/t1/session/permissions/r1': () => ++count === 1
    ? Response.json({ code: 'STORAGE_UNAVAILABLE', message: 'Answer failed.' }, { status: 503 })
    : Response.json({ request_id: 'r1', option_id: 'allow' }),
  })
  await until(() => a.button('Allow once') !== undefined)
  act(() => a.button('Allow once')!.click())
  await until(() => a.text().includes('Answer failed.') && a.button('Allow once')?.disabled === false)
  act(() => a.button('Allow once')!.click()); await until(() => count === 2)
  expect(a.bodies[1]).toEqual(a.bodies[0])
})
it.each(['PERMISSION_NOT_FOUND', 'PERMISSION_SETTLED'])('%s refetches the waiting list and shows Core message', async code => {
  let waiting = true
  const a = await open({
    'GET /api/aylul/permissions': () => Response.json(waiting ? [request] : []),
    'POST /api/threads/t1/session/permissions/r1': () => { waiting = false; return Response.json({ code, message: 'This request is no longer waiting.' }, { status: code === 'PERMISSION_NOT_FOUND' ? 404 : 409 }) },
  })
  await until(() => a.button('Allow once') !== undefined)
  const reads = a.calls.filter(call => call === 'GET /api/aylul/permissions').length
  act(() => a.button('Allow once')!.click())
  await until(() => a.text().includes('This request is no longer waiting.') && a.button('Allow once') === undefined)
  expect(a.calls.filter(call => call === 'GET /api/aylul/permissions').length).toBeGreaterThan(reads)
})
it('answered and refused entries display the chosen option and refusal reason', async () => {
  const a = await open({ 'GET /api/threads/t1/entries': [asked,
    { ...entryOfKind('permission2', 'PermissionAnswered', 'Allow once'), ordinal: 2, card: { request_id: 'r1', by: { agent: 'human', principal: 'user:local' } } },
    { ...entryOfKind('permission3', 'PermissionRefused', 'Another command'), ordinal: 3, card: { request_id: 'r2', reason: 'cancelled' } },
  ], 'GET /api/aylul/permissions': [] })
  await until(() => a.text().includes('by you'))
  expect(a.text()).toContain('Allow once')
  expect(a.text()).toContain('cancelled')
  expect(a.text()).not.toContain('would allow it')
})
