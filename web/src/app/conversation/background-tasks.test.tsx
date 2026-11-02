// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { answers } from '@/test/fake-daemon'
import { startApp, until, type TestApp } from '@/test/test-app'
let app: TestApp | null = null
afterEach(() => { app?.unmount(); app = null })
it('shows streamed tasks and stops only the selected task', async () => {
  app = await startApp('/projects/p1/threads/t1', { ...answers(),
    'POST /api/threads/t1/session/tasks/a/stop': () => Response.json({ stopped: true }) })
  const a = app
  await until(() => a.sources.some(s => s.param('thread_id') === 't1'))
  const source = a.sources.find(s => s.param('thread_id') === 't1')!
  expect(a.button('Background tasks: 1 running')).toBeUndefined()
  const task = { id: 'a', name: 'Build', description: 'Compile', task_type: 'bash', state: 'running', started_at: Date.now() - 5000, ended_at: null, summary: null, can_stop: true }
  await act(async () => { source.caughtUp(0); source.emit('tasks', JSON.stringify({ op: null, tasks: [task] })) })
  await until(() => a.button('Background tasks: 1 running') !== undefined)
  act(() => a.button('Background tasks: 1 running')!.click())
  expect(a.text()).toContain('Build')
  act(() => a.button('Stop Build')!.click())
  await until(() => a.calls.includes('POST /api/threads/t1/session/tasks/a/stop'))
  await act(async () => source.emit('tasks', JSON.stringify({ op: null, tasks: [{ ...task, state: 'completed', ended_at: Date.now() }] })))
  expect(a.text()).toContain('completed')
  expect(a.button('Stop Build')).toBeUndefined()
  await act(async () => source.emit('lagged'))
  expect(a.button('Background tasks: 0 running')).toBeUndefined()
  const next = a.sources.filter(s => s.param('thread_id') === 't1').at(-1)!
  await act(async () => { next.caughtUp(0); next.emit('tasks', JSON.stringify({ op: null, tasks: [task] })) })
  await until(() => a.button('Background tasks: 1 running') !== undefined)
})
