// @vitest-environment happy-dom
//
// A run of tool calls as one line, opened into numbered steps.

import { act } from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  runningOperation,
  toolCardEntry,
  toolCardFixture,
  userEntry,
} from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, until } from '@/test/test-app'
import type { ToolCallCard } from '@/api/threads'

const drawn = vi.hoisted(() => ({ count: 0, hit() { this.count += 1 } }))

vi.mock('./tool-group', async (original) => {
  const actual = await original<typeof import('./tool-group')>()
  const { memo } = await import('react')
  const ToolGroup = memo(function ToolGroup(props: Parameters<typeof actual.ToolGroup>[0]) {
    drawn.hit()
    return <actual.ToolGroup {...props} />
  })
  return { ...actual, ToolGroup }
})

let app: TestApp | null = null
afterEach(() => {
  app?.unmount()
  app = null
  drawn.count = 0
})

const card = (id: string, extra: Partial<ToolCallCard> = {}) =>
  toolCardFixture({ id, target: `${id}.ts`, ...extra })
const read = (id: string, extra: Partial<ToolCallCard> = {}) =>
  toolCardEntry(`e-${id}`, `Read ${id}.ts`, card(id, extra))

async function open(entries: ReturnType<typeof read>[], url = '/projects/p1/threads/t1') {
  const a = (app = await startApp(
    url,
    answers({ entries: [userEntry('u1', 'go'), ...entries], operations: [runningOperation()] }),
  ))
  await until(() => a.sources.length > 0)
  act(() => a.pushFrame('caught-up', { seq: 0 }))
  await until(() => a.container.querySelector('[data-entry-kind="tool"]') !== null)
  return a
}

const summary = () => document.querySelector<HTMLButtonElement>('[data-entry-kind="tool"] button[aria-expanded]')

describe('a tool group', () => {
  it('a finished group is one closed line with its time, and opens into numbered steps', async () => {
    const a = await open([
      read('a'),
      read('b', { started_at: '2026-10-31T10:00:01Z', finished_at: '2026-10-31T10:00:04Z' }),
    ])
    expect(summary()?.textContent).toContain('Read 2 files')
    expect(summary()?.textContent).toContain('4s')
    expect(summary()?.getAttribute('aria-expanded')).toBe('false')
    expect(a.text()).not.toContain('1.')
    await act(async () => summary()?.click())
    expect(summary()?.getAttribute('aria-expanded')).toBe('true')
    expect(a.text()).toContain('1.')
    expect(a.text()).toContain('Read b.ts')
    expect(a.text()).toContain('2.')
  })

  it('a running group is open, spins, and counts its seconds', async () => {
    const started = new Date(Date.now() - 5000).toISOString()
    const a = await open([
      read('a', { started_at: started, finished_at: started }),
      read('b', { status: 'in_progress', started_at: started, finished_at: started }),
    ])
    expect(summary()?.getAttribute('aria-expanded')).toBe('true')
    expect(a.container.querySelector('.animate-spin')).not.toBeNull()
    const before = summary()?.textContent
    await until(() => summary()?.textContent !== before)
    expect(summary()?.textContent).toMatch(/\d+s/)
  })

  it("a live tool frame draws a running step in its turn's group", async () => {
    const a = await open([read('a')])
    expect(summary()).toBeNull()
    act(() =>
      a.pushFrame('tool', {
        op: 'op1',
        id: 'b',
        tool: 'Read',
        kind: 'read',
        title: 'Read b.ts',
        target: 'b.ts',
        lines: null,
        status: 'in_progress',
        started_at: new Date().toISOString(),
      }),
    )
    await until(() => summary() !== null)
    expect(a.container.querySelectorAll('[data-entry-kind="tool"]')).toHaveLength(1)
    expect(summary()?.textContent).toContain('Read 2 files')
    expect(summary()?.getAttribute('aria-expanded')).toBe('true')
    expect(a.container.querySelector('[aria-label="Running"]')).not.toBeNull()
    expect(a.text()).toContain('Read b.ts')
  })

  it('a failed step is marked failed and the line says how many', async () => {
    const a = await open([read('a'), read('b', { status: 'failed', error: 'no such file' })])
    expect(summary()?.textContent).toContain('1 failed')
    await act(async () => summary()?.click())
    expect(a.container.querySelector('[aria-label="Failed"]')).not.toBeNull()
    expect(a.text()).toContain('failed')
  })

  it('a failed step shows its error under its line, wrapped, and a long one opens with Show all', async () => {
    const long = Array.from({ length: 12 }, (_, i) => `line ${i}`).join('\n')
    const a = await open([
      read('a', { status: 'failed', error: 'short\nerror' }),
      read('b', { status: 'failed', error: long }),
    ])
    await act(async () => summary()?.click())
    const errors = [...a.container.querySelectorAll('pre')]
    expect(errors[0]?.textContent).toBe('short\nerror')
    expect(errors[0]?.className).toContain('whitespace-pre-wrap')
    expect(errors[0]?.className).not.toContain('line-clamp-6')
    expect(errors[1]?.className).toContain('line-clamp-6')
    await act(async () => a.button('Show all')?.click())
    expect(a.container.querySelectorAll('pre')[1]?.className).not.toContain('line-clamp-6')
    expect(a.button('Show less')).toBeDefined()
  })

  it('Copy on a group copies every step\'s line', async () => {
    const writes: string[] = []
    vi.spyOn(navigator, 'clipboard', 'get').mockReturnValue({
      writeText: async (t: string) => {
        writes.push(t)
      },
    } as Clipboard)
    const a = await open([read('a'), read('b')])
    await until(() => a.buttons('Copy').length === 2)
    await act(async () => a.buttons('Copy').at(-1)?.click())
    await until(() => writes.length === 1)
    expect(writes).toEqual(['Read a.ts\nRead b.ts'])
  })

  it("a link to a member's entry opens its group", async () => {
    const entries = [read('a'), read('b')]
    const a = await open(entries, `/projects/p1/threads/t1#entry-${entries[1]?.ordinal}`)
    await until(() => summary()?.getAttribute('aria-expanded') === 'true')
    expect(a.container.querySelector(`#entry-${entries[1]?.ordinal}`)).not.toBeNull()
  })

  it('draws a group once while many deltas arrive', async () => {
    const a = await open([read('a'), read('b')])
    const stream = a.sources.find((s) => s.param('thread_id') === 't1')
    if (stream === undefined) throw new Error('no stream was opened for t1')
    const before = drawn.count
    expect(before).toBeGreaterThan(0)
    for (let i = 0; i < 50; i += 1) {
      await act(async () => {
        stream.emit('delta', JSON.stringify({ op: 'op1', text: `word${i} ` }))
        await new Promise((resolve) => setTimeout(resolve, 0))
      })
    }
    expect(drawn.count).toBe(before)
  })
})

