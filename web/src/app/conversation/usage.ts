// One job: turning what the harness reported about context and limits into
// the figures a person reads (spec §12.8). Nothing it did not report is
// estimated: a figure that is missing is `null` here and "no figures" there.

import type { Operation } from '@/api/client'

/** The session's context use as last reported; either may be missing. */
export interface ContextFigures {
  contextUsed: number | null
  contextWindow: number | null
}

/** Context as a share of the window, or `null` unless both were reported and
 * the window is usable. */
export function contextShown(
  used: number | null,
  window: number | null,
): { used: number; window: number; percent: number } | null {
  if (used === null || window === null || window <= 0) return null
  return { used, window, percent: Math.round((used / window) * 100) }
}

/** `126.8k`, `1M`, `950`: one decimal at most, none when it is zero. */
export function formatTokens(n: number): string {
  const short = (value: number, unit: string) => `${Number(value.toFixed(1))}${unit}`
  if (n >= 1_000_000) return short(n / 1_000_000, 'M')
  if (n >= 1_000) return short(n / 1_000, 'k')
  return String(n)
}

/** How long until `resetsAt` (Unix seconds) from `now` (Unix seconds), in
 * the two largest units: `3d21h`, `4h18m`, `7m`. */
export function resetIn(resetsAt: number, now: number): string {
  const minutes = Math.floor((resetsAt - now) / 60)
  if (minutes <= 0) return 'now'
  const days = Math.floor(minutes / 1440)
  const hours = Math.floor((minutes % 1440) / 60)
  if (days > 0) return `${days}d${hours}h`
  if (hours > 0) return `${hours}h${minutes % 60}m`
  return `${minutes}m`
}

/** The context the newest turn that reported one ended with (operations
 * newest first), or `null`. */
export function latestContext(operations: readonly Operation[]): ContextFigures | null {
  for (const operation of operations) {
    const invocation = operation.invocation
    if (invocation == null) continue
    if (invocation.context_used !== null || invocation.context_window !== null) {
      return { contextUsed: invocation.context_used, contextWindow: invocation.context_window }
    }
  }
  return null
}
