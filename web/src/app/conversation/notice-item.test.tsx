// @vitest-environment happy-dom
//
// An executor's notice: an item of the Planner's activity, not the person's bubble.

import { act } from 'react'
import { createRoot } from 'react-dom/client'
import { afterEach, expect, it } from 'vitest'
import type { AttemptNotice } from '@/api/runs'
import type { ThreadEntry } from '@/api/threads'
import { entryOfKind } from '@/test/contract-fixtures'
import { until } from '@/test/test-app'
import { Entry } from './entry'

;(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true

afterEach(() => { document.body.innerHTML = '' })

const policy = { initial_mode: null, unattended_mode: null } as never

const card = (extra: Partial<AttemptNotice> = {}): AttemptNotice => ({
  workflow_id: 'w1', version: 1, task: 2, title: 'Login screen', attempt: 3, attempt_id: 'a3',
  model: 'sonnet', outcome: 'committed', commit: 'abcdef123456', report: 'It is **done**\n\n- one\n- two',
  reason: null, last_addition: false, ...extra,
})

const notice = (body: string, cardValue: AttemptNotice | null, author = 'Executor'): ThreadEntry => ({
  ...entryOfKind('n1', 'UserMessage', body),
  author: { kind: author, id: author === 'Executor' ? 'thread-x' : 'local' },
  card: cardValue as unknown as ThreadEntry['card'],
})

async function draw(entry: ThreadEntry): Promise<HTMLElement> {
  const host = document.createElement('div')
  document.body.append(host)
  await act(async () => createRoot(host).render(
    <Entry entry={entry} policy={policy} requestedMode={null} projectId="p"
      onPointAt={() => {}} onOpenSubagent={() => {}} />,
  ))
  return host
}

const header = (host: HTMLElement) => host.querySelector('button[aria-expanded]') as HTMLButtonElement

it('a_notice_draws_as_an_activity_item_not_a_bubble', async () => {
  await import('./reply-text')
  const host = await draw(notice('Shadows instructions to the Planner', card()))
  expect(header(host).textContent).toContain('Executor (sonnet) · T2 · attempt 3 committed')
  expect(host.textContent).toContain('Login screen')
  expect(host.textContent).toContain('abcdef1')
  // Streamdown draws bold as a styled element carrying data-streamdown="strong".
  const bold = () => host.querySelector('strong, [data-streamdown="strong"]')
  await until(() => bold() !== null)
  expect(bold()?.textContent).toBe('done')
  expect(host.textContent).not.toContain('Shadows instructions to the Planner')
  expect(document.querySelector('.surface-glow')).toBeNull()
})

it('a_failed_notice_shows_its_reason_and_report', async () => {
  const host = await draw(notice('x', card({ outcome: 'failed', attempt: 1, reason: 'Tests failed', report: 'Tried twice' })))
  expect(header(host).textContent).toContain('attempt 1 failed')
  expect(host.textContent).toContain('Tests failed')
  expect(host.textContent).toContain('Tried twice')
})

it('a_long_report_is_cut_with_show_more', async () => {
  const report = Array.from({ length: 40 }, (_, i) => `line ${i}`).join('\n')
  const host = await draw(notice('x', card({ report })))
  const more = [...host.querySelectorAll('button')].find((b) => b.textContent === 'Show more')
  expect(more).toBeDefined()
  expect(host.querySelector('.max-h-80')).not.toBeNull()
  act(() => more?.click())
  expect([...host.querySelectorAll('button')].some((b) => b.textContent === 'Show less')).toBe(true)
  expect(host.querySelector('.max-h-80')).toBeNull()
})

it('a_notice_without_a_readable_card_shows_its_body_under_executor', async () => {
  const host = await draw(notice('The whole notice text', null))
  expect(header(host).textContent).toBe('Executor')
  expect(host.textContent).toContain('The whole notice text')
})

it('the_header_closes_and_reopens_the_report', async () => {
  const host = await draw(notice('x', card({ report: 'The report text' })))
  expect(host.textContent).toContain('The report text')
  act(() => header(host).click())
  expect(header(host).getAttribute('aria-expanded')).toBe('false')
  expect(host.textContent).not.toContain('The report text')
  act(() => header(host).click())
  expect(host.textContent).toContain('The report text')
})

it('an_old_notice_from_the_person_stays_a_bubble', async () => {
  const host = await draw(notice('[Shadows] Task 2 is done', null, 'User'))
  expect(host.querySelector('.surface-glow')).not.toBeNull()
  expect(host.textContent).toContain('[Shadows] Task 2 is done')
  expect(host.querySelector('button[aria-expanded]')).toBeNull()
})

it('a_shadows_notice_names_shadows', async () => {
  const entry = { ...notice('You changed files without a reason', null, 'Shadows') }
  const host = await draw(entry)
  expect(header(host).textContent).toBe('Shadows')
  expect(host.textContent).toContain('You changed files without a reason')
})

it('a_project_change_shows_its_reason', async () => {
  const change = (reason: string | null, noted: boolean) => ({
    ...entryOfKind('c1', 'ProjectChange', 'x'),
    card: { paths: ['src/a.ts', 'src/b.ts'], reason, noted },
  })
  const noted = await draw(change('Renamed the helper', true))
  expect(noted.textContent).toContain('src/a.ts')
  expect(noted.textContent).toContain('src/b.ts')
  expect(noted.textContent).toContain('Renamed the helper')
  const unnoted = await draw(change(null, false))
  expect(unnoted.textContent).toContain('no reason given')
})
