// @vitest-environment happy-dom
//
// Exercise image intake, upload replay and ordered delivery over the fake daemon.

import { act } from 'react'
import { expect, it, vi } from 'vitest'
import { fakeChoices, userEntry, runningOperation, threadFixture } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { start, ready, textarea, turnStarts } from '@/test/composer'
import { type TestApp, choose, typeInto, until } from '@/test/test-app'

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
