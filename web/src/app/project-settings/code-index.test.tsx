// @vitest-environment happy-dom

import { act } from 'react'
import { afterEach, describe, expect, it } from 'vitest'
import type { ProjectLink } from '@/api/code'
import type { Project } from '@/api/projects'
import { projectFixture } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, choose, startApp, until } from '@/test/test-app'

let open: TestApp | null = null
afterEach(() => {
  open?.unmount()
  open = null
})

const backend: Project = {
  ...projectFixture,
  id: 'p2',
  slug: 'backend',
  name: 'Backend',
  directory: 'C:\\work\\backend',
}

const linkRows = () => [...document.querySelectorAll('[data-link]')]

describe('reads code from', () => {
  it('links another project, then unlinks it', async () => {
    const table = answers()
    let links: ProjectLink[] = []
    const urls: string[] = []
    table['GET /api/projects'] = [projectFixture, backend]
    table['GET /api/projects/p1/code/links'] = () => Response.json(links)
    table['PUT /api/projects/p1/code/links/p2'] = () => {
      const link = { project: 'demo', linked: 'backend', created_at: '2026-09-30T00:00:00Z' }
      links = [link]
      return Response.json(link)
    }
    table['DELETE /api/projects/p1/code/links/p2'] = (r: Request) => {
      urls.push(r.url)
      links = []
      return new Response(null, { status: 204 })
    }

    const app = (open = await startApp('/projects/p1/settings', table))
    await until(() => app.text().includes('Only this project.'))
    expect(app.text()).not.toContain('Code index')

    await choose(app, 'Link a project…', 'Backend')
    await until(() => linkRows().length === 1)
    expect(app.calls).toContain('PUT /api/projects/p1/code/links/p2')
    expect(app.bodies.at(-1)).toEqual({ command_id: expect.stringMatching(/.+/) })
    // The row is the linked project as the person knows it.
    const row = linkRows()[0]!
    expect(row.textContent).toContain('Backend')
    expect(row.textContent).toContain('C:\\work\\backend')
    // Nothing is left to link: this project and Backend are both out.
    expect(app.button('Link a project…')?.disabled).toBe(true)

    act(() => app.button('Unlink Backend')?.click())
    await until(() => linkRows().length === 0)
    expect(new URL(urls[0]!).searchParams.get('command_id')).toMatch(/.+/)
    expect(app.text()).toContain('Only this project.')
    expect(app.button('Link a project…')?.disabled).toBe(false)
  })
})
