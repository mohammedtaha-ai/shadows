// @vitest-environment happy-dom
//

import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { runningOperation } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, typeInto, until } from '@/test/test-app'

const waiting = {
  id: 'q1',
  thread_id: 't1',
  position: 1,
  prompt: 'And the tests too',
  model: 'fake-small',
  mode: 'acceptEdits',
  effort: 'high',
  focus: null,
  plan: null,
  last_error: null,
  created_at: 'x',
}

const DAEMON = {
  ...answers({ operations: [runningOperation()] }),
  'GET /api/threads/t1/queue': [waiting],
  'POST /api/threads/t1/queue': () =>
    Response.json({ status: 'waiting', message: { ...waiting, id: 'q2' } }, { status: 202 }),
  'POST /api/threads/t1/queue/q1/send-now': () =>
    Response.json({ status: 'steered', entry_id: 'e9' }, { status: 202 }),
  'DELETE /api/threads/t1/queue/q1': () => new Response(null, { status: 204 }),
}

let app: TestApp | null = null
afterEach(() => {
  app?.unmount()
  app = null
})

it('queues on Enter while a turn runs, and offers Send now and Remove', async () => {
  const a = (app = await startApp('/projects/p1/threads/t1', DAEMON))
  await until(() => a.button('Stop') !== undefined)
  await until(() => a.text().includes('And the tests too'))
  expect(a.text()).toContain('Waiting')

  const box = a.container.querySelector('textarea')
  if (box === null) throw new Error('the composer is not enabled while a turn runs')
  await until(() => a.calls.includes('POST /api/threads/t1/session'))
  await act(async () => {
    typeInto(box, 'One more thing')
  })
  await act(async () => {
    box.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }))
  })
  await until(() => a.calls.includes('POST /api/threads/t1/queue'))

  await act(async () => a.button('Send now')?.click())
  await until(() => a.calls.includes('POST /api/threads/t1/queue/q1/send-now'))

  await act(async () => a.button('Remove')?.click())
  await until(() => a.calls.includes('DELETE /api/threads/t1/queue/q1'))
})

it.each(['INVALID_COMMAND', 'COMMAND_CONFLICT'] as const)('Send now preserves queued images and retries %s with the correct command', async code => {
  const requests: { command_id: string }[] = []
  const a = app = await startApp('/projects/p1/threads/t1', {
    ...DAEMON,
    'GET /api/threads/t1/queue': [{ ...waiting, attachments: ['image-1', 'image-2'] }],
    'POST /api/threads/t1/queue/q1/send-now': async (request: Request) => {
      requests.push(await request.json() as { command_id: string })
      return requests.length === 1 ? Response.json({ code, message: 'Core refused Send now' }, { status: 409 })
        : Response.json({ status: 'steered', entry_id: 'e9' }, { status: 202 })
    },
  })
  await until(() => a.button('Send now') !== undefined)
  act(() => a.button('Send now')!.click())
  await until(() => a.text().includes('Core refused Send now') && a.button('Send now')?.disabled === false)
  expect(a.container.querySelectorAll('img[alt="Queued image"]')).toHaveLength(2)
  act(() => a.button('Send now')!.click())
  await until(() => requests.length === 2)
  if (code === 'COMMAND_CONFLICT') expect(requests[1]?.command_id).not.toBe(requests[0]?.command_id)
  else expect(requests[1]).toEqual(requests[0])
})
