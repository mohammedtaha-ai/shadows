import { describe, expect, it } from 'vitest'
import { ago } from './when'

const now = Date.parse('2026-10-01T12:00:00Z')
const format = new Intl.RelativeTimeFormat([], { numeric: 'auto', style: 'short' })

describe('ago', () => {
  it('reads a past time as how long ago it was', () => {
    expect(ago('2026-10-01T11:58:00Z', now)).toBe(format.format(-2, 'minute'))
  })

  it('reads a time ahead of the browser clock as now', () => {
    expect(ago('2026-10-01T12:00:03Z', now)).toBe(format.format(0, 'second'))
  })
})
