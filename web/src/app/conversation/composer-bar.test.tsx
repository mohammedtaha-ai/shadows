// @vitest-environment happy-dom
//
// The conversation's CLI picker and composer bar over a faked daemon: the
// session opens with words, never a bare spinner; every menu is built from
// the session's answer; Send carries the chosen settings and a command id
// that a retry of the same send reuses.

import { act } from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { completedOperation, fakeChoices, userEntry, runningOperation, projectFixture, threadFixture } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, choose, menuItem, startApp, typeInto, until } from '../test-app'

let open: TestApp | null = null
afterEach(() => {
  open?.unmount()
  open = null
  vi.restoreAllMocks()
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


function imageFile(name = 'screen.png') {
  return new File(['fixture bytes'], name, { type: 'image/png' })
}

function deliverFiles(app: TestApp, via: 'paste' | 'drop' | 'picker', files: File[]) {
  const target = via === 'picker' ? app.container.querySelector<HTMLInputElement>('input[type="file"]')! : textarea(app)
  const event = new Event(via === 'picker' ? 'change' : via, { bubbles: true, cancelable: true })
  if (via === 'picker') Object.defineProperty(target, 'files', { value: files, configurable: true })
  else Object.defineProperty(event, via === 'paste' ? 'clipboardData' : 'dataTransfer', { value: { files, types: ['Files'] } })
  act(() => target.dispatchEvent(event))
  return event
}

/** The shared app fixture records JSON requests; intercept multipart at the fake daemon boundary. */
function imageUploads(app: TestApp, answer: (request: Request) => Promise<Response> = async request => {
  const body = await request.formData()
  const file = body.get('file') as File
  return Response.json({ id: file.name, name: file.name, media_type: file.type, byte_len: file.size })
}) {
  vi.spyOn(URL, 'createObjectURL').mockReturnValue('blob:fixture-preview')
  vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => {})
  const fallback = globalThis.fetch
  vi.stubGlobal('fetch', async (request: Request) => {
    const key = `${request.method} ${new URL(request.url).pathname}`
    if (key === 'POST /api/threads/t1/attachments') {
      app.calls.push(key)
      return answer(request)
    }
    return fallback(request)
  })
}

it.each(['paste', 'drop', 'picker', 'shortcut'] as const)('%s uploads immediately with a removable thumbnail', async via => {
  const app = await start('/projects/p1/threads/t1', answers())
  await until(ready(app))
  let uploads = 0
  imageUploads(app, async request => {
    uploads++
    expect(request.headers.get('content-type')).toContain('multipart/form-data; boundary=')
    const body = await request.formData()
    expect(body.get('command_id')).toEqual(expect.any(String))
    const file = body.get('file') as File
    expect(file.name).toBe('screen.png')
    expect(await file.text()).toBe('fixture bytes')
    return Response.json({ id: 'image-1', name: file.name, media_type: 'image/png', byte_len: file.size })
  })
  if (via === 'picker' || via === 'shortcut') {
    const click = vi.spyOn(app.container.querySelector<HTMLInputElement>('input[type="file"]')!, 'click')
    if (via === 'picker') await choose(app, 'Add', 'Add photos')
    else act(() => textarea(app).dispatchEvent(new KeyboardEvent('keydown', { key: 'u', ctrlKey: true, bubbles: true, cancelable: true })))
    expect(click).toHaveBeenCalledOnce()
  }
  deliverFiles(app, via === 'shortcut' ? 'picker' : via, [imageFile()])
  await until(() => app.text().includes('Uploaded'))
  expect(uploads).toBe(1)
  expect(app.container.querySelector('img[alt="screen.png"]')?.getAttribute('src')).toContain('/api/threads/t1/attachments/image-1')
  act(() => app.button('Remove screen.png')!.click())
  expect(app.container.querySelector('img[alt="screen.png"]')).toBeNull()
  expect(turnStarts(app)).toBe(0)
  expect(URL.revokeObjectURL).toHaveBeenCalled()
})

it('ordinary text paste keeps the browser paste behavior', async () => {
  const app = await start('/projects/p1/threads/t1', answers())
  await until(ready(app))
  typeInto(textarea(app), 'Existing text')
  const event = new Event('paste', { bubbles: true, cancelable: true })
  Object.defineProperty(event, 'clipboardData', { value: { files: [], getData: () => 'pasted text' } })
  act(() => textarea(app).dispatchEvent(event))
  expect(event.defaultPrevented).toBe(false)
  expect(textarea(app).value).toBe('Existing text')
  expect(app.calls.some(call => call.endsWith('/attachments'))).toBe(false)
})

it('non-image clipboard files keep browser paste behavior and mixed paste uploads only images in order', async () => {
  const app = await start('/projects/p1/threads/t1', answers())
  await until(ready(app))
  imageUploads(app)
  const text = new File(['notes'], 'notes.txt', { type: 'text/plain' })
  expect(deliverFiles(app, 'paste', [text]).defaultPrevented).toBe(false)
  expect(app.container.querySelector('[aria-label="Images"]')).toBeNull()
  expect(deliverFiles(app, 'paste', [imageFile('first.png'), text, imageFile('second.png')]).defaultPrevented).toBe(true)
  await until(() => app.container.querySelectorAll('img[src*="/attachments/"]').length === 2)
  act(() => app.button('Send')!.click())
  await until(() => turnStarts(app) === 1)
  expect(app.bodies.at(-1)).toMatchObject({ attachments: ['first.png', 'second.png'] })
  expect(app.calls.filter(call => call.endsWith('/attachments'))).toHaveLength(2)
})

it('switching conversations revokes local previews and ignores the old upload answer', async () => {
  const app = await start('/projects/p1/threads/t1', {
    ...answers(),
    'GET /api/projects/p1/threads': [threadFixture, { ...threadFixture, id: 't2' }],
    'POST /api/threads/t2/session': fakeChoices,
    'GET /api/threads/t2/entries': [],
    'GET /api/threads/t2/operations': [],
    'GET /api/threads/t2/queue': [],
    'POST /api/threads/t2/turns': () => Response.json({ operation_id: 'op2' }, { status: 202 }),
  })
  await until(ready(app))
  let finish!: () => void
  imageUploads(app, async () => new Promise<Response>(resolve => { finish = () => resolve(Response.json({ id: 'old-image', media_type: 'image/png', byte_len: 1 })) }))
  deliverFiles(app, 'paste', [imageFile()])
  await until(() => finish !== undefined)
  const { router } = await import('@/router')
  await act(() => router.navigate({ to: '/projects/$projectId/threads/$threadId', params: { projectId: 'p1', threadId: 't2' } }))
  await until(() => app.calls.includes('GET /api/threads/t2/operations') && app.button('fake-large') !== undefined)
  expect(URL.revokeObjectURL).toHaveBeenCalledWith('blob:fixture-preview')
  act(() => finish())
  typeInto(textarea(app), 'Another conversation')
  act(() => app.button('Send')!.click())
  await until(() => app.calls.includes('POST /api/threads/t2/turns'))
  expect(app.bodies.at(-1)).toMatchObject({ prompt: 'Another conversation', attachments: [] })
  expect(app.container.querySelector('[aria-label="Images"]')).toBeNull()
})

it('Send waits for uploads and sends image-only ids in selection order', async () => {
  const app = await start('/projects/p1/threads/t1', answers())
  await until(ready(app))
  const release = new Map<string, () => void>()
  imageUploads(app, async request => {
    const file = (await request.formData()).get('file') as File
    return new Promise<Response>(resolve => release.set(file.name, () => resolve(Response.json({ id: `id-${file.name}`, name: file.name, media_type: file.type, byte_len: file.size }))))
  })
  deliverFiles(app, 'paste', [imageFile('first.png'), imageFile('second.png')])
  await until(() => release.size === 2)
  act(() => app.button('Send')!.click())
  expect(turnStarts(app)).toBe(0)
  act(() => release.get('second.png')!())
  await until(() => app.text().includes('Uploaded'))
  expect(turnStarts(app)).toBe(0)
  act(() => release.get('first.png')!())
  await until(() => turnStarts(app) === 1)
  expect(app.bodies.at(-1)).toMatchObject({ prompt: '', attachments: ['id-first.png', 'id-second.png'] })
  await until(() => app.container.querySelector('[aria-label="Images"]') === null)
})

it('a failed upload keeps text and images with Core reason and retries the same upload command', async () => {
  const app = await start('/projects/p1/threads/t1', answers())
  await until(ready(app))
  const commands: FormDataEntryValue[] = []
  imageUploads(app, async request => {
    commands.push((await request.formData()).get('command_id')!)
    return commands.length === 1 ? Response.json({ code: 'INVALID_COMMAND', message: 'Core refuses these image bytes' }, { status: 422 })
      : Response.json({ id: 'ok', media_type: 'image/png', byte_len: 1 })
  })
  typeInto(textarea(app), 'Keep my text')
  deliverFiles(app, 'drop', [imageFile()])
  await until(() => app.text().includes('Core refuses these image bytes'))
  expect(textarea(app).value).toBe('Keep my text')
  expect(app.button('Remove screen.png')).toBeDefined()
  expect(app.button('Send')?.disabled).toBe(true)
  act(() => app.button('Retry screen.png')!.click())
  await until(() => app.text().includes('Uploaded'))
  expect(commands).toEqual([commands[0], commands[0]])
  expect(turnStarts(app)).toBe(0)
})

it('a refused send keeps text and images and replays the same ordered command', async () => {
  const requests: unknown[] = []
  const app = await start('/projects/p1/threads/t1', answers({ start: async (request: Request) => {
    requests.push(await request.json())
    return requests.length === 1 ? Response.json({ code: 'INVALID_COMMAND', message: 'Core refuses images for this model' }, { status: 422 })
      : Response.json({ operation_id: 'op1' }, { status: 202 })
  } }))
  await until(ready(app))
  imageUploads(app)
  typeInto(textarea(app), 'Keep both')
  deliverFiles(app, 'picker', [imageFile()])
  await until(() => app.text().includes('Uploaded'))
  act(() => app.button('Send')!.click())
  await until(() => app.text().includes('Core refuses images for this model'))
  expect(textarea(app).value).toBe('Keep both')
  expect(app.button('Remove screen.png')).toBeDefined()
  act(() => app.button('Send')!.click())
  await until(() => requests.length === 2)
  expect(requests[1]).toEqual(requests[0])
  expect(app.calls.filter(call => call.endsWith('/attachments'))).toHaveLength(1)
})

it('Core capability disables image paste, drop and Ctrl+U with its reason', async () => {
  const reason = 'The fixture harness cannot read images'
  const app = await start('/projects/p1/threads/t1', answers({ session: { ...fakeChoices, capabilities: { image: false, image_reason: reason } } }))
  await until(ready(app))
  imageUploads(app)
  const click = vi.spyOn(app.container.querySelector<HTMLInputElement>('input[type="file"]')!, 'click')
  typeInto(textarea(app), 'Keep text')
  expect(deliverFiles(app, 'paste', [imageFile()]).defaultPrevented).toBe(true)
  deliverFiles(app, 'drop', [imageFile()])
  act(() => textarea(app).dispatchEvent(new KeyboardEvent('keydown', { key: 'u', ctrlKey: true, bubbles: true, cancelable: true })))
  expect(app.text()).toContain(reason)
  expect(click).not.toHaveBeenCalled()
  expect(app.calls.some(call => call.endsWith('/attachments'))).toBe(false)
  expect(textarea(app).value).toBe('Keep text')
  expect(app.container.querySelector('[aria-label="Images"]')).toBeNull()
})

it('removing an uploading image prevents its late answer from being sent', async () => {
  const app = await start('/projects/p1/threads/t1', answers())
  await until(ready(app))
  let finish!: () => void
  imageUploads(app, async () => new Promise<Response>(resolve => { finish = () => resolve(Response.json({ id: 'removed', media_type: 'image/png', byte_len: 1 })) }))
  deliverFiles(app, 'paste', [imageFile()])
  await until(() => finish !== undefined)
  act(() => app.button('Remove screen.png')!.click())
  act(() => finish())
  typeInto(textarea(app), 'Text only')
  act(() => app.button('Send')!.click())
  await until(() => turnStarts(app) === 1)
  expect(app.bodies.at(-1)).toMatchObject({ prompt: 'Text only', attachments: [] })
})


it('the transcript renders ordered message images from the thread byte route', async () => {
  const entry = { ...userEntry('images', ''), attachments: [
    { id: 'first image', media_type: 'image/png', name: 'First screenshot' },
    { id: 'second', media_type: 'image/jpeg', name: null },
  ] }
  const app = await start('/projects/p1/threads/t1', answers({ entries: [entry] }))
  await until(() => app.container.querySelector('img[alt="First screenshot"]') !== null)
  const images = [...app.container.querySelectorAll('img')]
  expect(images.map(image => image.alt)).toEqual(['First screenshot', 'Attached image'])
  expect(images.map(image => image.getAttribute('src'))).toEqual([
    expect.stringContaining('/api/threads/t1/attachments/first%20image'),
    expect.stringContaining('/api/threads/t1/attachments/second'),
  ])
})

it('queue preserves ordered image ids and Send now uses the saved queued images', async () => {
  let queued: Record<string, unknown> | null = null
  const app = await start('/projects/p1/threads/t1', {
    ...answers({ operations: [runningOperation()] }),
    'GET /api/threads/t1/queue': () => Response.json(queued === null ? [] : [{ ...queued, id: 'q1', thread_id: 't1', position: 1, created_at: 'now' }]),
    'POST /api/threads/t1/queue': async (request: Request) => {
      queued = await request.json() as Record<string, unknown>
      return Response.json({ status: 'waiting', message: { ...queued, id: 'q1' } }, { status: 202 })
    },
    'POST /api/threads/t1/queue/q1/send-now': () => Response.json({ status: 'steered', entry_id: 'e1' }, { status: 202 }),
  })
  await until(ready(app))
  await until(() => app.button('Stop') !== undefined)
  imageUploads(app)
  deliverFiles(app, 'paste', [imageFile('first.png'), imageFile('second.png')])
  await until(() => app.text().includes('Uploaded'))
  act(() => textarea(app).dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true })))
  await until(() => app.button('Send now') !== undefined)
  expect(queued).toMatchObject({ prompt: '', attachments: ['first.png', 'second.png'] })
  expect(app.container.querySelectorAll('img[alt="Queued image"]')).toHaveLength(2)
  act(() => app.button('Send now')!.click())
  await until(() => app.calls.includes('POST /api/threads/t1/queue/q1/send-now'))
  expect(app.bodies.at(-1)).toMatchObject({ command_id: expect.any(String) })
  expect(app.calls.filter(call => call.endsWith('/attachments'))).toHaveLength(2)
})


