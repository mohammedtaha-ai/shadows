// @vitest-environment happy-dom
//

import { act } from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { fakeChoices, planFixture, planListing, threadFixture } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, choose, startApp, typeInto, until } from '@/test/test-app'

let open: TestApp | null = null
afterEach(() => {
  open?.unmount()
  open = null
  vi.restoreAllMocks()
})

describe('the draft', () => {
  it("a draft starts at Core's initial mode without a session mode write", async () => {
    const app = open = await startApp('/projects/p1/new', answers())
    await until(() => app.button('Mode') !== undefined)
    expect(app.button('Mode')?.textContent).toContain('Accept edits')
    await choose(app, 'Mode', 'Auto')
    expect(app.button('Mode')?.textContent).toContain('Auto')
    expect(app.calls.some(c => c.includes('/session'))).toBe(false)
  })
  it('offers no mode and shows the error when the harness list cannot be read', async () => {
    const routes = answers()
    routes['GET /api/harnesses'] = () => Response.json({ code: 'STORAGE_UNAVAILABLE', message: 'Harness list unavailable' }, { status: 500 })
    const app = open = await startApp('/projects/p1/new', routes)
    await until(() => app.text().includes('Harness list unavailable'))
    expect(app.button('Mode')).toBeUndefined()
    expect(app.button('Accept edits')).toBeUndefined()
  })
  it('shows the project stage before the first message', async () => {
    const app = open = await startApp('/projects/p1/new', answers())
    await until(() => app.text().includes('Stage: idea'))
    expect(app.container.querySelector('main header')?.textContent).toContain('Stage: idea')
  })
  it('carries the selected plan and command into a failed first-send retry', async () => {
    const requests: Record<string, unknown>[] = []
    const plan = planFixture()
    const routes = answers({ plans: [planListing(plan)], start: async (request: Request) => {
      requests.push(await request.json() as Record<string, unknown>)
      if (requests.length === 1) throw new Error('response lost')
      return Response.json({ operation_id: 'op1' }, { status: 202 })
    } })
    routes['POST /api/projects/p1/threads'] = () => Response.json(threadFixture, { status: 201 })
    const app = (open = await startApp('/projects/p1/new?plan=plan1', routes))
    typeInto(document.querySelector('textarea')!, 'Carry on with this plan')
    await until(() => app.button('Send')?.disabled === false)
    await act(async () => app.button('Send')?.click())
    await until(() => app.path() === '/projects/p1/threads/t1' && app.text().includes('response lost'))
    await until(() => app.button('Send')?.disabled === false)
    await act(async () => app.button('Send')?.click())
    await until(() => requests.length === 2)
    expect(requests[0]?.plan).toBe('plan1')
    expect(requests[1]).toEqual(requests[0])
  })

  it('the first message creates the thread, then sends, then opens the thread', async () => {
    const routes = answers()
    routes['POST /api/projects/p1/threads'] = () => Response.json(threadFixture, { status: 201 })
    const app = (open = await startApp('/projects/p1/new', routes))
    await until(() => app.button('Accept edits') !== undefined)
    // No model is chosen before the session exists, and none is named default.
    expect(app.button('Set on send')?.disabled).toBe(true)
    expect(app.text()).not.toMatch(/default/i)
    await choose(app, 'Accept edits', 'Auto')

    typeInto(document.querySelector('textarea')!, 'Plan the release')
    await until(() => app.button('Send')?.disabled === false)
    app.button('Send')?.click()

    await until(() => app.path() === '/projects/p1/threads/t1')
    const posts = app.calls.filter((c) => c.startsWith('POST'))
    expect(posts.slice(0, 3)).toEqual([
      'POST /api/projects/p1/threads',
      'POST /api/threads/t1/session',
      'POST /api/threads/t1/turns',
    ])
    expect(app.bodies[0]).toMatchObject({ harness: 'claude-code', command_id: expect.any(String) })
    // The session's own model and effort; the mode the draft chose.
    expect(app.bodies[1]).toMatchObject({
      prompt: 'Plan the release',
      mode: 'auto',
      model: 'fake-large',
      effort: 'high',
    })
  })

  it('the mode chosen in the draft stays on the thread page and runs the second turn', async () => {
    let core = fakeChoices
    const routes = answers({
      session: () => Response.json(core),
      start: async (request: Request) => {
        const { mode } = await request.json() as { mode: string }
        core = { ...core, current: { ...core.current, mode } }
        return Response.json({ operation_id: 'op1' }, { status: 202 })
      },
    })
    routes['POST /api/projects/p1/threads'] = () => Response.json(threadFixture, { status: 201 })
    const app = (open = await startApp('/projects/p1/new', routes))
    await until(() => app.button('Accept edits') !== undefined)
    await choose(app, 'Accept edits', 'Auto')
    typeInto(document.querySelector('textarea')!, 'Plan the release')
    await until(() => app.button('Send')?.disabled === false)
    app.button('Send')?.click()

    await until(() => app.path() === '/projects/p1/threads/t1')
    await until(() => app.button('Auto') !== undefined || app.button('Accept edits') !== undefined)
    // Core reports the mode remembered from the first turn.
    expect(app.button('Auto')).toBeDefined()
    expect(app.button('Accept edits')).toBeUndefined()

    typeInto(document.querySelector('textarea')!, 'And the changelog')
    await until(() => app.button('Send')?.disabled === false)
    app.button('Send')?.click()
    await until(() => app.calls.filter((c) => c === 'POST /api/threads/t1/turns').length === 2)
    await until(() => app.bodies.length >= 3)
    expect(app.bodies.at(-1)).toMatchObject({ prompt: 'And the changelog', mode: 'auto' })
  })
})

