// @vitest-environment happy-dom
//
// The conversation's CLI picker and composer bar over a faked daemon: the
// session opens with words, never a bare spinner; every menu is built from
// the session's answer; Send carries the chosen settings and a command id
// that a retry of the same send reuses.

import { act } from 'react'
import { afterEach, describe, expect, it } from 'vitest'
import { choice, completedOperation, fakeChoices, projectFixture, threadFixture } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, choose, menuItem, startApp, typeInto, until } from '../test-app'

let open: TestApp | null = null
afterEach(() => {
  open?.unmount()
  open = null
})

async function start(url: string, table: ReturnType<typeof answers>): Promise<TestApp> {
  open = await startApp(url, {
    'PUT /api/threads/t1/session/mode': async (r: Request) => {
      const { mode } = await r.json() as { mode: string }
      return Response.json({ ...fakeChoices, current: { ...fakeChoices.current, mode } })
    },
    ...table,
  })
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

it('Add shows disabled photos with its reason and omits project Plugins, folders and connectors', async () => {
  const app = await start('/projects/p1/threads/t1', answers())
  await until(ready(app))
  act(() => app.button('Add')!.click())
  await until(() => menuItem('Add photos') !== undefined)
  expect(menuItem('Add photos')?.getAttribute('aria-disabled')).toBe('true')
  expect(menuItem('Add photos')?.textContent).toContain('Comes with F-25')
  expect(menuItem('Slash commands')).toBeDefined()
  expect(menuItem('Plugins')).toBeUndefined()
  expect(menuItem('Add folder')).toBeUndefined()
  expect(menuItem('Connectors')).toBeUndefined()
  act(() => menuItem('Add photos')!.click())
  expect(document.querySelector('input[type="file"]')).toBeNull()
  expect(turnStarts(app)).toBe(0)
})

it('Add opens with Enter, arrows reach Slash commands and Escape returns to Message', async () => {
  const app = await start('/projects/p1/threads/t1', answers())
  await until(ready(app))
  const trigger = app.button('Add')!
  const key = (element: Element, value: string) => act(() => {
    const down = new KeyboardEvent('keydown', { key: value, bubbles: true, cancelable: true })
    element.dispatchEvent(down)
    // happy-dom dispatches keys without the browser's native button activation.
    if (value === 'Enter' && element instanceof HTMLButtonElement && !down.defaultPrevented) {
      element.click()
    }
    element.dispatchEvent(new KeyboardEvent('keyup', { key: value, bubbles: true, cancelable: true }))
  })
  act(() => trigger.focus())
  key(trigger, 'Enter')
  await until(() => menuItem('Slash commands') !== undefined)
  key(document.activeElement!, 'ArrowDown')
  // Disabled menu items stay focusable so their reason can be read.
  await until(() => document.activeElement === menuItem('Add photos'))
  key(document.activeElement!, 'ArrowDown')
  await until(() => document.activeElement === menuItem('Slash commands'))
  key(document.activeElement!, 'Escape')
  await until(() => document.querySelector('[role="menu"]') === null)
  await until(() => document.activeElement === textarea(app))
  expect(textarea(app).value).toBe('')
  expect(turnStarts(app)).toBe(0)
})

it.each(['Close', 'Escape'])('Plugins opens this conversations AYLUL agent settings and closing with %s returns to Message', async (closeWith) => {
  const thread = { ...threadFixture, harness: 'aylul', profile: 'work' }
  const agent = { slug: 'work', name: 'Work', provider: 'fixture-provider', default_model: 'fake-large', deleted: false, created_at: 'now', updated_at: 'now' }
  const app = await start('/projects/p1/threads/t1', {
    ...answers(),
    'GET /api/projects': [{ ...projectFixture, assistant: true }],
    'GET /api/projects/p1/threads': [thread],
    'GET /api/threads/t1': thread,
    'GET /api/aylul/agents': { project_id: 'p1', agents: [{ ...agent, slug: 'other', name: 'Other' }, agent] },
    'GET /api/aylul/providers': [{ slug: 'fixture-provider', name: 'Fixture provider', base_url: 'https://example.com', models: [{ id: 'fake-large', context_window: 200000 }], key_set: true, created_at: 'now', updated_at: 'now' }],
    'GET /api/aylul/agents/work/env': [],
  })
  await until(() => app.text().includes('Work') && app.button('Add') !== undefined)
  const box = textarea(app)
  typeInto(box, 'Keep this message')
  await choose(app, 'Add', 'Plugins')
  await until(() => document.querySelector('[role="group"][aria-label="Plugins & Skills"]') !== null)
  expect(new URLSearchParams(window.location.search).get('settings')).toBe('agents')
  expect(new URLSearchParams(window.location.search).get('agent')).toBe('work')
  expect(document.querySelector<HTMLInputElement>('input[aria-label="Agent name"]')?.value).toBe('Work')
  expect(app.calls).toContain('GET /api/aylul/agents/work/extensions')
  expect(app.calls).not.toContain('GET /api/aylul/agents/other/extensions')
  const dialog = document.querySelector<HTMLElement>('[role="dialog"]')!
  let finishExit!: () => void
  const finished = new Promise<void>((resolve) => { finishExit = resolve })
  // Model an exit animation lasting beyond the composer's first animation frame.
  Object.defineProperty(dialog, 'getAnimations', { value: () => [{ finished }] })
  act(() => {
    if (closeWith === 'Close') app.button('Close')!.click()
    else dialog.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }))
  })
  await until(() => !new URLSearchParams(window.location.search).has('settings'))
  await act(async () => { await new Promise<void>((resolve) => requestAnimationFrame(() => resolve())) })
  expect(document.querySelector('[role="dialog"]')).toBe(dialog)
  act(() => finishExit())
  await until(() => document.querySelector('[role="dialog"]') === null)
  await until(() => document.activeElement === box)
  expect(box.value).toBe('Keep this message')
})

