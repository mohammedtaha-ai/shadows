// One job: how a subagent's numbers and model read on its card and panel
// (spec §22.4).

import type { SubagentCard } from '@/stream/frames'

/** `13665` → `14s`, `252000` → `4m 12s`. */
export function duration(ms: number): string {
  const s = Math.round(ms / 1000)
  return s < 60 ? `${s}s` : `${Math.floor(s / 60)}m ${s % 60}s`
}

/** `43119` → `43k`, `950` → `950`. */
export function tokens(n: number): string {
  return n < 1000 ? `${n}` : `${Math.round(n / 1000)}k`
}

/** `claude-sonnet-5-5` → `Sonnet 5.5`; `sonnet` → `Sonnet`. */
export function modelName(model: string): string {
  const found = /^(?:claude-)?([a-z]+)(?:-(\d+)(?:-(\d+))?)?/.exec(model)
  if (found === null) return model
  const [, family, major, minor] = found
  const name = family.charAt(0).toUpperCase() + family.slice(1)
  return [name, [major, minor].filter(Boolean).join('.')].filter(Boolean).join(' ')
}

/** What the card says under its task: its numbers, each only when reported. */
export function numbers(card: SubagentCard): string {
  const steps = card.toolCount ?? card.steps.length
  return [
    card.durationMs === null ? null : duration(card.durationMs),
    card.tokens === null ? null : `${tokens(card.tokens)} tokens`,
    steps === 0 ? null : `${steps} ${steps === 1 ? 'tool' : 'tools'}`,
  ]
    .filter((part) => part !== null)
    .join(' · ')
}
