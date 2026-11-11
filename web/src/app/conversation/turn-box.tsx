// One job: one Planner turn's activity as one collapsible box with a
// one-line summary; open while the turn runs, folded once it ends.

import { cn } from 'cn'
import { useLocation } from '@tanstack/react-router'
import { ChevronRight, LoaderCircle, Terminal } from 'lucide-react'
import { type ReactNode, useState } from 'react'
import { noticeName } from './notice-item'
import type { TurnSummary } from './turn-blocks'

const plural = (count: number, one: string, many: string) => `${count} ${count === 1 ? one : many}`

/** "Received a notice from Executor (sonnet), ran 5 tools": the parts that apply. */
function summaryLine(summary: TurnSummary): string {
  const parts = [
    ...(summary.notices.length === 0
      ? []
      : [`received a notice from ${summary.notices.map(noticeName).join(' and ')}`]),
    ...(summary.tools === 0 ? [] : [`ran ${plural(summary.tools, 'tool', 'tools')}`]),
    ...(summary.notes === 0 ? [] : [`wrote ${plural(summary.notes, 'note', 'notes')}`]),
  ]
  const line = parts.join(', ')
  return line.charAt(0).toUpperCase() + line.slice(1)
}

export function TurnBox({
  summary,
  live,
  ordinals,
  children,
}: {
  summary: TurnSummary
  /** The turn is still running. */
  live: boolean
  /** Every ordinal inside: a link to one opens the box. */
  ordinals: readonly number[]
  children: ReactNode
}) {
  const [chosen, setChosen] = useState<boolean | null>(null)
  const open = chosen ?? live
  const hash = useLocation().hash.replace(/^#/, '')
  const linked = ordinals.some((ordinal) => hash === `entry-${ordinal}`)
  const [seen, setSeen] = useState(false)
  if (linked !== seen) {
    setSeen(linked)
    if (linked) setChosen(true)
  }
  return (
    <div className="surface-panel rounded-xl p-3">
      <button
        type="button"
        aria-expanded={open}
        onClick={() => setChosen(!open)}
        className="flex w-full items-center gap-2.5 text-start text-xs font-medium text-secondary-foreground transition-colors hover:text-foreground"
      >
        <ChevronRight
          className={cn('size-4 shrink-0 text-accent-line transition-transform duration-150', open && 'rotate-90')}
        />
        {live ? (
          <LoaderCircle
            aria-label="Running"
            className="size-3.5 shrink-0 animate-spin text-accent-line motion-reduce:animate-none"
          />
        ) : (
          <Terminal aria-hidden className="size-3.5 shrink-0 text-accent-line/85" />
        )}
        <span dir="auto" className="font-medium text-foreground">{summaryLine(summary)}</span>
      </button>
      {open && <ol className="mt-2.5 flex flex-col gap-4 border-t border-border/60 pt-2.5">{children}</ol>}
    </div>
  )
}
