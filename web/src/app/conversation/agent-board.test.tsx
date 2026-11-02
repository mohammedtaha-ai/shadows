// @vitest-environment happy-dom
//
// Agent Mode: the switch shows the board and back; the board lists the real
// steps and task list, counted, never as a percent; the stages are planned.

import { act } from 'react'
import { createRoot } from 'react-dom/client'
import { afterEach, describe, expect, it } from 'vitest'
import type { TodoItem } from '@/stream'
import { entryOfKind, toolCardEntry, toolCardFixture, userEntry } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, until } from '@/test/test-app'
import { AgentBoard } from './agent-board'

;(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true

const items: TodoItem[] = [
  { content: 'Read the code', status: 'completed' },
  { content: 'Write the fix', status: 'in_progress' },
]

const props = {
  title: 'Fix it',
  projectName: 'Demo',
  agentLabel: 'Claude Code',
  modelLabel: 'Sonnet',
  tools: {},
  todos: {},
  subagents: [],
  running: null,
  latest: null,
  thinking: false,
  label: undefined,
  context: null,
  onOpenSubagent: () => {},
}

function draw(node: React.ReactNode) {
  const host = document.createElement('div')
  document.body.append(host)
  const root = createRoot(host)
  act(() => root.render(node))
  return host
}

let app: TestApp | null = null
afterEach(() => {
  app?.unmount()
  app = null
  document.body.replaceChildren()
})

describe('AgentBoard', () => {
  it('lists the real steps and the task list, with a count of finished steps and no percent', () => {
    const host = draw(
      <AgentBoard
        {...props}
        entries={[
          userEntry('u1', 'Fix the bug'),
          toolCardEntry('c1', 'Read a.ts', toolCardFixture({ id: 'a', target: 'a.ts' })),
          toolCardEntry('c2', 'npm test', toolCardFixture({ id: 'b', kind: 'execute', status: 'failed', target: null })),
          { ...entryOfKind('l1', 'TodoList', 'x', 'op1'), card: { entries: items } },
        ]}
      />,
    )
    expect(host.textContent).toContain('Fix the bug')
    expect(host.textContent).toContain('1 of 2 steps done')
    expect(host.querySelector('[aria-label="Agent execution"]')?.textContent).not.toMatch(/\d+%/)
    expect(host.querySelectorAll('[aria-label="Agent execution"] li').length).toBe(2)
    expect(host.textContent).toContain('Tasks · 1 of 2 done')
  })

  it('with no steps shows an empty state and no percent', () => {
    const host = draw(<AgentBoard {...props} entries={[userEntry('u1', 'Hello')]} />)
    expect(host.textContent).toContain('No tool steps in this conversation yet')
    expect(host.textContent).not.toMatch(/\d+%/)
  })

  it('marks the stepper planned and inert', () => {
    const host = draw(<AgentBoard {...props} entries={[]} />)
    const planned = host.querySelector('[aria-label="Run stages (planned)"]')
    expect(planned?.textContent).toContain('Planned')
    expect(planned?.querySelector('[inert]')).not.toBeNull()
    expect(planned?.querySelector('[aria-label*="Done"]')).toBeNull()
  })
})

describe('the Agent Mode switch', () => {
  it('shows the board and switches back to the messages', async () => {
    const a = (app = await startApp(
      '/projects/p1/threads/t1',
      answers({ entries: [userEntry('u1', 'Plan the thing')] }),
    ))
    const mode = (name: string) =>
      [...a.container.querySelectorAll<HTMLButtonElement>('[aria-label="Mode"] button')].find((b) =>
        b.textContent?.includes(name),
      )
    await until(() => mode('Agent Mode') !== undefined)
    expect(mode('Chat Mode')?.getAttribute('aria-pressed')).toBe('true')
    await act(async () => mode('Agent Mode')?.click())
    await until(() => a.container.querySelector('[aria-label="Agent execution"]') !== null)
    expect(mode('Agent Mode')?.getAttribute('aria-pressed')).toBe('true')
    await act(async () => mode('Chat Mode')?.click())
    await until(() => a.container.querySelector('[aria-label="Agent execution"]') === null)
    expect(a.container.textContent).toContain('Plan the thing')
  })
})
