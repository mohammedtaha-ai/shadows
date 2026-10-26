// @vitest-environment happy-dom
//
// The conversation's CLI picker and composer bar over a faked daemon: the
// session opens with words, never a bare spinner; every menu is built from
// the session's answer; Send carries the chosen settings and a command id
// that a retry of the same send reuses.

import { act } from 'react'
import { describe, expect, it } from 'vitest'
import { completedOperation, fakeChoices, projectFixture, threadFixture } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { composer, start, ready, textarea, turnStarts } from '@/test/composer'
import { choose, menuItem, typeInto, until } from '../test-app'

it('Add shows Core image refusal and omits project Plugins, folders and connectors', async () => {
  const app = await start('/projects/p1/threads/t1', answers({ session: { ...fakeChoices, capabilities: { image: false, image_reason: 'Images refused by fixture harness' } } }))
  await until(ready(app))
  act(() => app.button('Add')!.click())
  await until(() => menuItem('Add photos') !== undefined)
  expect(menuItem('Add photos')?.getAttribute('aria-disabled')).toBe('true')
  expect(menuItem('Add photos')?.textContent).toContain('Images refused by fixture harness')
  expect(menuItem('Slash commands')).toBeDefined()
  expect(menuItem('Plugins')).toBeUndefined()
  expect(menuItem('Add folder')).toBeUndefined()
  expect(menuItem('Connectors')).toBeUndefined()
  act(() => menuItem('Add photos')!.click())
  expect(document.querySelector<HTMLInputElement>('input[type="file"]')?.disabled).toBe(true)
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
    composer.open = null

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
