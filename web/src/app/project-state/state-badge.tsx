// One job: a build state as an icon and a label.
import { CircleCheck, CircleHelp, CircleSlash, Hourglass, TriangleAlert, type LucideIcon } from 'lucide-react'
import type { BuildState } from '@/api/project-state'

const STATES: Record<BuildState, { Icon: LucideIcon; label: string }> = {
  planned: { Icon: Hourglass, label: 'Planned' },
  built: { Icon: CircleCheck, label: 'Built' },
  gap: { Icon: TriangleAlert, label: 'Gap' },
  unchecked: { Icon: CircleHelp, label: 'Unchecked' },
  removed: { Icon: CircleSlash, label: 'Removed' },
}

export function StateBadge({ state }: { state: BuildState }) {
  const { Icon, label } = STATES[state]
  return (
    <span data-element-state={state} className="inline-flex items-center gap-1 text-xs text-muted-foreground">
      <Icon aria-hidden className="size-3.5" />
      {label}
    </span>
  )
}
