// @vitest-environment happy-dom
//
// A new conversation's draft (§13.11): nothing exists until the first message,
// which creates the thread, starts its turn with the chosen mode, and replaces
// the draft's URL with the thread's.

import { afterEach, describe, expect, it } from 'vitest'
import { threadFixture } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, choose, startApp, typeInto, until } from '../test-app'

let open: TestApp | null = null
afterEach(() => {
  open?.unmount()
  open = null
})

describe('the draft', () => {
  it('the first message creates the thread, then sends, then opens the thread', async () => {
    const routes = answers()
    routes['POST /api/projects/p1/threads'] = () => Response.json(threadFixture, { status: 201 })
    const app = (open = await startApp('/projects/p1/new', routes))
    await until(() => app.button('Accept edits') !== undefined)
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
})
