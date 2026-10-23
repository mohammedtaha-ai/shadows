// @vitest-environment happy-dom
//
// A new conversation's draft (§13.11): nothing exists until the first message,
// which creates the thread, starts its turn with the chosen mode, and replaces
// the draft's URL with the thread's.

import { act } from 'react'
import { afterEach, describe, expect, it } from 'vitest'
import { fakeChoices, planFixture, planListing, threadFixture } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, choose, startApp, typeInto, until } from '../test-app'

let open: TestApp | null = null
afterEach(() => {
  open?.unmount()
  open = null
})

describe('the draft', () => {
  it('a draft starts at the policy default without a session mode write', async () => {
    const app = open = await startApp('/projects/p1/new', answers())
    await until(() => app.button('Mode') !== undefined)
    expect(app.button('Mode')?.textContent).toContain('Accept edits')
    await choose(app, 'Mode', 'Auto')
    expect(app.button('Mode')?.textContent).toContain('Auto')
    expect(app.calls.some(c => c.includes('/session'))).toBe(false)
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
