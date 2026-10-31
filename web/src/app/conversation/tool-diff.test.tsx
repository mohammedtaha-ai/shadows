// @vitest-environment happy-dom
//
// An edit's hunks as numbered lines, and the step that opens into them.

import { act } from 'react'
import { createRoot } from 'react-dom/client'
import { afterEach, describe, expect, it } from 'vitest'
import type { DiffHunk } from '@/api/threads'
import { toolCardEntry, toolCardFixture, runningOperation, userEntry } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, until } from '@/test/test-app'
import { ToolDiff } from './tool-diff'

;(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true

const hunk = (extra: Partial<DiffHunk> = {}): DiffHunk => ({
  path: 'a.ts',
  old_start: 10,
  new_start: 10,
  lines: [
    { op: 'context', text: 'keep' },
    { op: 'remove', text: 'old' },
    { op: 'add', text: 'new' },
    { op: 'context', text: 'tail' },
  ],
  ...extra,
})

function draw(diff: DiffHunk[] | null) {
  const host = document.createElement('div')
  document.body.append(host)
  const root = createRoot(host)
  act(() => root.render(<ToolDiff diff={diff} />))
  return { host, done: () => act(() => root.unmount()) }
}

let app: TestApp | null = null
afterEach(() => {
  app?.unmount()
  app = null
  document.body.innerHTML = ''
})

const rows = (host: HTMLElement) =>
  [...host.querySelectorAll('[data-diff]')].map((row) => [
    row.getAttribute('data-diff'),
    ...[...row.querySelectorAll('span')].map((span) => span.textContent),
  ])

describe('a tool diff', () => {
  it('draws added and removed lines with their old and new numbers', () => {
    const { host, done } = draw([hunk()])
    expect(rows(host)).toEqual([
      ['context', '10', '10', ' ', 'keep'],
      ['remove', '11', '', '−', 'old'],
      ['add', '', '11', '+', 'new'],
      ['context', '12', '12', ' ', 'tail'],
    ])
    done()
  })

  it('draws no numbers when the hunk has no start', () => {
    const { host, done } = draw([hunk({ old_start: null, new_start: null })])
    expect(rows(host)[1]).toEqual(['remove', '−', 'old'])
    done()
  })

  it('marks the gap between two hunks', () => {
    const { host, done } = draw([hunk(), hunk({ old_start: 40, new_start: 41 })])
    expect(host.querySelectorAll('[aria-label="Gap"]')).toHaveLength(1)
    expect(host.textContent).toContain('⋯')
    done()
  })

  it("names the file where a hunk's path changes", () => {
    const { host, done } = draw([hunk(), hunk({ path: 'b.ts' })])
    expect(host.textContent).toContain('b.ts')
    expect(host.textContent).not.toContain('a.ts')
    expect(host.querySelectorAll('[aria-label="Gap"]')).toHaveLength(0)
    done()
  })

  it('a diff too long to show says so and keeps its counts', async () => {
    const edit = toolCardEntry(
      'e-1',
      'Edit a.ts',
      toolCardFixture({ kind: 'edit', tool: 'Edit', target: 'a.ts', added: 30, removed: 6, diff: null }),
    )
    const a = (app = await startApp(
      '/projects/p1/threads/t1',
      answers({ entries: [userEntry('u1', 'go'), edit], operations: [runningOperation()] }),
    ))
    await until(() => a.container.querySelector('[data-entry-kind="tool"]') !== null)
    expect(a.text()).toContain('+30')
    await act(async () =>
      a.container.querySelector<HTMLButtonElement>('button[aria-label="Show diff"]')?.click(),
    )
    expect(a.text()).toContain('Diff too long to show')
    expect(a.text()).toContain('+30')
  })

  it('an edit step without a diff has no chevron', async () => {
    const edit = toolCardEntry(
      'e-1',
      'Edit a.ts',
      toolCardFixture({ kind: 'edit', tool: 'Edit', target: 'a.ts', diff: null }),
    )
    const a = (app = await startApp(
      '/projects/p1/threads/t1',
      answers({ entries: [userEntry('u1', 'go'), edit], operations: [runningOperation()] }),
    ))
    await until(() => a.container.querySelector('[data-entry-kind="tool"]') !== null)
    expect(a.container.querySelector('button[aria-label="Show diff"]')).toBeNull()
  })
})
