// @vitest-environment happy-dom
//
// The plan inside the conversation (§13.9, §13.11) over a faked daemon: a
// `PlanView` card draws its version's graph; clicking a task points the next
// turn at it; a live `plan-show` frame moves only the tab that sent the turn,
// and a replayed card moves nothing; Shadows' own tool calls read as sentences.

import { act } from 'react'
import { afterEach, describe, expect, it } from 'vitest'
import type { ThreadEntry } from '@/api/client'
import {
  agentEntry,
  entryOfKind,
  planFixture,
  planViewEntry,
  userEntry,
} from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, typeInto, until } from '../test-app'

const CONVERSATION = '/projects/p1/threads/t1'
const PANEL = 'aside[aria-label="Plan beside the conversation"]'

let open: TestApp | null = null
afterEach(() => {
  open?.unmount()
  open = null
})

async function start(entries: ThreadEntry[], extra: Parameters<typeof answers>[0] = {}) {
  open = await startApp(CONVERSATION, answers({ entries, ...extra }))
  return open
}

const shown = () => [userEntry('u1', 'show me the plan'), planViewEntry('v1', 'Plan v1', 'w1')]

/** The card of plan version `w1`. */
function card(app: TestApp): HTMLElement | null {
  return app.container.querySelector<HTMLElement>('[data-plan-card="w1"]')
}

/** Task `T{n}`'s node inside `scope`. */
function node(scope: ParentNode | null, n: number): HTMLElement | null {
  return scope?.querySelector<HTMLElement>(`.react-flow__node[data-id="t${n}"]`) ?? null
}

/** The session answered and the thread's turns are known: Send can act. */
const ready = (app: TestApp) => () =>
  app.button('fake-large') !== undefined && app.calls.includes('GET /api/threads/t1/operations')

function textarea(app: TestApp): HTMLTextAreaElement {
  const box = app.container.querySelector('textarea')
  if (box === null) throw new Error('no message box')
  return box
}

const turnStarts = (app: TestApp) =>
  app.calls.filter((c) => c === 'POST /api/threads/t1/turns').length

interface TurnBody {
  command_id: string
  prompt: string
  client_tab?: string | null
  focus?: { workflow_id: string; task_id: string; revision: number } | null
}

/** Sends `text` and answers the body the daemon received. */
async function send(app: TestApp, text: string): Promise<TurnBody> {
  const before = turnStarts(app)
  typeInto(textarea(app), text)
  await act(async () => app.button('Send')?.click())
  await until(() => turnStarts(app) === before + 1)
  // The start answered: the prompt clears.
  await until(() => textarea(app).value === '')
  return app.bodies.at(-1) as TurnBody
}

/** The conversation's stream, replayed to its end so frames arrive live. */
async function liveStream(app: TestApp) {
  await until(() => app.sources.length > 0)
  const stream = app.sources.find((s) => s.param('thread_id') === 't1')
  if (stream === undefined) throw new Error('no stream was opened for t1')
  await act(async () => stream.caughtUp(0))
  return stream
}

function planShow(
  targetTab: string | null,
  place: 'inline' | 'side' | 'page',
  taskNumber: number | null = null,
) {
  return {
    thread_id: 't1',
    target_tab: targetTab,
    workflow_id: 'w1',
    version: 1,
    task_number: taskNumber,
    place,
  }
}

