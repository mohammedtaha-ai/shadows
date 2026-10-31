// @vitest-environment happy-dom
//
// A reply streamed into a long thread: the thread's entries (a plan's graph,
// long markdown) are drawn once, not again for every delta.

import { act } from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { ThreadEntry } from '@/api/threads'
import {
  agentEntry,
  entryOfKind,
  planFixture,
  planViewEntry,
  runningOperation,
  toolCardEntry,
  toolCardFixture,
  userEntry,
} from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, until } from '@/test/test-app'

const drawn = vi.hoisted(() => new Map<string, number>())

vi.mock('./entry', async (original) => {
  const actual = await original<typeof import('./entry')>()
  function Entry(props: Parameters<typeof actual.Entry>[0]) {
    drawn.set(props.entry.id, (drawn.get(props.entry.id) ?? 0) + 1)
    return <actual.Entry {...props} />
  }
  return { ...actual, Entry }
})

let open: TestApp | null = null
afterEach(() => {
  open?.unmount()
  open = null
  drawn.clear()
})

const LONG = Array.from(
  { length: 40 },
  (_, i) => `## Part ${i}\n\nA paragraph with **bold**, \`code\` and a [link](https://example.com).\n\n- one\n- two`,
).join('\n\n')

describe('a streamed reply', () => {
  it('draws the plan card and the long markdown once while many deltas arrive', async () => {
    const entries = [
      userEntry('u1', 'show me the plan'),
      planViewEntry('v1', 'Plan v1', 'w1'),
      entryOfKind('a1', 'AgentMessage', LONG),
      userEntry('u2', 'and now?'),
    ]
    open = await startApp(
      '/projects/p1/threads/t1',
      answers({ entries, plan: planFixture(), operations: [runningOperation()] }),
    )
    const app = open
    await until(() => app.container.querySelector('[data-plan-card="w1"] .react-flow__node') !== null)
    await until(() => app.sources.some((s) => s.param('thread_id') === 't1'))
    const stream = app.sources.find((s) => s.param('thread_id') === 't1')
    if (stream === undefined) throw new Error('no stream was opened for t1')
    await act(async () => stream.caughtUp(0))
    await until(() => app.container.textContent?.includes('Thinking') === true)

    const before = { plan: drawn.get('v1'), markdown: drawn.get('a1') }
    for (let i = 0; i < 100; i += 1) {
      await act(async () => {
        stream.emit('delta', JSON.stringify({ op: 'op1', text: `word${i} ` }))
        await new Promise((resolve) => setTimeout(resolve, 0))
      })
    }
    await until(() => app.container.textContent?.includes('word99') === true)

    expect(before.plan).toBeGreaterThan(0)
    expect(drawn.get('v1')).toBe(before.plan)
    expect(drawn.get('a1')).toBe(before.markdown)
  })

  it('a written tool call stays between the paragraphs while the turn streams on', async () => {
    const list: ThreadEntry[] = [userEntry('u1', 'read the plan')]
    open = await startApp('/projects/p1/threads/t1', {
      ...answers({ operations: [runningOperation()] }),
      'GET /api/threads/t1/entries': () => Response.json(list),
    })
    const app = open
    await until(() => app.sources.some((s) => s.param('thread_id') === 't1'))
    const stream = app.sources.find((s) => s.param('thread_id') === 't1')
    if (stream === undefined) throw new Error('no stream was opened for t1')
    await act(async () => stream.caughtUp(0))
    const delta = (text: string) =>
      act(async () => stream.emit('delta', JSON.stringify({ op: 'op1', text })))
    // Core writes the message, then the call, each announced as it lands.
    const write = (seq: number, entry: ThreadEntry) =>
      act(async () => {
        list.push(entry)
        stream.durable(seq, 'ThreadEntryAppended', 'op1', { ordinal: entry.ordinal, kind: entry.kind })
      })
    const group = () => app.container.querySelector('[data-entry-kind="tool"]')

    await delta('First paragraph.')
    await write(1, agentEntry('a1', 'First paragraph.'))
    await act(async () =>
      stream.emit('tool', JSON.stringify({
        op: 'op1', id: 'b', tool: 'Read', kind: 'read', title: 'Read b.ts', target: 'b.ts',
        lines: null, status: 'completed', started_at: '2026-10-31T10:00:00Z',
      })),
    )
    await write(2, toolCardEntry('e-b', 'Read b.ts', toolCardFixture({ id: 'b', target: 'b.ts' })))
    await until(() => group() !== null)
    await delta('Second paragraph.')
    await until(() => app.text().includes('Second paragraph.'))
    // Give the list every chance to refetch and redraw.
    await act(async () => new Promise((resolve) => setTimeout(resolve, 50)))

    const tool = group()
    expect(tool).not.toBeNull()
    expect(tool?.textContent).toContain('Read b.ts')
    const first = [...app.container.querySelectorAll('li')].find((li) => li.textContent?.includes('First paragraph.'))
    const second = [...app.container.querySelectorAll('li')].find((li) => li.textContent?.includes('Second paragraph.'))
    if (tool === null || first === undefined || second === undefined) throw new Error('a row is missing')
    expect(first.compareDocumentPosition(tool) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
    expect(tool.compareDocumentPosition(second) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
    expect(first).not.toBe(second)
  })
})
