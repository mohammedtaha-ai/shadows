// @vitest-environment happy-dom
//
// Subagent cards (§22.4) over a faked daemon: drawn from a `[subagent: …]`
// entry and from a live `subagent` frame; a click opens the side panel.

import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { agentEntry, userEntry } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, until } from '../test-app'
import { modelName, numbers } from './subagent-text'

const CARD = {
  id: 'a1',
  title: 'List and read folder files',
  agent_type: 'general-purpose',
  model: 'claude-sonnet-5-5',
  status: 'completed',
  prompt: 'List the files and read each one.',
  steps: ['Find *', 'Read alpha.txt'],
  report: 'Two files: alpha and beta.',
  duration_ms: 252000,
  tokens: 43119,
  tool_count: 4,
}

let app: TestApp | null = null
afterEach(() => {
  app?.unmount()
  app = null
})

async function open(entries = [userEntry('u1', 'go'), agentEntry('e1', `[subagent: ${JSON.stringify(CARD)}]`)]) {
  const a = (app = await startApp('/projects/p1/threads/t1', answers({ entries })))
  await until(() => a.sources.length > 0)
  act(() => a.pushFrame('caught-up', { seq: 0 }))
  return a
}

const card = (id: string) => document.querySelector<HTMLButtonElement>(`[data-subagent="${id}"]`)

it('an entry is drawn as a card with its task, model and numbers', async () => {
  const a = await open()
  await until(() => card('a1') !== null)
  const text = card('a1')?.textContent ?? ''
  expect(text).toContain('List and read folder files')
  expect(text).toContain('general-purpose · Sonnet 5.5')
  expect(text).toContain('4m 12s · 43k tokens · 4 tools')
  expect(a.text()).not.toContain('[subagent:')
})

it('a click opens what it did beside the conversation', async () => {
  const a = await open()
  await until(() => card('a1') !== null)
  act(() => card('a1')?.click())
  await until(() => document.querySelector('[aria-label="Subagent beside the conversation"]') !== null)
  const panel = document.querySelector('[aria-label="Subagent beside the conversation"]')
  expect(panel?.textContent).toContain('List the files and read each one.')
  expect(panel?.textContent).toContain('Read alpha.txt')
  await until(() => (panel?.textContent ?? '').includes('Two files'))
  act(() => a.button('Close subagent')?.click())
  expect(document.querySelector('[aria-label="Subagent beside the conversation"]')).toBeNull()
})

it('a live frame draws a running card until its entry is in the list', async () => {
  const a = await open([userEntry('u1', 'go')])
  act(() =>
    a.pushFrame('subagent', {
      op: 'op1',
      card: { ...CARD, id: 'live', status: 'running', steps: ['Find *'], report: null,
        duration_ms: null, tokens: null, tool_count: null },
    }),
  )
  await until(() => card('live') !== null)
  expect(card('live')?.getAttribute('data-status')).toBe('running')
  expect(card('live')?.textContent).toContain('1 tool')
})

it('names a model the way the pickers do', () => {
  expect(modelName('claude-opus-5-5')).toBe('Opus 5.5')
  expect(modelName('sonnet')).toBe('Sonnet')
  const bare = { ...CARD, agentType: null, durationMs: null, tokens: null, toolCount: null, steps: [] }
  expect(numbers({ ...bare, status: 'running', model: null, prompt: null, report: null })).toBe('')
})
