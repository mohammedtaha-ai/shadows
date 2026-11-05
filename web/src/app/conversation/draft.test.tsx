// @vitest-environment happy-dom
//

import { act } from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { ThreadPlanningAnchor } from '@/api/planning-context'
import { fakeChoices, planFixture, planListing, threadFixture } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, choose, startApp, typeInto, until } from '@/test/test-app'

let open: TestApp | null = null
afterEach(() => {
  open?.unmount()
  open = null
  vi.restoreAllMocks()
})

const draftAnchor: ThreadPlanningAnchor = { kind: 'part', id: 'auth' }
const anchoredDraft = '/projects/p1/new?anchor=part%3Aauth'
const anchorRoute = 'PUT /api/threads/t1/anchor'
const turnRoute = 'POST /api/threads/t1/turns'

function anchorDaemon() {
  let anchor: ThreadPlanningAnchor | null = null
  const sentAnchors: (ThreadPlanningAnchor | null)[] = []
  const writes: { command_id: string; anchor: ThreadPlanningAnchor | null }[] = []
  const routes = answers({ start: () => {
    sentAnchors.push(anchor)
    return Response.json({ operation_id: 'op1' }, { status: 202 })
  } })
  routes['POST /api/projects/p1/threads'] = () => Response.json(threadFixture, { status: 201 })
  routes['GET /api/threads/t1/anchor'] = () => Response.json({ anchor })
  routes['GET /api/projects/p1/design/parts/auth'] = {
    part: { id: 'auth', content: { title: 'Authentication' } },
  }
  const answer = (value: ThreadPlanningAnchor | null) => {
    anchor = value
    return Response.json({ anchor })
  }
  const intercept = (respond: (body: (typeof writes)[number], call: number) => Response | Promise<Response>) => {
    routes[anchorRoute] = async (request: Request) => {
      const body = await request.json() as (typeof writes)[number]
      writes.push(body)
      return respond(body, writes.length)
    }
  }
  intercept(body => answer(body.anchor))
  return { routes, sentAnchors, writes, answer, intercept }
}

async function pressSend(app: TestApp, label = 'Send') {
  await until(() => app.button(label)?.disabled === false)
  await act(async () => app.button(label)?.click())
}

async function submitDraft(routes: ReturnType<typeof answers>, url = anchoredDraft) {
  const app = open = await startApp(url, routes)
  await until(() => app.container.querySelector('textarea') !== null)
  typeInto(app.container.querySelector('textarea')!, 'Plan the release')
  await pressSend(app)
  return app
}

async function appRouter() {
  return (await import('@/router')).router
}

async function revisitThread(app: TestApp) {
  const router = await appRouter()
  await act(() => router.navigate({ to: '/projects/$projectId/settings', params: { projectId: 'p1' } }))
  await until(() => app.path() === '/projects/p1/settings')
  await act(() => router.navigate({
    to: '/projects/$projectId/threads/$threadId', params: { projectId: 'p1', threadId: 't1' },
  }))
}

it('the_first_send_sets_the_anchor_before_the_turn', async () => {
  const daemon = anchorDaemon()
  const app = open = await startApp(anchoredDraft, daemon.routes)
  await until(() => app.text().includes('Anchored at Authentication'))
  typeInto(app.container.querySelector('textarea')!, 'Plan the release')
  await pressSend(app)
  await until(() => daemon.sentAnchors.length === 1)
  expect(app.calls.filter(c => c === anchorRoute || c === turnRoute)).toEqual([anchorRoute, turnRoute])
  expect(daemon.sentAnchors).toEqual([draftAnchor])
})

it('a_failed_anchor_keeps_the_thread_and_the_text', async () => {
  const daemon = anchorDaemon()
  daemon.intercept(() => Response.json({ code: 'NOT_FOUND', message: 'The part was removed.' }, { status: 404 }))
  const app = await submitDraft(daemon.routes)
  await until(() => app.path() === '/projects/p1/threads/t1' && app.text().includes('The part was removed.'))
  expect(app.container.querySelector('textarea')?.value).toBe('Plan the release')
  expect(app.calls).not.toContain(turnRoute)
})

it('a_retry_sets_the_anchor_first_with_the_same_command_id', async () => {
  const daemon = anchorDaemon()
  daemon.intercept((body, call) => {
    const response = daemon.answer(body.anchor)
    if (call === 1) throw new TypeError('Response lost')
    return response
  })
  const app = await submitDraft(daemon.routes)
  await until(() => app.path() === '/projects/p1/threads/t1')
  await pressSend(app)
  await until(() => daemon.sentAnchors.length === 1)
  expect(daemon.writes[1]).toEqual(daemon.writes[0])
  expect(app.calls.filter(c => c === anchorRoute || c === turnRoute)).toEqual([anchorRoute, anchorRoute, turnRoute])
  expect(app.calls.filter(c => c === 'POST /api/projects/p1/threads')).toHaveLength(1)
  expect(daemon.sentAnchors).toEqual([draftAnchor])
})