describe('leaving a draft while it sends', () => {
  it('keeps the person where they went', async () => {
    const routes = answers()
    let answer: (r: Response) => void = () => {}
    routes['POST /api/projects/p1/threads'] = () =>
      new Promise<Response>((resolve) => {
        answer = resolve
      })
    const app = (open = await startApp('/projects/p1/new', routes))
    typeInto(document.querySelector('textarea')!, 'Plan the release')
    await until(() => app.button('Send')?.disabled === false)
    app.button('Send')?.click()
    await until(() => app.calls.includes('POST /api/projects/p1/threads'))

    const { router } = await import('@/router')
    await act(() =>
      router.navigate({ to: '/projects/$projectId/settings', params: { projectId: 'p1' } }),
    )
    answer(Response.json(threadFixture, { status: 201 }))
    await until(() => app.calls.includes('POST /api/threads/t1/turns'))
    // Past the moment the draft would have opened the thread.
    await act(() => new Promise((resolve) => setTimeout(resolve, 20)))
    expect(app.path()).toBe('/projects/p1/settings')
  })
})


it.each(['success', 'upload refusal', 'turn refusal', 'turn command conflict'] as const)('draft first-send images survive %s', async outcome => {
  const requests: Record<string, unknown>[] = []
  const routes = answers({ start: async (request: Request) => {
    requests.push(await request.json() as Record<string, unknown>)
    if (outcome === 'turn refusal' && requests.length === 1) return Response.json({ code: 'INVALID_COMMAND', message: 'Core refused the first image turn' }, { status: 422 })
    if (outcome === 'turn command conflict' && requests.length === 1) return Response.json({ code: 'COMMAND_CONFLICT', message: 'Core refused the first image turn' }, { status: 409 })
    return Response.json({ operation_id: 'op1' }, { status: 202 })
  } })
  routes['POST /api/projects/p1/threads'] = () => Response.json(threadFixture, { status: 201 })
  const app = open = await startApp('/projects/p1/new', routes)
  await until(() => app.button('Mode') !== undefined)
  vi.spyOn(URL, 'createObjectURL').mockReturnValue('blob:draft-preview')
  vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => {})
  const fallback = globalThis.fetch
  let uploads = 0
  vi.stubGlobal('fetch', async (request: Request) => {
    const key = `${request.method} ${new URL(request.url).pathname}`
    if (key === 'POST /api/threads/t1/attachments') {
      app.calls.push(key)
      const attempt = ++uploads
      const file = (await request.formData()).get('file') as File
      if (outcome === 'upload refusal' && attempt === 1) return Response.json({ code: 'INVALID_COMMAND', message: 'Core refused the draft bytes' }, { status: 422 })
      return Response.json({ id: file.name, name: file.name, media_type: file.type, byte_len: file.size })
    }
    return fallback(request)
  })
  const input = document.querySelector<HTMLInputElement>('input[type="file"]')!
  Object.defineProperty(input, 'files', { value: [new File(['one'], 'first.png', { type: 'image/png' }), new File(['two'], 'second.png', { type: 'image/png' })] })
  act(() => input.dispatchEvent(new Event('change', { bubbles: true })))
  expect(uploads).toBe(0)
  expect(app.container.querySelectorAll('img')).toHaveLength(2)
  typeInto(document.querySelector('textarea')!, 'My first images')
  await until(() => app.button('Send')?.disabled === false)
  act(() => app.button('Send')!.click())
  await until(() => app.path() === '/projects/p1/threads/t1')
  const sequence = app.calls.filter(call => call.startsWith('POST'))
  expect(sequence.indexOf('POST /api/projects/p1/threads')).toBeLessThan(sequence.indexOf('POST /api/threads/t1/attachments'))
  if (outcome !== 'success') {
    await until(() => app.text().includes(outcome === 'upload refusal' ? 'Core refused the draft bytes' : 'Core refused the first image turn'))
    expect(document.querySelector('textarea')?.value).toBe('My first images')
    expect(app.button('Remove first.png')).toBeDefined()
    expect(app.button('Remove second.png')).toBeDefined()
    if (outcome === 'upload refusal') {
      expect(requests).toHaveLength(0)
      act(() => app.button('Retry first.png')!.click())
      await until(() => app.button('Retry first.png') === undefined)
    }
    await until(() => app.button('Send')?.disabled === false)
    act(() => app.button('Send')!.click())
  }
  await until(() => requests.length === (outcome === 'turn refusal' || outcome === 'turn command conflict' ? 2 : 1))
  expect(requests.at(-1)).toMatchObject({ prompt: 'My first images', attachments: ['first.png', 'second.png'] })
  expect(app.calls.filter(call => call === 'POST /api/projects/p1/threads')).toHaveLength(1)
  expect(uploads).toBe(outcome === 'upload refusal' ? 3 : 2)
  if (outcome === 'turn refusal') expect(requests[1]).toEqual(requests[0])
  if (outcome === 'turn command conflict') {
    expect(requests[1]?.command_id).not.toBe(requests[0]?.command_id)
    expect(requests[1]).toEqual({ ...requests[0], command_id: requests[1]?.command_id })
  }
  expect(URL.revokeObjectURL).toHaveBeenCalledWith('blob:draft-preview')
  expect(app.calls.indexOf('POST /api/threads/t1/attachments')).toBeLessThan(app.calls.indexOf('POST /api/threads/t1/turns'))
})


