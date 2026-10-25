import { expect, it } from 'vitest'
import { harnessKeys, harnessesQuery } from './harnesses'
import { harnessesQuery as publicQuery } from './queries'
it('harness consumers retain the same identity through the public query entry', () => {
  expect(harnessKeys.list()).toEqual(['harnesses'])
  expect(harnessesQuery.queryKey).toEqual(harnessKeys.list())
  expect(publicQuery).toBe(harnessesQuery)
})
