import { expect, it } from 'vitest'
import { usageKeys, usageQuery } from './usage'
it('usage reports retain interval-qualified keys and the shared report prefix', () => {
  expect(usageKeys.all()).toEqual(['aylul-usage'])
  expect(usageQuery('from', 'to').queryKey).toEqual(['aylul-usage', 'from', 'to'])
  expect(usageQuery('from', 'to').queryKey.slice(0, 1)).toEqual(usageKeys.all())
})
