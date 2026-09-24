// @vitest-environment happy-dom
//
// The conversation's CLI picker and composer bar over a faked daemon: the
// session opens with words, never a bare spinner; every menu is built from
// the session's answer; Send carries the chosen settings and a command id
// that a retry of the same send reuses.

import { act } from 'react'
import { afterEach, describe, expect, it } from 'vitest'
import { completedOperation, fakeChoices } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, choose, menuItem, startApp, typeInto, until } from '../test-app'

let open: TestApp | null = null
afterEach(() => {
  open?.unmount()
  open = null
})

async function start(url: string, table: ReturnType<typeof answers>): Promise<TestApp> {
  open = await startApp(url, table)
  return open
}

/** The session answered and the thread's turns are known: Send can act. */
const ready = (app: TestApp) => () =>
  app.button('fake-large') !== undefined && app.calls.includes('GET /api/threads/t1/operations')

const textarea = (app: TestApp) => {
  const box = app.container.querySelector('textarea')
  if (box === null) throw new Error('no message box')
  return box
}

const turnStarts = (app: TestApp) =>
  app.calls.filter((c) => c === 'POST /api/threads/t1/turns').length

describe('the composer bar', () => {
  it('says it is connecting, then builds every menu from the session', async () => {
    let release!: () => void
    const app = await start(
      '/projects/p1/threads/t1',
      answers({
        session: () =>
          new Promise<Response>((r) => {
            release = () => r(Response.json(fakeChoices))
          }),
      }),
    )
    await until(() => app.text().includes('Connecting to Claude Code…'))
    act(() => release())
    await until(() => app.button('fake-large') !== undefined)
    expect(app.text()).not.toContain('Connecting')
    expect(app.button('Accept edits')).toBeDefined()
    expect(app.button('high')).toBeDefined()
  })

  it('a frame sent while the session opens does not override its answer', async () => {
    // Found in the Phase B run: opening sets mode, model, then effort, and
    // each step's frame arrives before the opening answers. The answer holds
    // the result; a frame from before it is a step on the way.
    const opened = { ...fakeChoices, current: { ...fakeChoices.current, effort: 'max' } }
    let release!: () => void
    const app = await start(
      '/projects/p1/threads/t1',
      answers({
        session: () =>
          new Promise<Response>((r) => {
            release = () => r(Response.json(opened))
          }),
      }),
    )
    await until(() => app.text().includes('Connecting to Claude Code…'))
    act(() => app.pushFrame('caught-up', { seq: 0 }))
    act(() => app.pushFrame('options', { thread_id: 't1', choices: fakeChoices }))
    act(() => app.pushFrame('options', { thread_id: 't1', choices: opened }))
    await new Promise((r) => setTimeout(r, 50))
    expect(app.text()).toContain('Connecting to Claude Code…')
    act(() => release())
    await until(() => app.button('max') !== undefined)
    expect(app.button('high')).toBeUndefined()
  })

  it('a failed opening shows the message and a retry, never an endless spinner', async () => {
    let tries = 0
    const app = await start(
      '/projects/p1/threads/t1',
      answers({
        session: () => {
          tries += 1
          return Response.json(
            { code: 'HARNESS_START_FAILED', message: 'node was not found' },
            { status: 502 },
          )
        },
      }),
    )
    await until(() => app.text().includes('node was not found'))
    expect(app.button('Retry')).toBeDefined()
    act(() => app.button('Retry')?.click())
    await until(() => tries === 2)
  })

  it('choosing a value closes its menu, so it never covers Send', async () => {
    const app = await start('/projects/p1/threads/t1', answers())
    await until(ready(app))
    await choose(app, 'Accept edits', 'Auto')
    await until(() => menuItem('Auto') === undefined)
    expect(app.button('Auto')).toBeDefined()
  })

  it('sends the chosen model, mode and effort', async () => {
    const app = await start('/projects/p1/threads/t1', answers())
    await until(ready(app))
    await choose(app, 'Accept edits', 'Auto')
    typeInto(textarea(app), 'hi')
    act(() => app.button('Send')?.click())
    await until(() => app.calls.includes('POST /api/threads/t1/turns'))
    expect(app.bodies.at(-1)).toMatchObject({
      model: 'fake-large',
      mode: 'auto',
      effort: 'high',
      prompt: 'hi',
    })
  })

  it('a model without auto moves the mode and the bar says so', async () => {
    const app = await start('/projects/p1/threads/t1', answers())
    await until(ready(app))
    await choose(app, 'Accept edits', 'Auto')
    await choose(app, 'fake-large', 'fake-small')
    // the daemon answers the model change with an options frame whose current mode is acceptEdits
    act(() =>
      app.pushFrame('options', {
        thread_id: 't1',
        choices: {
          ...fakeChoices,
          current: { model: 'fake-small', mode: 'acceptEdits', effort: 'high' },
        },
      }),
    )
    await until(() => app.text().includes('fake-small does not offer Auto; switched to Accept edits'))
    expect(app.button('Accept edits')).toBeDefined()
  })

  it('the CLI picker changes the harness before the first turn and shows a lock after', async () => {
    const app = await start('/projects/p1/threads/t1', answers({ operations: [] }))
    await until(() => app.button('Claude Code') !== undefined)
    act(() => app.button('Claude Code')?.click())
    await until(() => menuItem('Codex') !== undefined)
    expect(menuItem('Codex')?.getAttribute('aria-disabled')).toBe('true')
    app.unmount()
    open = null

    const locked = await start(
      '/projects/p1/threads/t1',
      answers({ operations: [completedOperation(null)] }),
    )
    await until(
      () =>
        locked.container.querySelector('[aria-label="CLI locked for this conversation"]') !== null,
    )
  })

  it('a retried Send reuses the same command id', async () => {
    let n = 0
    const app = await start(
      '/projects/p1/threads/t1',
      answers({
        start: () =>
          ++n === 1
            ? Promise.reject(new TypeError('network down'))
            : Response.json({ operation_id: 'op1' }, { status: 202 }),
      }),
    )
    await until(ready(app))
    typeInto(textarea(app), 'hi')
    act(() => app.button('Send')?.click())
    await until(() => turnStarts(app) === 1 && app.button('Send')?.disabled === false)
    act(() => app.button('Send')?.click())
    await until(() => turnStarts(app) === 2)
    const [a, b] = app.bodies.slice(-2) as { command_id: string }[]
    expect(b.command_id).toBe(a.command_id)
  })

  it('changing a setting after a failed send makes a new command id', async () => {
    let n = 0
    const app = await start(
      '/projects/p1/threads/t1',
      answers({
        start: () =>
          ++n === 1
            ? Promise.reject(new TypeError('network down'))
            : Response.json({ operation_id: 'op1' }, { status: 202 }),
      }),
    )
    await until(ready(app))
    typeInto(textarea(app), 'hi')
    act(() => app.button('Send')?.click())
    await until(() => turnStarts(app) === 1 && app.button('Send')?.disabled === false)
    await choose(app, 'Accept edits', 'Auto')
    act(() => app.button('Send')?.click())
    await until(() => turnStarts(app) === 2)
    const [a, b] = app.bodies.slice(-2) as { command_id: string }[]
    expect(b.command_id).not.toBe(a.command_id)
  })

  it('shows the refusal when the daemon refuses the mode and keeps the prompt', async () => {
    const app = await start(
      '/projects/p1/threads/t1',
      answers({
        start: () =>
          Response.json({ code: 'MODE_NOT_ALLOWED', message: 'auto is not allowed' }, { status: 403 }),
      }),
    )
    await until(ready(app))
    typeInto(textarea(app), 'hi')
    act(() => app.button('Send')?.click())
    await until(() => app.text().includes('auto is not allowed'))
    expect(textarea(app).value).toBe('hi')
  })
})
