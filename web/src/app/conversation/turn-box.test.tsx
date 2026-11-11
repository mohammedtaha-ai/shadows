// @vitest-environment happy-dom
//
// A run of a Planner turn's notices and tools as one box: folded once it ends, open while it
// runs; the Planner's own text is drawn outside it and is not counted in the summary.

import { RouterProvider, createMemoryHistory, createRootRoute, createRouter } from '@tanstack/react-router'
import { act, useEffect, useState } from 'react'
import { createRoot } from 'react-dom/client'
import { afterEach, expect, it } from 'vitest'
import type { AttemptNotice } from '@/api/runs'
import type { ThreadEntry } from '@/api/threads'
import { agentEntry, entryOfKind, toolEntry } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, until } from '@/test/test-app'
import { TurnBox } from './turn-box'

let open: TestApp | null = null
afterEach(() => {
  open?.unmount()
  open = null
})

const card: AttemptNotice = {
  workflow_id: 'w1', version: 1, task: 2, title: 'Login', attempt: 3, attempt_id: 'a3', model: 'sonnet',
  outcome: 'committed', commit: null, report: null, reason: null, last_addition: false,
}

it('a_finished_turn_shows_one_summary_line_and_opens_on_click', async () => {
  const notice: ThreadEntry = {
    ...entryOfKind('n1', 'UserMessage', 'instructions'),
    author: { kind: 'Executor', id: 'x1' },
    card: card as unknown as ThreadEntry['card'],
  }
  const tools = ['one', 'two', 'three', 'four', 'five'].map((word) => toolEntry(`t-${word}`, `Read ${word}.ts`))
  const a = (open = await startApp('/projects/p1/threads/t1', answers({ entries: [notice, ...tools] })))
  const summary = 'Received a notice from Executor (sonnet), ran 5 tools'
  await until(() => a.button(summary) !== undefined)
  expect(a.text()).not.toContain('Read one.ts')
  await act(async () => a.button(summary)?.click())
  await until(() => a.text().includes('Read one.ts'))
  expect(a.text()).toContain('Read five.ts')
})

it('the_planner_text_between_tools_stays_outside_and_splits_the_box', async () => {
  const notice: ThreadEntry = {
    ...entryOfKind('n1', 'UserMessage', 'instructions'),
    author: { kind: 'Executor', id: 'x1' },
    card: card as unknown as ThreadEntry['card'],
  }
  const t1 = toolEntry('t-one', 'Read one.ts')
  const note = agentEntry('a-note', 'Let me check the second file.')
  const t2 = toolEntry('t-two', 'Read two.ts')
  const t3 = toolEntry('t-three', 'Read three.ts')
  const a = (open = await startApp('/projects/p1/threads/t1', answers({ entries: [notice, t1, note, t2, t3] })))
  await until(() => a.button('Received a notice from Executor (sonnet), ran 1 tool') !== undefined)
  expect(a.button('Ran 2 tools')).toBeDefined()
  expect(a.text()).toContain('Let me check the second file.')
  expect(a.text()).not.toContain('wrote')
  expect(a.text()).not.toContain('Read one.ts')
})

it('a_running_turn_is_open_and_folds_when_it_ends', async () => {
  let setLive: (live: boolean) => void = () => {}
  function Harness() {
    const [live, set] = useState(true)
    useEffect(() => { setLive = set }, [set])
    return (
      <TurnBox summary={{ notices: [], tools: 1 }} live={live} ordinals={[]}>
        <li>inside text</li>
      </TurnBox>
    )
  }
  const router = createRouter({
    routeTree: createRootRoute({ component: Harness }),
    history: createMemoryHistory({ initialEntries: ['/'] }),
  })
  const host = document.createElement('div')
  document.body.append(host)
  await act(async () => { createRoot(host).render(<RouterProvider router={router} />) })
  await until(() => host.querySelector('button') !== null)
  expect(host.textContent).toContain('inside text')
  expect(host.textContent).toContain('Ran 1 tool')
  await act(async () => { setLive(false) })
  expect(host.textContent).not.toContain('inside text')
  host.remove()
})
