// @vitest-environment happy-dom
//
// The New project dialog over a faked daemon: a Create whose answer was lost
// is retried under the same command id, a changed request gets a new one, and
// a refusal is shown in the dialog.

import { act } from 'react'
import { afterEach, describe, expect, it } from 'vitest'
import { type TestApp, startApp, typeInto, until } from '../test-app'

let app: TestApp | null = null
afterEach(() => {
  app?.unmount()
  app = null
})

describe('the New project dialog', () => {
  it('retries a lost create under the same command id, and a changed one under a new id', async () => {
    let attempt = 0
    const a = (app = await startApp('/', {
      'GET /api/projects': [],
      'GET /api/fs/dirs': { path: null, parent: null, entries: [] },
      'POST /api/projects': () => {
        attempt += 1
        // The first answer is lost on the way back; the second is a refusal.
        if (attempt === 1) throw new TypeError('Failed to fetch')
        return Response.json(
          { code: 'PATH_NOT_FOUND', message: 'no such folder' },
          { status: 404 },
        )
      },
    }))

    await until(() => a.button('New project') !== undefined)
    await act(async () => a.button('New project')?.click())
    await until(() => document.querySelector('#new-project-folder') !== null)

    const name = document.querySelector<HTMLInputElement>('input[placeholder="My project"]')
    const folder = document.querySelector<HTMLInputElement>('#new-project-folder')
    if (name === null || folder === null) throw new Error('the dialog has no fields')
    typeInto(name, 'My Project')
    typeInto(folder, 'C:\\work')
    expect(document.body.textContent).toContain('slug: my-project')

    await act(async () => a.button('Create project')?.click())
    await until(() => document.body.textContent?.includes('not reachable') ?? false)
    await act(async () => a.button('Create project')?.click())
    await until(() => document.body.textContent?.includes('PATH_NOT_FOUND') ?? false)

    typeInto(folder, 'C:\\work2')
    await act(async () => a.button('Create project')?.click())
    await until(() => a.bodies.length === 3)

    const ids = a.bodies.map((body) => (body as { command_id: string }).command_id)
    expect(ids[1]).toBe(ids[0])
    expect(ids[2]).not.toBe(ids[0])
    expect(a.bodies[0]).toMatchObject({ name: 'My Project', slug: 'my-project', directory: 'C:\\work' })
  })
})
