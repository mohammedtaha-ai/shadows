// One job: the task a person points at (§13.9) — the chip above the composer
// that says which, and the `focus` the next turn carries for it.

import { Crosshair, X } from 'lucide-react'
import type { PlanTask } from '@/api/client'

/** A task clicked in a card or the side panel, in the version and at the
 * revision the person was looking at. */
export interface PointedTask {
  readonly workflowId: string
  readonly revision: number
  readonly task: PlanTask
}

/** "T4 · Login screen ×" */
export function FocusChip({ pointed, onClear }: { pointed: PointedTask; onClear: () => void }) {
  const { number, title } = pointed.task
  return (
    <div
      data-focus-chip
      className="inline-flex max-w-full items-center gap-1.5 rounded-full border border-accent-line/50 bg-accent-softer py-0.5 pr-1 pl-2.5 text-xs text-secondary-foreground"
    >
      <Crosshair aria-hidden className="size-3 shrink-0" />
      <span dir="auto" className="shrink-0 font-medium">
        T{number}
      </span>
      {' · '}
      <span dir="auto" className="truncate">
        {title}
      </span>
      <button
        type="button"
        onClick={onClear}
        aria-label="Remove focus"
        className="rounded-full p-0.5 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
      >
        <X className="size-3" />
      </button>
    </div>
  )
}