it('send_without_the_anchor_waits_for_the_pending_write', async () => {
  const daemon = anchorDaemon()
  let finishWrite: () => void = () => { throw new Error('The first write has not started.') }
  const committed = new Promise<void>(resolve => {
    daemon.intercept(async (body, call) => {
      if (call === 1) {
        finishWrite = () => { daemon.answer(body.anchor); resolve() }
        throw new TypeError('Connection closed with the write pending')
      }
      if (body.anchor === null) return daemon.answer(null)
      await committed
      return Response.json({ anchor: body.anchor })
    })
  })
  const app = await submitDraft(daemon.routes)
  await until(() => app.path() === '/projects/p1/threads/t1')
  await pressSend(app, 'Send without the anchor')
  await until(() => daemon.writes.length === 2)
  expect(daemon.writes[1]).toEqual(daemon.writes[0])
  expect(app.calls.filter(c => c === anchorRoute || c === turnRoute)).toEqual([anchorRoute, anchorRoute])
  await act(async () => finishWrite())
  await until(() => daemon.sentAnchors.length === 1)
  expect(app.calls.filter(c => c === anchorRoute || c === turnRoute)).toEqual([anchorRoute, anchorRoute, anchorRoute, turnRoute])
  expect(daemon.writes[2]?.anchor).toBeNull()
  expect(daemon.writes[2]?.command_id).not.toBe(daemon.writes[0]?.command_id)
  expect(daemon.sentAnchors).toEqual([null])
})

it.each([
  ['a_gateway_answer_is_treated_as_unknown', true, () => new Response('Gateway timeout', { status: 504 })],
  ['send_without_the_anchor_after_a_refusal_only_clears', false,
    () => Response.json({ code: 'NOT_FOUND', message: 'The part was removed.' }, { status: 404 })],
  ['a_daemon_server_error_only_clears', false,
    () => Response.json({ code: 'INTERNAL', message: 'The write failed.' }, { status: 500 })],
] as const)('%s', async (_name, replay, firstAnswer) => {
  const daemon = anchorDaemon()
  daemon.intercept((body, call) => call === 1 ? firstAnswer() : daemon.answer(body.anchor))
  const app = await submitDraft(daemon.routes)
  await until(() => app.path() === '/projects/p1/threads/t1')
  await pressSend(app, 'Send without the anchor')
  await until(() => daemon.sentAnchors.length === 1)
  expect(daemon.writes.map(write => write.anchor)).toEqual(replay ? [draftAnchor, draftAnchor, null] : [draftAnchor, null])
  if (replay) expect(daemon.writes[1]).toEqual(daemon.writes[0])
  expect(daemon.sentAnchors).toEqual([null])
})

it('an_unknown_anchor_value_is_ignored', async () => {
  const daemon = anchorDaemon()
  const app = await submitDraft(daemon.routes, '/projects/p1/new?anchor=unknown%3Aauth')
  await until(() => daemon.sentAnchors.length === 1)
  expect(app.calls).not.toContain(anchorRoute)
  expect(daemon.sentAnchors).toEqual([null])
})

it('a_lost_clear_answer_retries_the_clear_before_sending', async () => {
  const daemon = anchorDaemon()
  daemon.intercept((body, call) => {
    if (call === 1) return Response.json({ code: 'NOT_FOUND', message: 'The part was removed.' }, { status: 404 })
    const response = daemon.answer(body.anchor)
    if (call === 2) throw new TypeError('Clear response lost')
    return response
  })
  const app = await submitDraft(daemon.routes)
  await until(() => app.path() === '/projects/p1/threads/t1')
  await pressSend(app, 'Send without the anchor')
  await until(() => app.text().includes('The daemon is not reachable'))
  expect(app.calls).not.toContain(turnRoute)
  expect(app.button('Send')).toBeUndefined()
  await revisitThread(app)
  await pressSend(app, 'Send without the anchor')
  await until(() => daemon.sentAnchors.length === 1)
  expect(daemon.writes[1]?.anchor).toBeNull()
  expect(daemon.writes[2]).toEqual(daemon.writes[1])
  expect(daemon.sentAnchors).toEqual([null])
})

it('a_revisited_thread_keeps_its_anchor_attempt_and_blocks_header_writes', async () => {
  const daemon = anchorDaemon()
  daemon.intercept((body, call) => {
    if (call === 1) throw new TypeError('Anchor response lost')
    return daemon.answer(body.anchor)
  })
  const app = await submitDraft(daemon.routes)
  await until(() => app.path() === '/projects/p1/threads/t1')
  await revisitThread(app)
  await until(() => app.button('Anchor…') !== undefined)
  expect(app.button('Anchor…')?.disabled).toBe(true)
  expect(app.container.querySelector('textarea')?.value).toBe('Plan the release')
  await pressSend(app)
  await until(() => daemon.sentAnchors.length === 1)
  expect(daemon.writes[1]).toEqual(daemon.writes[0])
  expect(daemon.sentAnchors).toEqual([draftAnchor])
})

it('the_open_thread_receives_a_draft_whose_anchor_write_fails_late', async () => {
  const daemon = anchorDaemon()
  let refuse!: (error: Error) => void
  daemon.intercept(() => new Promise<Response>((_resolve, reject) => { refuse = reject }))
  const app = await submitDraft(daemon.routes)
  await until(() => daemon.writes.length === 1)
  const router = await appRouter()
  await act(() => router.navigate({
    to: '/projects/$projectId/threads/$threadId', params: { projectId: 'p1', threadId: 't1' },
  }))
  await until(() => app.container.querySelector('textarea')?.readOnly === true)
  expect(app.container.querySelector('textarea')?.value).toBe('')
  await act(async () => refuse(new TypeError('Connection closed')))
  await until(() => app.button('Send without the anchor') !== undefined)
  expect(app.container.querySelector('textarea')?.value).toBe('Plan the release')
  expect(app.text()).toContain('The daemon is not reachable')
  expect(app.calls).not.toContain(turnRoute)
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

    const router = await appRouter()
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
