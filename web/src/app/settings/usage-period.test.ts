import { expect, it } from 'vitest'
import { usageRange } from './usage-period'

it('periods produce bounded UTC ranges', () => {
  const now = new Date('2026-10-21T12:00:00.000Z')
  expect(usageRange('7d', now)).toEqual({ from: '2026-10-14T12:00:00.000Z', to: now.toISOString() })
  expect(usageRange('30d', now)).toEqual({ from: '2026-09-21T12:00:00.000Z', to: now.toISOString() })
  expect(usageRange('month', now)).toEqual({ from: new Date(2026, 9, 1).toISOString(), to: now.toISOString() })
  const boundary = new Date(2027, 0, 1)
  expect(usageRange('month', boundary)).toEqual({ from: boundary.toISOString(), to: new Date(boundary.getTime() + 1).toISOString() })
  const rollover = usageRange('7d', boundary)
  expect(new Date(rollover.to).getTime() - new Date(rollover.from).getTime()).toBe(7 * 86400000)
})
