import { expect, it } from 'vitest'
import { attemptBriefQuery, attemptChecksQuery, attemptConflictQuery, holdRecheckStream, lastRecheckQuery } from './runs'

it('query keys follow the agreement', () => {
  expect(attemptBriefQuery('a1').queryKey).toEqual(['runs', 'brief', 'a1'])
  expect(attemptConflictQuery('a1').queryKey).toEqual(['runs', 'conflict', 'a1'])
})

it('checks_and_conflict_have_no_interval', () => {
  for (const options of [attemptChecksQuery('a1'), attemptConflictQuery('a1')]) expect('refetchInterval' in options).toBe(false)
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
