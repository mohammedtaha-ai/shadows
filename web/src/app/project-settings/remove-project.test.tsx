// @vitest-environment happy-dom
//
// Removing a project from its settings over a faked daemon (spec §4.2,
// §13.11): refused in the page while it has conversations; otherwise
// confirmed, sent as one command, and the person lands home.

import { act } from 'react'
import { afterEach, describe, expect, it } from 'vitest'
import type { Project } from '@/api/client'
import { projectFixture, threadFixture } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, until } from '../test-app'

const PAGE = '/projects/p1/settings'

let open: TestApp | null = null
afterEach(() => {
  open?.unmount()
  open = null
})

describe('remove project', () => {
  it('is off while the project has conversations, and says why', async () => {
    const app = (open = await startApp(PAGE, answers()))
    await until(() =>
      app.text().includes('This project has 1 conversation. Delete them from the sidebar first.'),
    )
    expect(app.button('Remove')?.disabled).toBe(true)
  })

  it('with none, confirms, sends the command and goes home', async () => {
    const table = answers()
    let projects: Project[] = [projectFixture]
    const urls: string[] = []
    table['GET /api/projects'] = () => Response.json(projects)
    table['GET /api/projects/p1/threads'] = []
    table['DELETE /api/projects/p1'] = (r: Request) => {
      urls.push(r.url)
      projects = []
      return Response.json(projectFixture)
    }

    const app = (open = await startApp(PAGE, table))
    await until(() => app.button('Remove')?.disabled === false)
    act(() => app.button('Remove')?.click())
    await until(() => app.text().includes('Its folder on disk is not touched'))
    expect(app.text()).toContain('stays taken')
    expect(urls).toHaveLength(0)

    act(() => app.button('Remove project')?.click())
    await until(() => app.path() === '/')
    expect(new URL(urls[0]!).searchParams.get('command_id')).toMatch(/.+/)
    // The project is gone from the sidebar.
    await until(() => app.text().includes('No projects yet.'))
  })

  it('a retry after a lost answer sends the same command', async () => {
    const table = answers()
    table['GET /api/projects/p1/threads'] = []
    const ids: (string | null)[] = []
    table['DELETE /api/projects/p1'] = (r: Request) => {
      ids.push(new URL(r.url).searchParams.get('command_id'))
      return ids.length === 1
        ? Response.json(
            { code: 'STORAGE_UNAVAILABLE', message: 'database is locked' },
            { status: 500 },
          )
        : Response.json(projectFixture)
    }

    const app = (open = await startApp(PAGE, table))
    await until(() => app.button('Remove')?.disabled === false)
    act(() => app.button('Remove')?.click())
    await until(() => app.button('Remove project') !== undefined)
    act(() => app.button('Remove project')?.click())
    await until(() => app.text().includes('database is locked'))

    // Closed and opened again: the same removal, so the same command id.
    act(() => app.button('Cancel')?.click())
    await until(() => app.button('Remove project') === undefined)
    act(() => app.button('Remove')?.click())
    await until(() => app.button('Remove project') !== undefined)
    act(() => app.button('Remove project')?.click())
    await until(() => app.path() === '/')
    expect(ids).toHaveLength(2)
    expect(ids[1]).toBe(ids[0])
  })

  it('a conversation started meanwhile is refused in the dialog, and turns Remove off', async () => {
    const table = answers()
    let threads: unknown[] = []
    table['GET /api/projects/p1/threads'] = () => Response.json(threads)
    table['DELETE /api/projects/p1'] = () => {
      threads = [threadFixture]
      return Response.json(
        { code: 'PROJECT_HAS_THREADS', message: 'the project has conversations' },
        { status: 409 },
      )
    }

    const app = (open = await startApp(PAGE, table))
    await until(() => app.button('Remove')?.disabled === false)
    act(() => app.button('Remove')?.click())
    await until(() => app.button('Remove project') !== undefined)
    act(() => app.button('Remove project')?.click())
    await until(() => app.text().includes('PROJECT_HAS_THREADS'))
    expect(app.text()).toContain('the project has conversations')
    expect(app.path()).toBe(PAGE)
    await until(() => app.text().includes('This project has 1 conversation.'))
    expect(app.button('Remove')?.disabled).toBe(true)
  })
})
