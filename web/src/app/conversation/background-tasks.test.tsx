// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { answers } from '@/test/fake-daemon'
import { startApp, until, type TestApp } from '@/test/test-app'
import type { FakeSource } from '@/test/fake-event-source'

let app: TestApp | null = null
afterEach(() => { app?.unmount(); app = null })

const task = { id: 'a', name: 'Build', description: 'Compile', task_type: 'bash', state: 'running', started_at: Date.now() - 5000, ended_at: null, summary: null, can_stop: true }

async function open(extra = {}): Promise<{ a: TestApp; source: FakeSource }> {
  app = await startApp('/projects/p1/threads/t1', { ...answers(), ...extra })
  const a = app
  await until(() => a.sources.some(s => s.param('thread_id') === 't1'))
  return { a, source: a.sources.find(s => s.param('thread_id') === 't1')! }
}

const countButton = () => [...document.querySelectorAll('button')]
  .find(b => b.textContent?.trim().startsWith('Background tasks:'))

it('shows streamed tasks and stops only the selected task', async () => {
  const { a, source } = await open({
    'POST /api/threads/t1/session/tasks/a/stop': () => Response.json({ stopped: true }),
  })
  expect(a.button('Background tasks: 1 running')).toBeUndefined()
  await act(async () => { source.caughtUp(0); source.emit('tasks', JSON.stringify({ op: null, tasks: [task] })) })
  await until(() => a.button('Background tasks: 1 running') !== undefined)
  act(() => a.button('Background tasks: 1 running')!.click())
  expect(a.text()).toContain('Build')
  act(() => a.button('Stop Build')!.click())
  await until(() => a.calls.includes('POST /api/threads/t1/session/tasks/a/stop'))
  expect(a.calls.filter(call => call.includes('/session/tasks/'))).toEqual(['POST /api/threads/t1/session/tasks/a/stop'])
  await act(async () => source.emit('tasks', JSON.stringify({ op: null, tasks: [{ ...task, state: 'completed', ended_at: Date.now() }] })))
  expect(a.text()).toContain('completed')
  expect(a.button('Stop Build')).toBeUndefined()
  await act(async () => source.emit('lagged'))
  expect(a.button('Background tasks: 0 running')).toBeUndefined()
  const next = a.sources.filter(s => s.param('thread_id') === 't1').at(-1)!
  await act(async () => { next.caughtUp(0); next.emit('tasks', JSON.stringify({ op: null, tasks: [task] })) })
  await until(() => a.button('Background tasks: 1 running') !== undefined)
})

it('the count is hidden while the session has no tasks', async () => {
  const { a, source } = await open()
  await act(async () => source.caughtUp(0))
  expect(countButton()).toBeUndefined()
  await act(async () => source.emit('tasks', JSON.stringify({ op: null, tasks: [{ ...task, state: 'completed', ended_at: Date.now() }] })))
  await until(() => a.button('Background tasks: 0 running') !== undefined)
  // Closing the session sends an empty list.
  await act(async () => source.emit('tasks', JSON.stringify({ op: null, tasks: [] })))
  expect(countButton()).toBeUndefined()
})

it('finished tasks are listed under the running ones with their state', async () => {
  const { a, source } = await open()
  const failed = { ...task, id: 'f', name: 'Lint', state: 'failed', ended_at: Date.now(), summary: 'exit 1' }
  const stopped = { ...task, id: 's', name: 'Watch', state: 'stopped', ended_at: Date.now() }
  await act(async () => { source.caughtUp(0); source.emit('tasks', JSON.stringify({ op: null, tasks: [failed, task, stopped] })) })
  await until(() => a.button('Background tasks: 1 running') !== undefined)
  act(() => a.button('Background tasks: 1 running')!.click())
  const running = document.querySelector('section[aria-label="Running"]')!
  const finished = document.querySelector('section[aria-label="Finished"]')!
  expect(running.textContent).toContain('Build')
  expect(finished.textContent).toContain('Lint')
  expect(finished.textContent).toContain('failed')
  expect(finished.textContent).toContain('exit 1')
  expect(finished.textContent).toContain('Watch')
  expect(finished.textContent).toContain('stopped')
  expect(running.compareDocumentPosition(finished) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
  expect(a.button('Stop Lint')).toBeUndefined()
})

it('a stop in flight is pending and a refused stop shows its error', async () => {
  let release!: (response: Response) => void
  const answer = new Promise<Response>(resolve => { release = resolve })
  const { a, source } = await open({ 'POST /api/threads/t1/session/tasks/a/stop': () => answer })
  await act(async () => { source.caughtUp(0); source.emit('tasks', JSON.stringify({ op: null, tasks: [task] })) })
  await until(() => a.button('Background tasks: 1 running') !== undefined)
  act(() => a.button('Background tasks: 1 running')!.click())
  act(() => a.button('Stop Build')!.click())
  await until(() => a.button('Stop Build')!.disabled)
  expect(a.button('Stop Build')!.textContent).toBe('Stopping…')
  await act(async () => release(Response.json({ code: 'HARNESS_START_FAILED', message: 'The adapter did not answer.' }, { status: 502 })))
  await until(() => a.text().includes('The adapter did not answer.'))
  expect(a.button('Stop Build')!.disabled).toBe(false)
})
