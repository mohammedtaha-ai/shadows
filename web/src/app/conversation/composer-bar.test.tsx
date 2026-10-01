// @vitest-environment happy-dom
//
// The conversation's CLI picker and composer bar over a faked daemon: the
// session opens with words, never a bare spinner; every menu is built from
// the session's answer; Send carries the chosen settings and a command id
// that a retry of the same send reuses.

import { act } from 'react'
import { afterEach, describe, expect, it } from 'vitest'
import { choice, completedOperation, fakeChoices, threadFixture } from '@/test/contract-fixtures'
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
    // Inside act: what the frames set off settles here, not in a gap React
    // does not see.
    await act(async () => {
      await new Promise((r) => setTimeout(r, 50))
    })
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
    // The daemon answers the model change with the session's choices, whose
    // current mode is acceptEdits.
    await choose(app, 'fake-large', 'fake-small')
    await until(() => app.text().includes('fake-small does not offer Auto; switched to Accept edits'))
    expect(app.button('Accept edits')).toBeDefined()
  })

  it('picking a model sets it on the session at once, so its efforts show before Send', async () => {
    let release!: () => void
    let held = fakeChoices
    const small = {
      ...fakeChoices,
      efforts: [choice('low'), choice('high')],
      current: { ...fakeChoices.current, model: 'fake-small' },
    }
    const app = await start(
      '/projects/p1/threads/t1',
      answers({
        model: () =>
          new Promise<Response>((r) => {
            release = () => {
              held = small
              r(Response.json(small))
            }
          }),
        // The session's model is kept, as the daemon keeps it.
        effort: async (r: Request) => {
          const { effort } = (await r.json()) as { effort: string }
          return Response.json({ ...held, current: { ...held.current, effort } })
        },
      }),
    )
    await until(ready(app))
    await choose(app, 'high', 'max')
    await until(() => app.button('max')?.disabled === false)
    await choose(app, 'fake-large', 'fake-small')
    await until(() => app.bodies.length === 2)
    expect(app.calls.at(-1)).toBe('PUT /api/threads/t1/session/model')
    expect(app.bodies).toEqual([{ effort: 'max' }, { model: 'fake-small' }])
    // While the session changes, the effort menu is off and Send waits.
    typeInto(textarea(app), 'hi')
    expect(app.button('max')?.disabled).toBe(true)
    expect(app.button('Send')?.disabled).toBe(true)
    act(() => release())
    // fake-small offers no max: the effort moves to one it offers.
    await until(() => app.button('low')?.disabled === false)
    act(() => app.button('low')?.click())
    await until(() => menuItem('high') !== undefined)
    expect(menuItem('max')).toBeUndefined()
    expect(app.button('Send')?.disabled).toBe(false)
    expect(turnStarts(app)).toBe(0)
  })

  it('picking an effort sets it on the session at once', async () => {
    const app = await start('/projects/p1/threads/t1', answers())
    await until(ready(app))
    await choose(app, 'high', 'max')
    await until(() => app.bodies.length === 1)
    expect(app.calls.at(-1)).toBe('PUT /api/threads/t1/session/effort')
    expect(app.bodies).toEqual([{ effort: 'max' }])
    await until(() => app.button('max')?.disabled === false)
    typeInto(textarea(app), 'hi')
    await until(() => app.button('Send')?.disabled === false)
    act(() => app.button('Send')?.click())
    await until(() => app.calls.includes('POST /api/threads/t1/turns'))
    expect(app.bodies.at(-1)).toMatchObject({ effort: 'max' })
  })

  it('an effort the harness refuses goes back to the session’s, on the error line', async () => {
    const message = 'effort max was refused by the harness: effort not offered'
    const app = await start(
      '/projects/p1/threads/t1',
      answers({
        effort: () => Response.json({ code: 'SETTING_NOT_OFFERED', message }, { status: 422 }),
      }),
    )
    await until(ready(app))
    await choose(app, 'high', 'max')
    await until(() => app.text().includes(message))
    expect(app.button('high')).toBeDefined()
    expect(app.button('max')).toBeUndefined()
    expect(app.calls.filter((c) => c === 'PUT /api/threads/t1/session/effort')).toHaveLength(1)
  })

  it('a model the harness refuses goes back to the session’s, with its words', async () => {
    const message = 'model fake-small was refused by the harness: Usage credits are required'
    const app = await start(
      '/projects/p1/threads/t1',
      answers({
        model: () => Response.json({ code: 'SETTING_NOT_OFFERED', message }, { status: 422 }),
      }),
    )
    await until(ready(app))
    await choose(app, 'fake-large', 'fake-small')
    await until(() => app.text().includes(message))
    expect(app.button('fake-large')).toBeDefined()
    expect(app.button('fake-small')).toBeUndefined()
    expect(app.button('high')?.disabled).toBe(false)
  })

  it('the mode menu says plainly what Accept edits allows', async () => {
    const app = await start('/projects/p1/threads/t1', answers())
    await until(ready(app))
    act(() => app.button('Accept edits')?.click())
    await until(() => menuItem('Accept edits') !== undefined)
    expect(menuItem('Accept edits')?.textContent).toContain(
      'Claude Code edits, creates and deletes files in the project folder without asking. Other commands are refused.',
    )
    expect(menuItem('Auto')?.textContent).toContain(
      'Claude Code decides on its own; nothing is asked.',
    )
  })

  it('a fresh fork shows the CLI locked before any turn', async () => {
    const app = await start('/projects/p1/threads/t1', {
      ...answers({ operations: [] }),
      'GET /api/projects/p1/threads': [{ ...threadFixture, forked_from_thread: 't0' }],
    })
    await until(
      () => app.container.querySelector('[aria-label="CLI locked for this conversation"]') !== null,
    )
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