it('command conflict retains the message and makes the next explicit send a new command', async () => {
  const requests: Record<string, unknown>[] = []
  const app = await start('/projects/p1/threads/t1', answers({ start: async (request: Request) => {
    requests.push(await request.json() as Record<string, unknown>)
    return requests.length === 1 ? Response.json({ code: 'COMMAND_CONFLICT', message: 'Core says this command belongs to another request' }, { status: 409 })
      : Response.json({ operation_id: 'op1' }, { status: 202 })
  } }))
  await until(ready(app))
  imageUploads(app)
  typeInto(textarea(app), 'Keep on conflict')
  deliverFiles(app, 'paste', [imageFile()])
  await until(() => app.text().includes('Uploaded'))
  act(() => app.button('Send')!.click())
  await until(() => app.text().includes('Core says this command belongs to another request'))
  expect(textarea(app).value).toBe('Keep on conflict')
  expect(app.button('Remove screen.png')).toBeDefined()
  expect(requests).toHaveLength(1)
  act(() => app.button('Send')!.click())
  await until(() => requests.length === 2)
  expect(requests[1]?.command_id).not.toBe(requests[0]?.command_id)
  expect(requests[1]?.attachments).toEqual(requests[0]?.attachments)
})

it('a refused queue keeps text and images and its unchanged retry command', async () => {
  const requests: unknown[] = []
  const app = await start('/projects/p1/threads/t1', {
    ...answers({ operations: [runningOperation()] }),
    'GET /api/threads/t1/queue': [],
    'POST /api/threads/t1/queue': async (request: Request) => {
      requests.push(await request.json())
      return Response.json({ code: 'INVALID_COMMAND', message: 'Core refused the queued images' }, { status: 422 })
    },
  })
  await until(ready(app))
  await until(() => app.button('Stop') !== undefined)
  imageUploads(app)
  typeInto(textarea(app), 'Keep queued text')
  deliverFiles(app, 'paste', [imageFile()])
  await until(() => app.text().includes('Uploaded'))
  const enter = () => act(() => textarea(app).dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true })))
  enter()
  await until(() => app.text().includes('Core refused the queued images'))
  expect(textarea(app).value).toBe('Keep queued text')
  expect(app.button('Remove screen.png')).toBeDefined()
  enter()
  await until(() => requests.length === 2)
  expect(requests[1]).toEqual(requests[0])
})

