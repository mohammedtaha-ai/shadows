// @vitest-environment happy-dom
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { act } from 'react'
import { createRoot } from 'react-dom/client'
import { afterEach, expect, it } from 'vitest'
import { foldFrame, rowKey, useRecheckOutput, type OutputSource, type RecheckOutput } from './recheck-output'

;(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true

afterEach(() => { document.body.innerHTML = '' })

const row = { task: 1, attempt_id: 'a1', check_id: 'c1' }
const key = rowKey(row)
const empty: RecheckOutput = { text: {}, running: null, ended: false, lagged: false }

it('chunks_append_to_the_started_row_and_a_new_start_restarts_it', () => {
  let state = foldFrame(empty, 'started', row)
  state = foldFrame(state, 'chunk', { ...row, text: 'a\n' })
  state = foldFrame(state, 'chunk', { ...row, text: 'b\n' })
  expect(state.text[key]).toBe('a\nb\n')
  expect(state.running).toBe(key)
  state = foldFrame(state, 'started', row)
  expect(state.text[key]).toBe('')
})

it('finished_clears_the_running_row_and_end_closes_the_batch', () => {
  let state = foldFrame(foldFrame(empty, 'started', row), 'finished', { ...row })
  expect(state.running).toBeNull()
  state = foldFrame(foldFrame(state, 'lagged', {}), 'end', {})
  expect(state.ended).toBe(true)
  expect(state.lagged).toBe(true)
})

it('the_hook_reads_frames_and_closes_on_end', async () => {
  const listeners = new Map<string, (event: Event) => void>()
  let closed = 0
  const source: OutputSource = { addEventListener: (type, fn) => { listeners.set(type, fn) }, close: () => { closed += 1 } }
  const connect = () => source
  function Probe() { return <p>{JSON.stringify(useRecheckOutput('r1', 'w1', connect))}</p> }
  const host = document.createElement('div')
  await act(async () => createRoot(host).render(
    <QueryClientProvider client={new QueryClient()}><Probe /></QueryClientProvider>))
  const send = (type: string, data: object) => act(async () => {
    listeners.get(type)?.(new MessageEvent(type, { data: JSON.stringify(data) }))
  })
  await send('started', row)
  await send('chunk', { ...row, text: 'ok' })
  expect(host.textContent).toContain(`"${key}":"ok"`)
  await send('end', {})
  expect(host.textContent).toContain('"ended":true')
  expect(closed).toBeGreaterThan(0)
})

it('a_finished_frame_refreshes_only_that_batch_and_workflow', async () => {
  const listeners = new Map<string, (event: Event) => void>()
  const source: OutputSource = { addEventListener: (type, fn) => { listeners.set(type, fn) }, close: () => {} }
  const client = new QueryClient()
  const seed = (queryKey: string[]) => client.fetchQuery({ queryKey, queryFn: () => 1 })
  await Promise.all([seed(['runs', 'recheck', 'w1']), seed(['runs', 'recheck-rows', 'r1']), seed(['runs', 'task', 'w1', '1', '1'])])
  function Probe() { useRecheckOutput('r1', 'w1', () => source); return null }
  await act(async () => createRoot(document.createElement('div')).render(
    <QueryClientProvider client={client}><Probe /></QueryClientProvider>))
  await act(async () => { listeners.get('finished')?.(new MessageEvent('finished', { data: JSON.stringify(row) })) })
  const stale = (queryKey: string[]) => client.getQueryState(queryKey)?.isInvalidated
  expect(stale(['runs', 'recheck', 'w1'])).toBe(true)
  expect(stale(['runs', 'recheck-rows', 'r1'])).toBe(true)
  expect(stale(['runs', 'task', 'w1', '1', '1'])).toBe(false)
})

it('unmounting_the_hook_refreshes_the_workflows_recheck_read', async () => {
  const source: OutputSource = { addEventListener: () => {}, close: () => {} }
  const client = new QueryClient()
  const seed = (queryKey: string[]) => client.fetchQuery({ queryKey, queryFn: () => 1 })
  const stale = (queryKey: string[]) => client.getQueryState(queryKey)?.isInvalidated
  await seed(['runs', 'recheck', 'w1'])
  function Probe() { useRecheckOutput('r1', 'w1', () => source); return null }
  const root = createRoot(document.createElement('div'))
  await act(async () => root.render(<QueryClientProvider client={client}><Probe /></QueryClientProvider>))
  expect(stale(['runs', 'recheck', 'w1'])).toBe(false)
  await act(async () => root.unmount())
  expect(stale(['runs', 'recheck', 'w1'])).toBe(true)
})
