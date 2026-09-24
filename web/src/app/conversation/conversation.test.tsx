// @vitest-environment happy-dom
//
// The conversation screen over a faked daemon: opened (as after a reload) while
// a turn is running, it shows Running and a Stop that reaches the daemon, and
// the turn's durable ending returns it to Send.

import { act } from 'react'
import { afterEach, describe, expect, it } from 'vitest'
import { type TestApp, startApp, until } from '../test-app'

const operation = (status_kind: string) => ({
  id: 'op1',
  kind: 'PlannerTurn',
  status_kind,
  thread_id: 't1',
  runtime_instance_id: 'r',
  created_at: '2026-09-23T00:00:00Z',
  invocation: null,
})

const DAEMON = {
  'GET /api/projects': [
    { id: 'p1', slug: 'demo', name: 'Demo', directory: 'C:\\work\\demo', created_at: 'x' },
  ],
  'GET /api/projects/p1/threads': [
    { id: 't1', project_id: 'p1', title: 'Conversation 1', status: 'Open', created_at: 'x' },
  ],
  'GET /api/threads/t1/entries': [
    {
      id: 'e1',
      thread_id: 't1',
      ordinal: 1,
      kind: 'UserMessage',
      author: { kind: 'User', id: 'local' },
      body: 'Plan the thing',
      refs: [],
      created_at: 'x',
    },
  ],
  'GET /api/threads/t1/operations': [operation('Running')],
  'POST /api/operations/op1/stop': operation('Running'),
}

let app: TestApp | null = null
afterEach(() => {
  app?.unmount()
  app = null
})

describe('the conversation, opened while a turn runs', () => {
  it('shows Running and a Stop that reaches the daemon, until the turn ends', async () => {
    const a = (app = await startApp('/projects/p1/threads/t1', DAEMON))
    await until(() => a.button('Stop') !== undefined)
    expect(a.container.textContent).toContain('Running')
    expect(a.container.textContent).toContain('Plan the thing')
    expect(a.container.textContent).toContain('Runs in C:\\work\\demo')
    expect(a.button('Send')).toBeUndefined()

    await act(async () => a.button('Stop')?.click())
    await until(() => a.calls.includes('POST /api/operations/op1/stop'))
    await until(() => a.button('Stopping…') !== undefined)

    // The stream replays the turn's history, goes live, then the durable end.
    const stream = a.sources.at(-1)
    if (stream === undefined) throw new Error('no stream was opened')
    await act(async () => {
      stream.durable(5, 'OperationCreated', 'op1', { kind: 'PlannerTurn' })
      stream.durable(6, 'OperationStarted', 'op1', {})
      stream.caughtUp(6)
      stream.durable(7, 'OperationCancellationRequested', 'op1', {})
      stream.durable(8, 'OperationCancelled', 'op1', {})
    })
    await until(() => a.button('Send') !== undefined)
    expect(a.button('Stop')).toBeUndefined()
    expect(a.container.textContent).toContain('Stopped')
  })
})

describe('a stop the daemon could not carry out', () => {
  it('shows why and can be asked again, even though the request is durable', async () => {
    let stops = 0
    const a = (app = await startApp('/projects/p1/threads/t1', {
      ...DAEMON,
      'POST /api/operations/op1/stop': () => {
        stops += 1
        return Response.json(
          { code: 'PROCESS_TERMINATION_FAILED', message: 'the tree could not be terminated' },
          { status: 500 },
        )
      },
    }))
    await until(() => a.button('Stop') !== undefined)
    await act(async () => a.button('Stop')?.click())

    // The request was recorded before termination failed, so the stream
    // says a stop was asked for; the turn is still running.
    const stream = a.sources.at(-1)
    if (stream === undefined) throw new Error('no stream was opened')
    await act(async () => {
      stream.durable(5, 'OperationCreated', 'op1', { kind: 'PlannerTurn' })
      stream.durable(6, 'OperationStarted', 'op1', {})
      stream.durable(7, 'OperationCancellationRequested', 'op1', {})
      stream.caughtUp(7)
    })

    await until(() => a.button('Stop again') !== undefined)
    expect(a.button('Stop again')?.disabled).toBe(false)
    expect(a.container.textContent).toContain('PROCESS_TERMINATION_FAILED')

    await act(async () => a.button('Stop again')?.click())
    await until(() => stops === 2)
  })
})
