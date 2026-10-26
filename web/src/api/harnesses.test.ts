import { expect, it } from 'vitest'
import { harnessKeys, harnessesQuery } from './harnesses'
it('harness consumers retain the same identity through the harness query', () => {
  expect(harnessKeys.list()).toEqual(['harnesses'])
  expect(harnessesQuery.queryKey).toEqual(harnessKeys.list())
})
