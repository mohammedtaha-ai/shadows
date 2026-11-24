// @vitest-environment happy-dom
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { act } from 'react'
import { createRoot } from 'react-dom/client'
import { afterEach, expect, it, vi } from 'vitest'
import type { LastRecheck, RecheckRow } from '@/api/runs'
import { RecheckPanel } from './recheck-panel'

;(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true

afterEach(() => {
  document.body.innerHTML = ''
  vi.unstubAllGlobals()
})

const row = (task: number, extra: Partial<RecheckRow> = {}): RecheckRow => ({
  workflow_id: 'w1', version: 1, task, attempt_id: `a${task}`, check_id: 'c', command: `cargo test t${task}`,
  outcome: null, started_at: null, finished_at: null, output: null, truncated: false, ...extra,
})
const last: LastRecheck = {
  recheck_id: 'r1', began: { kind: 'person' }, created_at: '2026-11-24T09:00:00Z', finished_at: null,
  counts: { passed: 1, failed: 1, timed_out: 0, inconclusive: 0, not_run: 0, unfinished: 1 },
}
const rows = [
  row(1, { outcome: 'passed', started_at: 'x', output: 'stored one' }),
  row(2, { outcome: 'failed', started_at: 'x', output: 'stored two' }),
  row(3, { started_at: 'x' }),
]

class FakeSource {
  static last: FakeSource | undefined
  listeners = new Map<string, (event: Event) => void>()
  constructor() { FakeSource.last = this }
  addEventListener(type: string, fn: (event: Event) => void) { this.listeners.set(type, fn) }
  close() { /* nothing */ }
  send(type: string, data: object) { this.listeners.get(type)?.(new MessageEvent(type, { data: JSON.stringify(data) })) }
}

async function draw(onClose = () => { /* closed */ }) {
  vi.stubGlobal('EventSource', FakeSource)
  vi.stubGlobal('fetch', async (request: Request) =>
    Response.json(new URL(request.url).pathname.endsWith('/recheck') ? last : rows))
  const host = document.createElement('div')
  document.body.append(host)
  await act(async () => createRoot(host).render(<QueryClientProvider client={new QueryClient()}>
    <RecheckPanel workflowId="w1" onClose={onClose} /></QueryClientProvider>))
  for (let i = 0; i < 3; i += 1) await act(async () => { await new Promise((resolve) => setTimeout(resolve, 0)) })
  return host
}
const output = (host: HTMLElement) => host.querySelector('pre')?.textContent

it('shows_who_began_it_and_the_counts_and_one_line_per_check', async () => {
  const host = await draw()
  expect(host.textContent).toContain('Project recheck')
  expect(host.textContent).toContain('You asked')
  expect(host.textContent).toContain('1 passed · 1 failed · 1 left')
  expect(host.querySelectorAll('li')).toHaveLength(3)
})

it('live_chunks_append_to_the_running_check_which_is_selected_first', async () => {
  const host = await draw()
  await act(async () => {
    FakeSource.last?.send('started', { task: 3, attempt_id: 'a3', check_id: 'c' })
    FakeSource.last?.send('chunk', { task: 3, attempt_id: 'a3', check_id: 'c', text: 'compiling\n' })
    FakeSource.last?.send('chunk', { task: 3, attempt_id: 'a3', check_id: 'c', text: 'ok\n' })
  })
  expect(output(host)).toBe('compiling\nok\n')
})

it('selecting_a_finished_check_shows_its_stored_output', async () => {
  const host = await draw()
  await act(async () => { host.querySelectorAll('li button')[1]?.dispatchEvent(new MouseEvent('click', { bubbles: true })) })
  expect(output(host)).toBe('stored two')
})

// happy-dom has no layout, so this holds the cause: the absolutely placed state labels must have the
// scrolling list as their containing block, or they extend the page past the window.
it('the_checks_list_contains_its_hidden_state_labels', async () => {
  const host = await draw()
  const list = host.querySelector('ul[aria-label="Checks"]')
  expect(list?.classList.contains('relative')).toBe(true)
  for (const label of host.querySelectorAll('li .sr-only')) {
    expect(label.parentElement?.closest('.relative, .absolute, .fixed, .sticky')).toBe(list)
  }
})

it('the_chevron_closes_the_panel', async () => {
  const onClose = vi.fn()
  const host = await draw(onClose)
  await act(async () => { host.querySelector<HTMLButtonElement>('button[aria-label="Close the recheck panel"]')?.click() })
  expect(onClose).toHaveBeenCalledOnce()
})
