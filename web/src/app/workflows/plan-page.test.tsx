// @vitest-environment happy-dom
//
// The Workflows page over a faked daemon (§13.11): the graph, the list above
// Approve, an approval that meets a changed plan (Review Focus 4), an
// approved version, and Arabic text (Review Focus 3).

import { act } from 'react'
import { afterEach, describe, expect, it } from 'vitest'
import type { Plan } from '@/api/client'
import { answers } from '@/test/fake-daemon'
import { planFixture, planListing, planTask } from '@/test/contract-fixtures'
import { type TestApp, startApp, until } from '../test-app'

const PAGE = '/projects/p1/workflows/w1'

let app: TestApp | null = null
afterEach(() => {
  app?.unmount()
  app = null
})

/** The graph node of task `T{n}`. */
function node(n: number): HTMLElement | null {
  return document.querySelector<HTMLElement>(`.react-flow__node[data-id="t${n}"]`)
}

/** Answers `plans` in turn, then the last one for every later read. */
function inTurn(...plans: Plan[]) {
  let reads = 0
  return () => Response.json(plans[Math.min(reads++, plans.length - 1)])
}

function getsOfPlan(a: TestApp): number {
  return a.calls.filter((c) => c === 'GET /api/workflows/w1').length
}

describe('the plan page', () => {
  it('shows Draft v2 with its blockers above Approve', async () => {
    const plan = planFixture({
      version: 2,
      blockers: [{ message: 'T2 has no acceptance items' }],
    })
    const a = (app = await startApp(PAGE, answers({ plan, plans: [planListing(plan)] })))
    await until(() => a.button('Approve') !== undefined)

    expect(a.container.textContent).toContain('Draft v2')
    const blocker = [...a.container.querySelectorAll('li')].find(
      (li) => li.textContent === 'T2 has no acceptance items',
    )
    if (blocker === undefined) throw new Error('the blocker is not listed')
    const approve = a.button('Approve')
    if (approve === undefined) throw new Error('no Approve')
    // Above it, and Approve stays enabled: pressed anyway, it shows the 422.
    expect(blocker.compareDocumentPosition(approve) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
    expect(approve.disabled).toBe(false)
  })

  it('approve sends the revision it showed', async () => {
    const a = (app = await startApp(
      PAGE,
      answers({ plan: inTurn(planFixture({ revision: 7 }), planFixture({ state: 'Frozen', revision: 7 })) }),
    ))
    await until(() => a.button('Approve') !== undefined)
    const reads = getsOfPlan(a)

    await act(async () => a.button('Approve')?.click())
    await until(() => a.calls.includes('POST /api/workflows/w1/approve'))

    const body = a.bodies.at(-1) as { command_id: string; expected_revision: number }
    expect(body.expected_revision).toBe(7)
    expect(body.command_id).toMatch(/^[0-9a-f-]{36}$/)
    // The answer is an `Approved`, not the plan: the page reads the plan again.
    await until(() => getsOfPlan(a) > reads)
    await until(() => a.container.textContent?.includes('Approved v1') === true)
    expect(a.button('Approve')).toBeUndefined()
  })

  it('a revision conflict on Approve refetches and says the plan changed', async () => {
    const seen = planFixture({ revision: 3 })
    const changed = planFixture({
      revision: 4,
      tasks: [...seen.tasks, planTask(3, 'Session timeout')],
    })
    const a = (app = await startApp(
      PAGE,
      answers({
        plan: inTurn(seen, changed),
        approve: () =>
          Response.json(
            { code: 'REVISION_CONFLICT', message: 'the plan changed', current_revision: 4 },
            { status: 409 },
          ),
      }),
    ))
    await until(() => a.button('Approve') !== undefined)

    await act(async () => a.button('Approve')?.click())
    await until(() =>
      a.text().includes('The plan changed while you were looking; review it again'),
    )
    // What changed is on screen before the person can approve it.
    await until(() => node(3)?.textContent?.includes('Session timeout') === true)

    await act(async () => a.button('Approve')?.click())
    await until(() => a.calls.filter((c) => c === 'POST /api/workflows/w1/approve').length === 2)
    const [first, second] = a.bodies.slice(-2) as { command_id: string; expected_revision: number }[]
    expect(first?.expected_revision).toBe(3)
    expect(second?.expected_revision).toBe(4)
    // A different request is a new command.
    expect(second?.command_id).not.toBe(first?.command_id)
  })

  it('shows the problems when approval is refused as invalid', async () => {
    const a = (app = await startApp(
      PAGE,
      answers({
        approve: () =>
          Response.json(
            {
              code: 'WORKFLOW_VALIDATION_FAILED',
              message: 'T1 has no goal; T2 has no acceptance items',
              problems: ['T1 has no goal', 'T2 has no acceptance items'],
            },
            { status: 422 },
          ),
      }),
    ))
    await until(() => a.button('Approve') !== undefined)
    await act(async () => a.button('Approve')?.click())
    await until(() => a.text().includes('T2 has no acceptance items'))
    const items = [...a.container.querySelectorAll('[role="alert"] li')].map((li) => li.textContent)
    expect(items).toEqual(['T1 has no goal', 'T2 has no acceptance items'])
  })

  it('an approved plan has no Approve and shows the banner', async () => {
    const a = (app = await startApp(
      PAGE,
      answers({ plan: planFixture({ state: 'Frozen', frozen_at: '2026-09-25T01:00:00Z' }) }),
    ))
    await until(() => a.text().includes('Approved v1 · editing creates draft v2'))
    expect(a.button('Approve')).toBeUndefined()
  })

  it('changed tasks are marked', async () => {
    const plan = planFixture({
      last_edit: { revision: 3, summary: 'Renamed T2', changed_tasks: [2] },
    })
    app = await startApp(PAGE, answers({ plan }))
    await until(() => node(2) !== null)

    expect(node(2)?.textContent).toContain('changed')
    expect(node(1)?.textContent).not.toContain('changed')
  })

  it('a completes_after edge is dashed and labelled', async () => {
    const plan = planFixture({
      tasks: [planTask(1, 'Schema'), planTask(2, 'Login screen'), planTask(3, 'Audit')],
      links: [
        { task: 2, after: 1, kind: 'needs', label: 'the users table', waiting_items: [] },
        { task: 3, after: 2, kind: 'completes_after', label: 'the final screen', waiting_items: [1] },
      ],
    })
    const a = (app = await startApp(PAGE, answers({ plan })))
    await until(() => document.querySelector('[data-link="completes_after-2-3"]') !== null)

    const dashed = document.querySelector<SVGPathElement>('path[data-link="completes_after-2-3"]')
    const solid = document.querySelector<SVGPathElement>('path[data-link="needs-1-3"], path[data-link="needs-1-2"]')
    expect(dashed?.style.strokeDasharray).not.toBe('')
    expect(solid?.style.strokeDasharray ?? '').toBe('')
    const label = document.querySelector('div[data-link="completes_after-2-3"]')
    expect(label?.textContent).toBe('the final screen')
    expect(node(3)?.textContent).toContain('1 part waits for T2')
    expect(a.text()).toContain('the users table')
  })

  it('an Arabic task title renders with dir auto', async () => {
    const title = 'شاشة تسجيل الدخول'
    const plan = planFixture({ tasks: [planTask(1, 'Schema'), planTask(2, title)] })
    const a = (app = await startApp(PAGE, answers({ plan })))
    await until(() => node(2) !== null)

    const element = [...(node(2)?.querySelectorAll('*') ?? [])].find((e) => e.textContent === title)
    expect(element?.getAttribute('dir')).toBe('auto')
    // Every text element in the node says so, not only the title.
    for (const text of node(2)?.querySelectorAll('p, span') ?? []) {
      expect(text.getAttribute('dir'), text.outerHTML).toBe('auto')
    }
    expect(a.container.textContent).toContain(title)
  })

  it('refits the graph when Inspect opens and closes', async () => {
    const a = (app = await startApp(PAGE, answers()))
    await until(() => node(2) !== null)
    const fullGraph = a.container.querySelector('.react-flow')
    if (fullGraph === null) throw new Error('no graph was mounted')

    await act(async () => node(2)?.click())
    await until(() => a.container.querySelector('aside[aria-label="T2"]') !== null)
    const narrowGraph = a.container.querySelector('.react-flow')
    expect(narrowGraph).not.toBe(fullGraph)

    await act(async () => a.button('Close')?.click())
    await until(() => a.container.querySelector('aside[aria-label="T2"]') === null)
    expect(a.container.querySelector('.react-flow')).not.toBe(narrowGraph)
  })

  it('a Workflow durable frame refetches the plan', async () => {
    const a = (app = await startApp(
      PAGE,
      answers({
        plan: inTurn(
          planFixture(),
          planFixture(),
          planFixture({ revision: 4, tasks: [...planFixture().tasks, planTask(3, 'Session timeout')] }),
        ),
      }),
    ))
    await until(() => node(2) !== null)
    await until(() => a.sources.length > 0)
    const stream = a.sources.at(-1)
    if (stream === undefined) throw new Error('no stream was opened')
    expect(stream.param('thread_id')).toBe('t1')

    await act(async () => stream.caughtUp(0))
    const reads = getsOfPlan(a)
    await act(async () =>
      stream.durable(1, 'WorkflowEdited', null, {
        workflow_id: 'w1',
        version: 1,
        revision: 4,
        summary: 'Added T3',
        changed_tasks: [3],
      }),
    )
    await until(() => getsOfPlan(a) > reads)
    await until(() => node(3)?.textContent?.includes('Session timeout') === true)
  })

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
