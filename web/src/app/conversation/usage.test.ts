import { describe, expect, it } from 'vitest'
import { completedOperation, invocationFixture } from '@/test/contract-fixtures'
import { contextShown, formatTokens, latestContext, resetIn } from './usage'

describe('usage figures', () => {
  it('shows context only when both numbers were reported', () => {
    expect(contextShown(126800, 1_000_000)).toEqual({ used: 126800, window: 1_000_000, percent: 13 })
    expect(contextShown(126800, null)).toBeNull()
    expect(contextShown(10, 0)).toBeNull()
  })

  it('formats tokens and reset times', () => {
    expect(formatTokens(126800)).toBe('126.8k')
    expect(formatTokens(1_000_000)).toBe('1M')
    expect(formatTokens(200_000)).toBe('200k')
    expect(formatTokens(950)).toBe('950')
    expect(resetIn(1000 + 4 * 3600 + 18 * 60, 1000)).toBe('4h18m')
    expect(resetIn(1000 + 3 * 86400 + 21 * 3600, 1000)).toBe('3d21h')
    expect(resetIn(1000 + 7 * 60, 1000)).toBe('7m')
    expect(resetIn(900, 1000)).toBe('now')
  })

  it("reads the context of the newest turn that reported one", () => {
    const reported = { ...completedOperation({ ...invocationFixture, context_used: 5, context_window: 10 }), id: 'op0' }
    const silent = completedOperation(invocationFixture)
    expect(latestContext([silent, reported])).toEqual({ contextUsed: 5, contextWindow: 10 })
    expect(latestContext([silent])).toBeNull()
  })
})
