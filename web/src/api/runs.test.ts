import { expect, it } from 'vitest'
import { attemptBriefQuery, attemptChecksQuery, attemptConflictQuery, lastRecheckQuery } from './runs'

it('query keys follow the agreement', () => {
  expect(attemptBriefQuery('a1').queryKey).toEqual(['runs', 'brief', 'a1'])
  expect(attemptConflictQuery('a1').queryKey).toEqual(['runs', 'conflict', 'a1'])
})

it('checks_and_conflict_have_no_interval', () => {
  for (const options of [attemptChecksQuery('a1'), attemptConflictQuery('a1')]) expect('refetchInterval' in options).toBe(false)
})

it('the_last_recheck_has_no_interval', () => {
  expect('refetchInterval' in lastRecheckQuery('w1')).toBe(false)
})
