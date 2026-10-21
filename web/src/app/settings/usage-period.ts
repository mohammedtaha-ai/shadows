export type UsagePeriod = '7d' | '30d' | 'month'

export function usageRange(period: UsagePeriod, now: Date): { from: string; to: string } {
  const end = now.getTime()
  const start = period === 'month' ? new Date(now.getFullYear(), now.getMonth(), 1).getTime() : end - (period === '7d' ? 7 : 30) * 86400000
  return { from: new Date(start).toISOString(), to: new Date(Math.max(end, start + 1)).toISOString() }
}
