import { expect, it } from 'vitest'
import { attemptBriefQuery, attemptConflictQuery, holdRecheckStream, lastRecheckQuery, type TaskConflict } from './runs'

const conflict = (extra: Partial<TaskConflict> = {}): TaskConflict => ({
  attempt_id: 'a1', project_id: 'p1', workflow_id: 'w1', version: 1, task: 1, expected: 'x', observed: 'y',
  evidence: [], attempt_state: 'stopped', scope_held: false, delivery: 'delivered', created_at: '2026-11-14T10:00:00Z', ...extra,
})

const interval = (data: TaskConflict | null): number | false => {
  const options = attemptConflictQuery('a1') as unknown as {
    refetchInterval: (query: { state: { data: TaskConflict | null } }) => number | false
  }
  return options.refetchInterval({ state: { data } })
}

it('query keys follow the agreement', () => {
  expect(attemptBriefQuery('a1').queryKey).toEqual(['runs', 'brief', 'a1'])
  expect(attemptConflictQuery('a1').queryKey).toEqual(['runs', 'conflict', 'a1'])
})

it('settled_conflict_stops_polling', () => {
  expect(interval(conflict())).toBe(false)
  expect(interval(null)).toBe(false)
  expect(interval(conflict({ delivery: 'no_planner' }))).toBe(false)
})

it('an_unsettled_conflict_keeps_polling', () => {
  expect(interval(conflict({ attempt_state: 'running' }))).toBe(2000)
  expect(interval(conflict({ scope_held: true }))).toBe(2000)
  expect(interval(conflict({ delivery: 'pending' }))).toBe(2000)
})

const recheckInterval = (unfinished: number): number | false => {
  const options = lastRecheckQuery('w1') as unknown as {
    refetchInterval: (query: { state: { data: unknown } }) => number | false
  }
  return options.refetchInterval({ state: { data: { recheck_id: 'r1', counts: { unfinished } } } })
}

it('a_held_output_stream_stops_the_last_recheck_interval', () => {
  expect(recheckInterval(2)).toBe(5000)
  const release = holdRecheckStream('r1')
  expect(recheckInterval(2)).toBe(false)
  release()
  expect(recheckInterval(2)).toBe(5000)
  expect(recheckInterval(0)).toBe(false)
})