it('a queue waiting for an upload keeps the full message locked until Core refuses it', async () => {
  const app = await start('/projects/p1/threads/t1', {
    ...answers({ operations: [runningOperation()] }),
    'GET /api/threads/t1/queue': [],
    'POST /api/threads/t1/queue': () => Response.json({ code: 'INVALID_COMMAND', message: 'Queue refused' }, { status: 422 }),
  })
  await until(ready(app))
  await until(() => app.button('Stop') !== undefined)
  let finish!: () => void
  imageUploads(app, async () => new Promise<Response>(resolve => { finish = () => resolve(Response.json({ id: 'image', media_type: 'image/png', byte_len: 1 })) }))
  typeInto(textarea(app), 'Keep all of this')
  deliverFiles(app, 'paste', [imageFile()])
  await until(() => finish !== undefined)
  act(() => textarea(app).dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true })))
  await until(() => textarea(app).disabled)
  expect(textarea(app).value).toBe('Keep all of this')
  expect(app.button('Remove screen.png')?.disabled).toBe(true)
  act(() => finish())
  await until(() => app.text().includes('Queue refused'))
  expect(textarea(app).disabled).toBe(false)
  expect(textarea(app).value).toBe('Keep all of this')
  expect(app.button('Remove screen.png')?.disabled).toBe(false)
})


it('an upload command conflict needs an explicit retry with a fresh command', async () => {
  const app = await start('/projects/p1/threads/t1', answers())
  await until(ready(app))
  const commands: FormDataEntryValue[] = []
  imageUploads(app, async request => {
    commands.push((await request.formData()).get('command_id')!)
    return commands.length === 1 ? Response.json({ code: 'COMMAND_CONFLICT', message: 'Core reports an upload command conflict' }, { status: 409 })
      : Response.json({ id: 'ok', media_type: 'image/png', byte_len: 1 })
  })
  deliverFiles(app, 'paste', [imageFile()])
  await until(() => app.text().includes('Core reports an upload command conflict'))
  expect(commands).toHaveLength(1)
  act(() => app.button('Retry screen.png')!.click())
  await until(() => app.text().includes('Uploaded'))
  expect(commands).toHaveLength(2)
  expect(commands[1]).not.toBe(commands[0])
})
