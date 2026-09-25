// @vitest-environment happy-dom
//
// What a person can do with a message (spec §12.9, §12.11): copy any of them,
// fork from the last one while nothing runs; and how a refused permission and
// a tool call read in the conversation.

import { act } from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  agentEntry,
  completedOperation,
  entryOfKind,
  runningOperation,
  threadFixture,
  userEntry,
} from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, until } from '../test-app'

let open: TestApp | null = null
afterEach(() => {
  open?.unmount()
  open = null
  vi.restoreAllMocks()
})

async function start(table: ReturnType<typeof answers>): Promise<TestApp> {
  open = await startApp('/projects/p1/threads/t1', table)
  return open
}

describe('message actions', () => {
  it('copy is on every message and copies its text', async () => {
    const writes: string[] = []
    // Only the clipboard: the router reads the rest of `navigator`.
    vi.spyOn(navigator, 'clipboard', 'get').mockReturnValue({
      writeText: async (t: string) => {
        writes.push(t)
      },
    } as Clipboard)
    const app = await start(answers({ entries: [userEntry('u1', 'hi'), agentEntry('a1', 'hello')] }))
    await until(() => app.buttons('Copy').length === 2)
    // Async act: the copy answers a promise later, and its "Copied" must land
    // inside act, not in the tick after a check that was already true.
    await act(async () => app.buttons('Copy')[1]?.click())
    await until(() => writes.length === 1)
    expect(writes).toEqual(['hello'])
    await until(() => app.buttons('Copied').length === 1)
  })

  it('fork shows only on the last message of an idle thread and opens the fork', async () => {
    const app = await start(
      answers({
        entries: [userEntry('u1', 'hi'), agentEntry('a1', 'hello')],
        operations: [completedOperation(null)],
        fork: () => Response.json({ ...threadFixture, id: 't9' }, { status: 201 }),
      }),
    )
    await until(() => app.buttons('Fork').length === 1)
    act(() => app.buttons('Fork')[0]?.click())
    await until(() => app.calls.includes('POST /api/threads/t1/fork'))
    expect(app.bodies.at(-1)).toMatchObject({ at_entry_id: 'a1' })
    await until(() => app.path() === '/projects/p1/threads/t9')
  })

  it('fork is absent while a turn runs', async () => {
    const app = await start(answers({ operations: [runningOperation()] }))
    await until(() => app.buttons('Copy').length > 0 && app.button('Stop') !== undefined)
    expect(app.buttons('Fork')).toHaveLength(0)
  })

  it('a refused fork shows the daemon message and stays', async () => {
    const app = await start(
      answers({
        operations: [completedOperation(null)],
        fork: () =>
          Response.json(
            { code: 'FORK_POINT_NOT_SUPPORTED', message: 'fork from the last completed message' },
            { status: 422 },
          ),
      }),
    )
    await until(() => app.buttons('Fork').length === 1)
    act(() => app.buttons('Fork')[0]?.click())
    await until(() => app.text().includes('fork from the last completed message'))
    expect(app.path()).toBe('/projects/p1/threads/t1')
  })
})

describe('entries that are not messages', () => {
  it('a refused permission renders as a quiet line naming what was asked', async () => {
    const app = await start(
      answers({ entries: [entryOfKind('r1', 'PermissionRefused', 'Run echo probe')] }),
    )
    await until(() => app.text().includes('Run echo probe'))
    expect(app.text()).toContain('refused in Accept edits')
    expect(app.text()).toContain('Auto would allow it')
  })

  it('a tool call reads as a tool line, not as bracketed text', async () => {
    const app = await start(answers({ entries: [agentEntry('a1', '[tool: npm test]')] }))
    await until(() => app.text().includes('npm test'))
    expect(app.text()).not.toContain('[tool:')
  })
})
