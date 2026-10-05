// One job: a subagent's card in the conversation (spec §22.4) — its task,
// model, state and numbers; clicking it opens what it did beside the
// conversation.

import { Bot, ChevronRight, CircleCheck, CircleStop, CircleX, LoaderCircle } from 'lucide-react'
import type { SubagentCard } from '@/stream/frames'

const STATE = {
  running: { icon: LoaderCircle, text: 'Running', tone: 'text-accent-line' },
  completed: { icon: CircleCheck, text: 'Done', tone: 'text-accent-line' },
  failed: { icon: CircleX, text: 'Failed', tone: 'text-destructive' },
  stopped: { icon: CircleStop, text: 'Stopped', tone: 'text-faint-foreground' },
} as const

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

export function SubagentCardView({
  card,
  onOpen,
}: {
  card: SubagentCard
  onOpen: (id: string) => void
}) {
  const state = STATE[card.status]
  const who = [card.agentType, card.model === null ? null : modelName(card.model)]
    .filter((part) => part !== null)
    .join(' · ')
  const said = numbers(card)
  return (
    <button
      type="button"
      onClick={() => onOpen(card.id)}
      data-subagent={card.id}
      data-status={card.status}
      className="group/card flex w-full max-w-xl items-start gap-3 rounded-lg border border-border bg-card px-3 py-2.5 text-start transition-colors hover:bg-muted"
    >
      <Bot aria-hidden className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
      <span className="min-w-0 flex-1">
        <span dir="auto" className="block truncate text-sm text-card-foreground">
          {card.title || 'Subagent'}
        </span>
        <span className="mt-0.5 flex flex-wrap items-center gap-x-2 text-xs text-muted-foreground">
          <span className={`flex items-center gap-1 ${state.tone}`}>
            <state.icon
              aria-hidden
              className={`size-3 ${card.status === 'running' ? 'animate-spin motion-reduce:animate-none' : ''}`}
            />
            {state.text}
          </span>
          {who !== '' && <span>{who}</span>}
          {said !== '' && <span>{said}</span>}
        </span>
      </span>
      <ChevronRight
        aria-hidden
        className="mt-0.5 size-4 shrink-0 text-faint-foreground group-hover/card:text-foreground"
      />
    </button>
  )
}