describe('the plan in the conversation', () => {
  it('a PlanView entry renders the graph of its version', async () => {
    const a = await start(shown(), { plan: planFixture({ revision: 3 }) })
    await until(() => node(card(a), 2) !== null)

    expect(a.calls).toContain('GET /api/workflows/w1')
    expect(card(a)?.textContent).toContain('Plan v1 · revision 3')
    expect(node(card(a), 1)?.textContent).toContain('Schema')
    const openPlan = [...(card(a)?.querySelectorAll('a') ?? [])].find(
      (l) => l.textContent?.trim() === 'Open plan',
    )
    expect(openPlan?.getAttribute('href')).toBe('/projects/p1/workflows/w1')
    // A compact card: no minimap.
    expect(card(a)?.querySelector('.react-flow__minimap')).toBeNull()
    for (const text of card(a)?.querySelectorAll('header p, header span, header h2') ?? []) {
      expect(text.getAttribute('dir'), text.outerHTML).toBe('auto')
    }
  })

  it('clicking a task adds the focus chip and the next send carries it', async () => {
    const a = await start(shown(), { plan: planFixture({ revision: 3 }) })
    await until(() => node(card(a), 2) !== null)
    await until(ready(a))

    await act(async () => node(card(a), 2)?.click())
    const chip = () => a.container.querySelector<HTMLElement>('[data-focus-chip]')
    await until(() => chip() !== null)
    expect(chip()?.textContent).toContain('T2 · Login screen')
    const title = [...(chip()?.querySelectorAll('span') ?? [])].find(
      (s) => s.textContent === 'Login screen',
    )
    expect(title?.getAttribute('dir')).toBe('auto')
    expect(a.button('Remove focus')).toBeDefined()

    const first = await send(a, 'change this')
    expect(first.focus).toEqual({ workflow_id: 'w1', task_id: 'task-2', revision: 3 })
    expect(typeof first.client_tab).toBe('string')
    // Sent: the chip is gone, and the turn after carries no focus.
    await until(() => chip() === null)
    const stream = await liveStream(a)
    await act(async () => stream.durable(1, 'OperationCompleted', 'op1', {}))
    await until(() => a.button('Send') !== undefined)
    const second = await send(a, 'thanks')
    expect(second.focus ?? null).toBeNull()
    // One tab, one id, on every turn.
    expect(second.client_tab).toBe(first.client_tab)
  })

  it('a plan-show frame for this tab and page navigates', async () => {
    const a = await start(shown())
    await until(ready(a))
    const { client_tab: tab } = await send(a, 'open it on its page')
    const stream = await liveStream(a)

    await act(async () => stream.emit('plan-show', JSON.stringify(planShow(tab ?? null, 'page'))))
    await until(() => a.path() === '/projects/p1/workflows/w1')
  })

  it('a plan-show frame for this tab and side opens the panel on that version', async () => {
    const a = await start(shown())
    await until(ready(a))
    const { client_tab: tab } = await send(a, 'open it on the side')
    const stream = await liveStream(a)

    await act(async () =>
      stream.emit('plan-show', JSON.stringify(planShow(tab ?? null, 'side', 2))),
    )
    const panel = () => a.container.querySelector<HTMLElement>(PANEL)
    await until(() => node(panel(), 2) !== null)
    expect(panel()?.textContent).toContain('Plan v1 · revision 3')
    expect(a.path()).toBe(CONVERSATION)

    await act(async () => a.button('Close plan')?.click())
    await until(() => panel() === null)
  })

  it('a plan-show frame for another tab only shows the card', async () => {
    const a = await start(shown())
    await until(() => node(card(a), 2) !== null)
    await until(ready(a))
    await send(a, 'open it on its page')
    const stream = await liveStream(a)

    await act(async () => {
      stream.emit('plan-show', JSON.stringify(planShow('another-tab', 'page')))
      stream.emit('plan-show', JSON.stringify(planShow('another-tab', 'side')))
      stream.emit('plan-show', JSON.stringify(planShow(null, 'side')))
    })
    // Give a navigation or a panel every chance to happen.
    await act(async () => new Promise((resolve) => setTimeout(resolve, 50)))
    expect(a.path()).toBe(CONVERSATION)
    expect(a.container.querySelector(PANEL)).toBeNull()
    expect(card(a)).not.toBeNull()
  })

  it('a replayed PlanView entry does not open the side panel', async () => {
    const a = await start(shown())
    await until(() => a.sources.length > 0)
    const stream = a.sources.find((s) => s.param('thread_id') === 't1')
    if (stream === undefined) throw new Error('no stream was opened for t1')

    // The card's history, as a reopened conversation replays it.
    await act(async () => {
      stream.durable(1, 'ThreadEntryAppended', 'op1', { ordinal: 2, kind: 'PlanView' })
      stream.durable(2, 'PlanShown', 'op1', {
        workflow_id: 'w1',
        version: 1,
        task_number: null,
        place: 'side',
        entry_id: 'v1',
      })
      stream.caughtUp(2)
    })
    await until(() => node(card(a), 2) !== null)
    await act(async () => new Promise((resolve) => setTimeout(resolve, 50)))
    expect(a.container.querySelector(PANEL)).toBeNull()
    expect(a.path()).toBe(CONVERSATION)
  })

  it('plan tool lines read as sentences', async () => {
    const a = await start([
      userEntry('u1', 'plan the login'),
      agentEntry('a1', '[tool: mcp__shadows__draft_start]'),
      agentEntry('a2', '[tool: mcp__shadows__plan_edit]'),
      agentEntry('a3', '[tool: mcp__shadows__workflow_get]'),
      agentEntry('a4', '[tool: mcp__shadows__plan_show]'),
      planViewEntry('v1', 'Plan v1', 'w1'),
      agentEntry('a5', '[tool: Read notes.md]'),
      entryOfKind('ap', 'PlanApproved', 'Plan v1 approved', null),
    ])
    await until(() => a.text().includes('Plan v1 approved'))

    const text = a.container.textContent ?? ''
    expect(text).toContain('Plan started')
    expect(text).toContain('Plan edited')
    expect(text).toContain('Read the plan')
    // Shadows' own tool names are not shown; another tool's title still is.
    expect(text).not.toContain('mcp__shadows')
    expect(text).toContain('Read notes.md')
    // `plan_show` adds no line of its own: its card is the entry.
    const lines = [...a.container.querySelectorAll('ol > li')].map((li) => li.textContent ?? '')
    expect(
      lines.filter(
        (l) =>
          l.includes('Plan started') || l.includes('Plan edited') || l.includes('Read the plan'),
      ),
    ).toHaveLength(3)
    expect(a.container.querySelectorAll('ol > li[data-entry-kind="tool"]')).toHaveLength(4)

    const approved = [...a.container.querySelectorAll('[data-entry-kind="PlanApproved"]')]
    expect(approved).toHaveLength(1)
    expect(approved[0]?.querySelector('[dir="auto"]')?.textContent).toBe('Plan v1 approved')
  })

  it('a conversation with a plan card holds one stream for its thread', async () => {
    const a = await start(shown())
    await until(() => node(card(a), 2) !== null)
    await act(async () => new Promise((resolve) => setTimeout(resolve, 20)))

    const open = a.sources.filter((s) => s.param('thread_id') === 't1' && !s.closed)
    expect(open).toHaveLength(1)

    // And that one stream keeps the card current.
    const stream = await liveStream(a)
    const reads = a.calls.filter((c) => c === 'GET /api/workflows/w1').length
    await act(async () =>
      stream.durable(3, 'WorkflowEdited', null, {
        workflow_id: 'w1',
        version: 1,
        revision: 4,
        summary: 'Renamed T2',
        changed_tasks: [2],
      }),
    )
    await until(() => a.calls.filter((c) => c === 'GET /api/workflows/w1').length > reads)
  })
})
