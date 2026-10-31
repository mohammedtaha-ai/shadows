// @vitest-environment happy-dom
//
// The agent's checklist: drawn from a live list, and from its entry.

import { act } from 'react'
import { createRoot } from 'react-dom/client'
import { afterEach, describe, expect, it } from 'vitest'
import type { TodoItem } from '@/stream'
import { entryOfKind, runningOperation, userEntry } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, until } from '@/test/test-app'
import { TodoCard } from './todo-card'
import { Entry } from './entry'
import { copyText } from './tool-text'

;(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true

const items: TodoItem[] = [
  { content: 'Read the code', status: 'completed' },
  { content: 'Write the fix', status: 'in_progress' },
  { content: 'Run tests', status: 'pending' },
]

function draw(node: React.ReactNode) {
  const host = document.createElement('div')
  document.body.append(host)
  const root = createRoot(host)
  act(() => root.render(node))
  return { host }
}

const entry = (card: Record<string, unknown> | null) => ({
  ...entryOfKind('t1', 'TodoList', '- [x] Read the code', 'op1'),
  card,
})
const policy = { initial_mode: null, unattended_mode: null } as never
const drawEntry = (card: Record<string, unknown> | null) =>
  draw(
    <Entry entry={entry(card)} policy={policy} requestedMode={null} projectId="p"
      onPointAt={() => {}} onOpenSubagent={() => {}} />,
  )

let app: TestApp | null = null
afterEach(() => {
  app?.unmount()
  app = null
})

describe('TodoCard', () => {
  it('once the TodoList entry is in the list, the live card goes and the entry shows it, never both', async () => {
    const list = { ...entryOfKind('t1', 'TodoList', 'x', 'op1'), card: { entries: items } }
    const a = (app = await startApp(
      '/projects/p1/threads/t1',
      answers({ entries: [userEntry('u1', 'go')], operations: [runningOperation()] }),
    ))
    await until(() => a.sources.length > 0)
    act(() => a.pushFrame('caught-up', { seq: 0 }))
    act(() => a.pushFrame('todos', { op: 'op1', entries: items }))
    await until(() => a.container.querySelectorAll('[data-todo-card]').length === 1)
    expect(a.container.querySelector('[data-entry-kind="todos"]')).not.toBeNull()
    app.unmount()
    app = await startApp(
      '/projects/p1/threads/t1',
      answers({ entries: [userEntry('u1', 'go'), list], operations: [runningOperation()] }),
    )
    await until(() => app!.sources.length > 0)
    act(() => app!.pushFrame('caught-up', { seq: 0 }))
    act(() => app!.pushFrame('todos', { op: 'op1', entries: items }))
    await until(() => app!.container.querySelector('[data-entry-kind="TodoList"]') !== null)
    expect(app.container.querySelectorAll('[data-todo-card]')).toHaveLength(1)
    expect(app.container.querySelector('[data-entry-kind="todos"]')).toBeNull()
  })

  it('draws done, in progress and pending items with their marks and the count', () => {
    const { host } = draw(<TodoCard items={items} />)
    expect(host.textContent).toContain('Tasks · 1 of 3 done')
    const states = [...host.querySelectorAll('li')].map((li) => li.getAttribute('data-status'))
    expect(states).toEqual(['completed', 'in_progress', 'pending'])
    expect(host.querySelectorAll('svg')).toHaveLength(3)
  })

  it('a live list updates with each todos frame', async () => {
    const a = (app = await startApp(
      '/projects/p1/threads/t1',
      answers({ entries: [userEntry('u1', 'go')], operations: [runningOperation()] }),
    ))
    await until(() => a.sources.length > 0)
    act(() => a.pushFrame('caught-up', { seq: 0 }))
    act(() => a.pushFrame('todos', { op: 'op1', entries: items }))
    await until(() => a.text().includes('1 of 3 done'))
    const done = items.map((item) => ({ ...item, status: 'completed' as const }))
    act(() => a.pushFrame('todos', { op: 'op1', entries: done }))
    await until(() => a.text().includes('3 of 3 done'))
    expect(a.container.querySelectorAll('[data-todo-card]')).toHaveLength(1)
  })

  it('after a reload the TodoList entry draws the checklist', () => {
    const { host } = drawEntry({ entries: items })
    expect(host.textContent).toContain('Write the fix')
    expect(host.textContent).toContain('1 of 3 done')
  })

  it('an unreadable card shows its body', () => {
    const { host } = drawEntry({ entries: 'nope' })
    expect(host.textContent).toContain('- [x] Read the code')
    expect(host.querySelector('[data-todo-card]')).toBeNull()
  })

  it('Copy on a checklist copies its Markdown list', () => {
    expect(copyText(entry({ entries: items }) as never)).toBe('- [x] Read the code')
  })
})