describe('the composer bar', () => {
  it('choosing Auto saves at once and shows only Core’s answer', async () => {
    let release!: () => void
    const app = await start('/projects/p1/threads/t1', {
      ...answers(),
      'PUT /api/threads/t1/session/mode': () => new Promise<Response>(resolve => {
        release = () => resolve(Response.json(fakeChoices))
      }),
    })
    await until(ready(app))
    await choose(app, 'Mode', 'Auto')
    await until(() => app.bodies.length === 1)
    expect(app.bodies).toEqual([{ command_id: expect.any(String), mode: 'auto' }])
    expect(app.button('Mode')?.textContent).toContain('Accept edits')
    expect(app.button('Mode')?.disabled).toBe(true)
    expect(app.button('Send')?.disabled).toBe(true)
    expect(turnStarts(app)).toBe(0)
    act(() => release())
    await until(() => app.button('Mode')?.disabled === false)
    expect(app.button('Mode')?.textContent).toContain('Accept edits')
    expect(app.button('Auto')).toBeUndefined()
  })

  it('Auto survives navigating away and back and a fresh QueryClient remount', async () => {
    let core = fakeChoices
    const routes = {
      ...answers({ session: () => Response.json(core) }),
      'PUT /api/threads/t1/session/mode': async (r: Request) => {
        const { mode } = await r.json() as { mode: string }
        core = { ...core, current: { ...core.current, mode } }
        return Response.json(core)
      },
    }
    const app = await start('/projects/p1/threads/t1', routes)
    await until(ready(app))
    await choose(app, 'Mode', 'Auto')
    await until(() => app.button('Auto')?.disabled === false)
    const { router } = await import('@/router')
    await act(() => router.navigate({ to: '/projects/$projectId/settings', params: { projectId: 'p1' } }))
    await until(() => app.container.querySelector('textarea') === null)
    await act(() => router.navigate({ to: '/projects/$projectId/threads/$threadId', params: { projectId: 'p1', threadId: 't1' } }))
    await until(() => app.button('Auto') !== undefined)
    const previousClient = app.queryClient
    app.unmount()
    open = null
    const remounted = await start('/projects/p1/threads/t1', routes)
    await until(() => remounted.button('Auto') !== undefined)
    expect(remounted.queryClient).not.toBe(previousClient)
    expect(remounted.calls).toContain('POST /api/threads/t1/session')
    expect(remounted.calls).not.toContain('PUT /api/threads/t1/session/mode')
  })

  it.each([
    [403, 'MODE_NOT_ALLOWED', 'This project does not allow Auto'],
    [409, 'COMMAND_CONFLICT', 'This command was already used with another mode'],
    [422, 'SETTING_NOT_OFFERED', 'The harness has no such mode'],
  ])('a mode refusal %s keeps Core’s mode and shows its message', async (status, code, message) => {
    const app = await start('/projects/p1/threads/t1', {
      ...answers(),
      'PUT /api/threads/t1/session/mode': () => Response.json({ code, message }, { status }),
    })
    await until(ready(app))
    typeInto(textarea(app), 'Keep this prompt')
    await choose(app, 'Mode', 'Auto')
    await until(() => app.text().includes(message))
    expect(app.button('Mode')?.textContent).toContain('Accept edits')
    expect(textarea(app).value).toBe('Keep this prompt')
    await choose(app, 'Mode', 'Auto')
    await until(() => app.bodies.length === 2 && app.button('Mode')?.disabled === false)
    const [first, second] = app.bodies as { command_id: string }[]
    expect(second.command_id).not.toBe(first.command_id)
    expect(turnStarts(app)).toBe(0)
  })

  it('a lost mode answer keeps Core’s mode and retries with the same command id', async () => {
    let attempts = 0
    const app = await start('/projects/p1/threads/t1', {
      ...answers(),
      'PUT /api/threads/t1/session/mode': () => {
        if (++attempts === 1) throw new TypeError('connection lost')
        return Response.json({ ...fakeChoices, current: { ...fakeChoices.current, mode: 'auto' } })
      },
    })
    await until(ready(app))
    await choose(app, 'Mode', 'Auto')
    await until(() => app.text().includes('The daemon is not reachable.'))
    expect(app.button('Mode')?.textContent).toContain('Accept edits')
    expect(app.calls.filter(c => c === 'PUT /api/threads/t1/session/mode')).toHaveLength(1)
    await choose(app, 'Mode', 'Auto')
    await until(() => app.button('Auto')?.disabled === false)
    expect(app.bodies).toHaveLength(2)
    expect(app.bodies[1]).toEqual(app.bodies[0])
    expect(app.text()).not.toContain('The daemon is not reachable.')
  })

  it('a mode save cancels an older session read before caching Core’s answer', async () => {
    let release!: () => void
    let reads = 0
    const app = await start('/projects/p1/threads/t1', {
      ...answers({ session: () => ++reads === 1 ? Response.json(fakeChoices) : new Promise<Response>(resolve => {
        release = () => resolve(Response.json(fakeChoices))
      }) }),
    })
    await until(ready(app))
    const key = ['threads', 't1', 'session'] as const
    let refetch!: Promise<void>
    act(() => { refetch = app.queryClient.refetchQueries({ queryKey: key, exact: true }) })
    await until(() => reads === 2)
    await choose(app, 'Mode', 'Auto')
    await until(() => app.button('Auto')?.disabled === false)
    await act(async () => { release(); await refetch })
    expect(app.queryClient.getQueryData(key)).toEqual({ ...fakeChoices, current: { ...fakeChoices.current, mode: 'auto' } })
    expect(app.button('Mode')?.textContent).toContain('Auto')
  })

  it('switching conversations isolates their modes and a late save answer', async () => {
    let release!: () => void
    const second = { ...threadFixture, id: 't2', title: 'Second conversation' }
    const app = await start('/projects/p1/threads/t1', {
      ...answers(),
      'GET /api/projects/p1/threads': [threadFixture, second],
      'POST /api/threads/t2/session': fakeChoices,
      'GET /api/threads/t2/entries': [],
      'GET /api/threads/t2/operations': [],
      'GET /api/threads/t2/queue': [],
      'PUT /api/threads/t1/session/mode': () => new Promise<Response>(resolve => {
        release = () => resolve(Response.json({ ...fakeChoices, current: { ...fakeChoices.current, mode: 'auto' } }))
      }),
    })
    await until(ready(app))
    await choose(app, 'Mode', 'Auto')
    await until(() => app.button('Mode')?.disabled === true)
    const { router } = await import('@/router')
    await act(() => router.navigate({ to: '/projects/$projectId/threads/$threadId', params: { projectId: 'p1', threadId: 't2' } }))
    await until(() => app.button('Mode')?.disabled === false)
    expect(app.button('Mode')?.textContent).toContain('Accept edits')
    act(() => release())
    await until(() => app.queryClient.getQueryData<{ current: { mode: string } }>(['threads', 't1', 'session'])?.current.mode === 'auto')
    expect(app.button('Mode')?.textContent).toContain('Accept edits')
    await act(() => router.navigate({ to: '/projects/$projectId/threads/$threadId', params: { projectId: 'p1', threadId: 't1' } }))
    await until(() => app.button('Auto')?.disabled === false)
    expect(app.calls).not.toContain('PUT /api/threads/t2/session/mode')
  })

  it('every mode choice uses a fresh command and options frames remain authoritative', async () => {
    const app = await start('/projects/p1/threads/t1', answers())
    await until(ready(app))
    await choose(app, 'Mode', 'Auto')
    await until(() => app.button('Auto')?.disabled === false)
    act(() => app.pushFrame('options', { thread_id: 't1', choices: fakeChoices }))
    await until(() => app.button('Accept edits') !== undefined)
    await choose(app, 'Mode', 'Auto')
    await until(() => app.button('Auto')?.disabled === false)
    const [first, second] = app.bodies as { command_id: string }[]
    expect(second.command_id).not.toBe(first.command_id)
    expect(app.bodies).toHaveLength(2)
    expect(turnStarts(app)).toBe(0)
  })
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
    await until(() => app.button('Auto')?.disabled === false)
    await until(() => menuItem('Auto') === undefined)
    expect(app.button('Auto')).toBeDefined()
  })

  it('sends the chosen model, mode and effort', async () => {
    const app = await start('/projects/p1/threads/t1', answers())
    await until(ready(app))
    await choose(app, 'Accept edits', 'Auto')
    await until(() => app.button('Auto')?.disabled === false)
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
    await until(() => app.button('Auto')?.disabled === false)
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
    // The effort is the one the session reports for fake-small, which offers
    // no max; it is not set back to the person's previous one (§12.4).
    await until(() => app.button('high')?.disabled === false)
    act(() => app.button('high')?.click())
    await until(() => menuItem('low') !== undefined)
    expect(menuItem('max')).toBeUndefined()
    expect(app.button('Send')?.disabled).toBe(false)
    expect(app.bodies).toHaveLength(2)
    expect(turnStarts(app)).toBe(0)
  })

  it('a picked model ends at the effort its answer reports, not an earlier report', async () => {
    // Measured with the real adapter: a model change reports the effort it
    // carried over, then the remembered one the daemon sets (§12.4).
    let release!: () => void
    const small = (effort: string) => ({
      ...fakeChoices,
      efforts: [choice('low'), choice('high')],
      current: { ...fakeChoices.current, model: 'fake-small', effort },
    })
    const app = await start(
      '/projects/p1/threads/t1',
      answers({
        model: () =>
          new Promise<Response>((r) => {
            release = () => r(Response.json(small('high')))
          }),
      }),
    )
    await until(ready(app))
    await choose(app, 'fake-large', 'fake-small')
    await until(() => app.bodies.length === 1)
    act(() => app.pushFrame('options', { thread_id: 't1', choices: small('low') }))
    await until(() => app.button('low') !== undefined)
    act(() => release())
    await until(() => app.button('high')?.disabled === false)
    // The remembered effort stands: nothing sets the carried one back.
    expect(app.bodies).toEqual([{ model: 'fake-small' }])
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
    await until(() => app.button('Auto')?.disabled === false)
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
