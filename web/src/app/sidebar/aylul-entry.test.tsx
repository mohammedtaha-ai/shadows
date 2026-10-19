// @vitest-environment happy-dom
//
// AYLUL's own entry above Projects: its project is not listed among them.

import { afterEach, describe, expect, it } from 'vitest'
import { projectFixture, threadFixture } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, until } from '../test-app'

const aylul = { ...projectFixture, id: 'pa', slug: 'aylul', name: 'AYLUL', assistant: true }

let app: TestApp | null = null
afterEach(() => {
  app?.unmount()
  app = null
  window.localStorage.clear()
})

describe('the AYLUL entry', () => {
  it('sidebar_shows_aylul_entry_and_hides_its_project', async () => {
    const a = (app = await startApp('/', {
      ...answers(),
      'GET /api/projects': [projectFixture, aylul],
      'GET /api/projects/pa/threads': [{ ...threadFixture, id: 'ta', project_id: 'pa' }],
    }))
    const entries = () =>
      [...document.querySelectorAll('nav a')].filter((l) => l.textContent?.trim() === 'AYLUL')
    await until(() => entries().length === 1 && a.text().includes('Demo'))
    // One entry, to the latest conversation; the project is not among Projects.
    expect(entries()[0]?.getAttribute('href')).toBe('/projects/pa/threads/ta')
    const rows = [...document.querySelectorAll('button[aria-expanded]')]
    expect(rows.map((r) => r.textContent)).toEqual([expect.stringContaining('Demo')])
  })
})
