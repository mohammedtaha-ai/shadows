// @vitest-environment happy-dom
//
// The sidebar's Workflows section over a faked daemon (§13.11): each
// conversation's latest plan, an invitation when there is none, and why the
// list could not be read.

import { afterEach, describe, expect, it } from 'vitest'
import { planFixture, planListing } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, until } from '../test-app'

const PAGE = '/projects/p1/workflows/w1'

let app: TestApp | null = null
afterEach(() => {
  app?.unmount()
  app = null
})

describe('the Workflows section', () => {
  it('the sidebar lists the project’s plans, or invites asking for one', async () => {
    const a = (app = await startApp(PAGE, answers({ plans: [planListing(planFixture({ version: 2 }))] })))
    const sidebarLink = () =>
      [...document.querySelectorAll('nav[aria-label="Projects and conversations"] a')].find((l) =>
        l.textContent?.includes('Login flow'),
      )
    await until(() => sidebarLink() !== undefined)
    const link = sidebarLink()
    expect(link?.textContent).toContain('Draft v2')
    expect(link?.getAttribute('href')).toBe(PAGE)
    a.unmount()

    const empty = (app = await startApp('/projects/p1', answers()))
    await until(() => empty.text().includes('Ask the Planner for a plan'))
  })

  it('shows a sidebar error when plans cannot be listed', async () => {
    const routes = answers()
    routes['GET /api/projects/p1/workflows'] = () =>
      Response.json({ code: 'STORAGE_UNAVAILABLE', message: 'database is locked' }, { status: 503 })
    const a = (app = await startApp('/projects/p1', routes))

    await until(() => a.text().includes('database is locked'))
    expect(a.container.querySelector('[role="alert"]')?.textContent).toContain('STORAGE_UNAVAILABLE')
  })
})
