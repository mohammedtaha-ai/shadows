// @vitest-environment happy-dom
//
// Session-setting changes over a faked daemon preserve Core's answers.

import { sessionKeys } from '@/api/sessions'
import { act } from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { choice, fakeChoices, threadFixture } from '@/test/contract-fixtures'
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

describe('session settings', () => {
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
    // Project settings has its own instructions textarea.
    await until(() => app.path() === '/projects/p1/settings' &&
      app.container.querySelector('textarea[aria-label="Message"]') === null)
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
    const key = sessionKeys.choices('t1')
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
    await until(() => app.queryClient.getQueryData<{ current: { mode: string } }>(sessionKeys.choices('t1'))?.current.mode === 'auto')
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

  it('choosing a value closes its menu, so it never covers Send', async () => {
    const app = await start('/projects/p1/threads/t1', answers())
    await until(ready(app))
    await choose(app, 'Accept edits', 'Auto')
    await until(() => app.button('Auto')?.disabled === false)
    await until(() => menuItem('Auto') === undefined)
    expect(app.button('Auto')).toBeDefined()
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
    // no max; it is not set back to the person's previous one.
    await until(() => app.button('high')?.disabled === false)
    act(() => app.button('high')?.click())
    await until(() => menuItem('low') !== undefined)
    expect(menuItem('max')).toBeUndefined()
    expect(app.button('Send')?.disabled).toBe(false)
    expect(app.bodies).toHaveLength(2)
    expect(turnStarts(app)).toBe(0)
  })

  it('a picked model ends at the effort its answer reports, not an earlier report', async () => {
    // A model change can report its carried effort before the daemon
    // answers with the remembered effort that must stand.
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
})
