
import { ArrowLeftRight, Bot, ChevronRight, CircleCheck, CircleStop, CircleX, LoaderCircle } from 'lucide-react'
import type { SubagentCard } from '@/stream'
import { modelName, numbers } from './subagent-text'

const STATE = {
  running: { icon: LoaderCircle, text: 'Running', tone: 'text-success' },
  completed: { icon: CircleCheck, text: 'Done', tone: 'text-accent-line' },
  failed: { icon: CircleX, text: 'Failed', tone: 'text-destructive' },
  stopped: { icon: CircleStop, text: 'Stopped', tone: 'text-faint-foreground' },
} as const

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
      className="group/card flex w-full max-w-xl items-start gap-3.5 rounded-xl surface-glow px-4 py-3 text-start transition-colors hover:border-accent-line/60"
    >
      <span className="mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-lg border border-accent-line/35 bg-accent/25 text-accent-line">
        <Bot aria-hidden className="size-4" />
      </span>
      <span className="min-w-0 flex-1">
        <span dir="auto" className="block truncate text-sm font-semibold text-card-foreground">
          {card.title || 'Subagent'}
        </span>
        <span className="mt-1.5 flex flex-wrap items-center gap-x-2.5 gap-y-1 text-xs text-muted-foreground">
          <span className={`inline-flex items-center gap-1 font-medium ${state.tone}`}>
            <state.icon
              aria-hidden
              className={`size-3.5 ${card.status === 'running' ? 'animate-spin motion-reduce:animate-none' : ''}`}
            />
            {state.text}
          </span>
          {who !== '' && (
            <span className="inline-flex items-center gap-1 rounded-md border border-accent-line/25 bg-card px-2 py-0.5 font-mono text-2xs text-accent-foreground">
              <ArrowLeftRight aria-hidden className="size-3 text-accent-line" />
              {who}
            </span>
          )}
          {said !== '' && (
            <span className="rounded-md border border-accent-line/15 bg-background/50 px-1.5 py-0.5 font-mono text-2xs tabular-nums">
              {said}
            </span>
          )}
        </span>
      </span>
      <ChevronRight
        aria-hidden
        className="mt-1.5 size-4 shrink-0 text-faint-foreground transition-transform group-hover/card:translate-x-0.5 group-hover/card:text-accent-line"
      />
    </button>
  )
}
