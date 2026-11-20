// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, typeInto, until } from '@/test/test-app'
import type { FileEntry, SymbolEntry, WhereAnswer } from '@/api/code-record'

let app: TestApp | null = null
afterEach(() => { app?.unmount(); app = null })

const WHERE = 'GET /api/projects/p1/code-where'
const owner = { part_id: 'web', part_title: 'Web client', layer: 'ui' }
const symbol = (over: Partial<SymbolEntry> = {}): SymbolEntry => ({
  symbol_id: 'login', name: 'login', kind: 'fn', at_done: 'found', behavior: [], live: { state: 'not_asked' },
  task: 1, version: 1, workflow_id: 'w1', uses: [], uses_live: { state: 'not_asked' }, uses_more: 0, uses_total: null, ...over,
})
const file = (path: string, rank: FileEntry['rank'], symbols: SymbolEntry[] = []): FileEntry & { kind: 'file' } => ({
  kind: 'file', path, rank, live: { state: 'not_asked' }, symbols, tasks: [],
})
const answer = (over: Partial<WhereAnswer> = {}): WhereAnswer => ({
  topic: 'login', asked: 2, entries: [], entries_more: 0, not_in_record: [], not_in_record_more: 0, ...over,
})
const open = (reply: () => WhereAnswer | Response, url = '/projects/p1/state?view=where&topic=login') =>
  startApp(url, { ...answers(), [WHERE]: () => { const r = reply(); return r instanceof Response ? r : Response.json(r) } })
const withSymbol = (over: Partial<SymbolEntry>) => answer({ entries: [file('src/auth.rs', 'exact', [symbol(over)])] })
const reads = (a: TestApp) => a.calls.filter(call => call === WHERE).length

it('asking_reads_once_and_keeps_cores_order', async () => {
  const a = app = await open(() => answer({ entries: [file('src/words.rs', 'words'), file('src/exact.rs', 'exact')] }), '/projects/p1/state?view=where')
  expect(reads(a)).toBe(0)
  typeInto(a.container.querySelector<HTMLInputElement>('input[aria-label="Topic"]')!, 'login')
  act(() => a.container.querySelector('form[aria-label="Where is it"]')!.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true })))
  await until(() => a.container.querySelectorAll('[data-where-file]').length === 2)
  expect([...a.container.querySelectorAll('[data-where-file]')].map(x => x.getAttribute('data-where-file'))).toEqual(['src/words.rs', 'src/exact.rs'])
  expect(reads(a)).toBe(1)
  expect(a.text()).toContain('2 language-server questions asked')
})

it('an_answered_empty_lookup_says_no_uses', async () => {
  const a = app = await open(() => withSymbol({ uses_live: { state: 'answered' }, uses_total: 0 }))
  await until(() => a.text().includes('No uses.'))
})

it.each(['failed', 'skipped'] as const)('a_%s_lookup_says_why_and_never_no_uses', async state => {
  const a = app = await open(() => withSymbol({ uses_live: { state, why: 'server busy' } }))
  await until(() => a.text().includes('Uses not checked: server busy'))
  expect(a.text()).not.toContain('No uses.')
})

it('a_truncated_lookup_keeps_its_uses_and_says_at_least', async () => {
  const a = app = await open(() => withSymbol({
    uses_live: { state: 'truncated', why: 'budget spent' }, uses: [{ path: 'src/a.rs', line: 4, owner }], uses_more: 3,
  }))
  await until(() => a.text().includes('at least 3 more'))
  expect(a.text()).toContain('src/a.rs:4')
  expect(a.text()).toContain('budget spent')
  expect(a.text()).not.toContain('No uses.')
})

it('answered_uses_show_the_total_and_the_rest_grouped_by_part', async () => {
  const a = app = await open(() => withSymbol({
    uses_live: { state: 'answered' }, uses_total: 5, uses_more: 2,
    uses: [{ path: 'src/a.rs', line: 1, owner }, { path: 'src/b.rs', line: 2, owner }, { path: 'src/c.rs', line: 3, owner: null }],
  }))
  await until(() => a.text().includes('5 uses'))
  const text = a.text()
  expect(text).toContain('2 more')
  expect(text.match(/Web client · ui/g)).toHaveLength(1)
  expect(text.indexOf('src/a.rs:1')).toBeLessThan(text.indexOf('src/b.rs:2'))
  expect(text.indexOf('src/b.rs:2')).toBeLessThan(text.indexOf('No part'))
})

it('a_not_asked_symbol_shows_no_uses_line', async () => {
  const a = app = await open(() => withSymbol({}))
  await until(() => a.container.querySelector('[data-where-symbol="login"]') !== null)
  const text = a.container.querySelector('[data-where-symbol="login"]')!.textContent!
  expect(text).not.toContain('No uses.')
  expect(text).not.toContain('Uses not checked')
  expect(text).not.toContain(' uses')
})

it('no_entry_says_shadows_has_no_record_with_files_not_in_the_record', async () => {
  const a = app = await open(() => answer({
    not_in_record: [{ path: 'src/new.rs', owner, symbols: [{ name: 'make', line: 9 }] }], not_in_record_more: 4,
  }))
  await until(() => a.text().includes('Shadows has no record of this.'))
  const section = a.container.querySelector('section[aria-label="Not in the record"]')!
  expect(section.textContent).toContain('src/new.rs')
  expect(section.textContent).toContain('make:9')
  expect(section.textContent).toContain('4 more files')
})

it('a_blank_topic_shows_cores_refusal', async () => {
  const a = app = await open(
    () => Response.json({ code: 'INVALID_COMMAND', message: 'topic is empty' }, { status: 422 }),
    '/projects/p1/state?view=where&topic=%20',
  )
  await until(() => a.text().includes('INVALID_COMMAND'))
  expect(a.text()).toContain('topic is empty')
})

it('the_budget_line_is_shown_when_core_gives_one', async () => {
  const a = app = await open(() => answer({ budget: 'stopped after 20 asks' }))
  await until(() => a.container.querySelector('[role="status"]')?.textContent === 'stopped after 20 asks')
})

it('ask_again_reads_the_answer_again', async () => {
  const a = app = await open(() => answer())
  await until(() => a.button('Ask again') !== undefined)
  expect(reads(a)).toBe(1)
  act(() => a.button('Ask again')!.click())
  await until(() => reads(a) === 2)
})

it('project_events_do_not_ask_again', async () => {
  const a = app = await open(() => answer())
  await until(() => a.button('Ask again') !== undefined && a.sources.some(s => s.url.includes('/projects/p1/events')))
  const before = reads(a)
  act(() => a.sources.find(s => s.url.includes('/projects/p1/events'))!.durable(1, 'TaskProgressChanged', null,
    { project_id: 'p1', workflow_id: 'w1', version: 1, task: 1, attempt_id: null }))
  await act(async () => { await new Promise(resolve => setTimeout(resolve, 50)) })
  expect(reads(a)).toBe(before)
})