it('the draft keeps staged images when its opened session refuses images', async () => {
  const reason = 'Core says this session cannot receive images'
  const routes = answers({ session: { ...fakeChoices, capabilities: { image: false, image_reason: reason } } })
  routes['POST /api/projects/p1/threads'] = () => Response.json(threadFixture, { status: 201 })
  const app = open = await startApp('/projects/p1/new', routes)
  await until(() => app.button('Mode') !== undefined)
  vi.spyOn(URL, 'createObjectURL').mockReturnValue('blob:draft-preview')
  vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => {})
  const input = document.querySelector<HTMLInputElement>('input[type="file"]')!
  Object.defineProperty(input, 'files', { value: [new File(['one'], 'first.png', { type: 'image/png' })] })
  act(() => input.dispatchEvent(new Event('change', { bubbles: true })))
  typeInto(document.querySelector('textarea')!, 'Keep staged images')
  await until(() => app.button('Send')?.disabled === false)
  act(() => app.button('Send')!.click())
  await until(() => app.path() === '/projects/p1/threads/t1' && app.text().includes(reason))
  expect(document.querySelector('textarea')?.value).toBe('Keep staged images')
  expect(app.button('Remove first.png')).toBeDefined()
  expect(app.calls).not.toContain('POST /api/threads/t1/attachments')
  expect(app.calls).not.toContain('POST /api/threads/t1/turns')
  expect(document.querySelector<HTMLInputElement>('input[type="file"]')?.disabled).toBe(true)
})
