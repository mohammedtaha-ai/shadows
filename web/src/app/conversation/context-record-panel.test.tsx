// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import type { ContextRecord } from '@/api/planning-context'
import type { Operation } from '@/api/threads'
import { completedOperation, invocationFixture, threadFixture, userEntry } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { startApp, until, type TestApp } from '@/test/test-app'

const path = '/projects/p1/threads/t1'
const recordRoute = 'GET /api/operations/op1/context'
const operationsRoute = 'GET /api/threads/t1/operations'
const panelSelector = 'aside[aria-label="Planning context beside the conversation"]'
const heard = 'Planning context · heard'
const prepared = 'Planning context · not confirmed yet'
const record: ContextRecord = {
  heard: false,
  carried: {
    text: 'The full recorded context.',
    digest: 'd1',
    index_tokens: 140,
    detail_tokens: 8100,
    over_tokens: 240,
    cuts: ['Old plan rationale', 'Large outcome description'],
    sources: [{ kind: 'part', id: 'part-1', version: 4 }, { kind: 'outcome', id: 'outcome-2', version: 7 }],
  },
}

let app: TestApp | undefined
afterEach(() => { app?.unmount(); app = undefined })

function operations(planning_context: 'heard' | 'prepared' | null): Operation[] {
  const op = completedOperation({ ...invocationFixture, planning_context })
  return [{ ...op, status_kind: planning_context === 'prepared' ? 'Running' : 'Completed' }]
}

function routesFor(state: 'heard' | 'prepared' | null) {
  const routes = answers({ entries: [userEntry('u1', 'Show planning context')], operations: operations(state) })
  routes[recordRoute] = record
  return routes
}

async function show(routes: ReturnType<typeof routesFor>) {
  const opened = app = await startApp(path, routes)
  await until(() => opened.text().includes('Show planning context') && opened.calls.includes(operationsRoute))
  return opened
}

const panel = (opened: TestApp) => opened.container.querySelector<HTMLElement>(panelSelector)

async function openRecord(opened: TestApp, label = heard) {
  await until(() => opened.button(label) !== undefined)
  await act(async () => opened.button(label)?.click())
  await until(() => panel(opened)?.textContent?.includes(record.carried.text) === true)
}

async function liveStream(opened: TestApp) {
  await until(() => opened.sources.some(source => source.param('thread_id') === 't1'))
  const stream = opened.sources.find(source => source.param('thread_id') === 't1')!
  await act(async () => stream.caughtUp(0))
  return stream
}

it('a_heard_turn_shows_heard', async () => {
  const opened = await show(routesFor('heard'))
  await until(() => opened.button(heard) !== undefined)
  expect(opened.calls).not.toContain(recordRoute)
  expect(opened.button(heard)?.closest('[data-entry-kind]')?.getAttribute('data-entry-kind')).toBe('UserMessage')
})

it('a_prepared_turn_shows_not_confirmed_yet', async () => {
  const opened = await show(routesFor('prepared'))
  await until(() => opened.button(prepared) !== undefined)
  expect(opened.button(heard)).toBeUndefined()
})

it('an_open_panel_turns_to_heard_when_the_turn_completes', async () => {
  const routes = routesFor('prepared')
  let state: 'prepared' | 'heard' = 'prepared'
  routes[operationsRoute] = () => Response.json(operations(state))
  const opened = await show(routes)
  const stream = await liveStream(opened)
  await openRecord(opened, prepared)
  const initial = panel(opened)
  expect(initial?.querySelector('[role="status"]')?.textContent).toBe('not confirmed yet')
  state = 'heard'
  await act(async () => stream.durable(1, 'OperationCompleted', 'op1', {
    status_kind: 'Completed', cancel_requested: false,
  }))
  await until(() => panel(opened)?.querySelector('[role="status"]')?.textContent === 'heard')
  expect(panel(opened)).toBe(initial)
  expect(opened.button(heard)).toBeDefined()
  expect(opened.calls.filter(call => call === recordRoute)).toHaveLength(1)
})

it('a_plan_show_replaces_the_context_panel', async () => {
  const opened = await show(routesFor('heard'))
  const stream = await liveStream(opened)
  await openRecord(opened)
  const { tabId } = await import('@/stream')
  await act(async () => stream.emit('plan-show', JSON.stringify({
    thread_id: 't1', target_tab: tabId(), workflow_id: 'w1', version: 1, task_number: null, place: 'side',
  })))
  await until(() => opened.button('Close plan') !== undefined)
  expect(panel(opened)).toBeNull()
})

it('a_turn_without_a_record_shows_nothing', async () => {
  const opened = await show(routesFor(null))
  await until(() => opened.text().includes('Completed'))
  expect(opened.button(prepared)).toBeUndefined()
  expect(opened.button(heard)).toBeUndefined()
  expect(opened.calls).not.toContain(recordRoute)
})

it('lists_cuts_before_the_text', async () => {
  const opened = await show(routesFor('heard'))
  await openRecord(opened)
  const text = panel(opened)?.textContent ?? ''
  for (const cut of record.carried.cuts) {
    expect(text.indexOf(cut)).toBeGreaterThanOrEqual(0)
    expect(text.indexOf(cut)).toBeLessThan(text.indexOf(record.carried.text))
  }
  expect(text).toContain('Over-budget tokens240')
})

it('lists_each_source_with_its_version', async () => {
  const opened = await show(routesFor('heard'))
  await openRecord(opened)
  expect(panel(opened)?.textContent).toContain('part · part-1 · version 4')
  expect(panel(opened)?.textContent).toContain('outcome · outcome-2 · version 7')
})

it('deleted_history_can_read_the_record', async () => {
  const routes = routesFor('heard')
  routes['GET /api/projects/p1/threads'] = []
  routes['GET /api/threads/t1'] = { ...threadFixture, removed_at: '2026-11-05T00:00:00Z' }
  const opened = await show(routes)
  await openRecord(opened)
  expect(opened.text()).toContain('This conversation was deleted')
  expect(opened.calls).not.toContain('POST /api/threads/t1/session')
})
