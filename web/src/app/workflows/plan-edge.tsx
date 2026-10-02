// One job: drawing one link of the plan graph — `needs` solid,
// `completes_after` dashed, each with its label (§13.11).

import { BaseEdge, EdgeLabelRenderer, type EdgeProps, getSmoothStepPath } from '@xyflow/react'
import type { PlanEdge } from './layout'

const STROKE: Record<string, string> = {
  needs: 'var(--accent-line)',
  completes_after: 'var(--muted-foreground)',
  start: 'var(--border)',
}

export function LinkEdge({
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
  markerEnd,
  data,
}: EdgeProps<PlanEdge>) {
  const actualSourceY = sourceY + (data?.sourceYOffset ?? 0)
  const actualTargetY = targetY + (data?.targetYOffset ?? 0)
  const stepPosition = data?.stepPosition ?? 0.5

  const [path, labelX, labelY] = getSmoothStepPath({
    sourceX,
    sourceY: actualSourceY,
    targetX,
    targetY: actualTargetY,
    sourcePosition,
    targetPosition,
    stepPosition,
    borderRadius: 8,
  })
  const kind = data?.kind ?? 'needs'
  const label = data?.label ?? ''
  return (
    <>
      <BaseEdge
        id={id}
        path={path}
        markerEnd={kind === 'start' ? undefined : markerEnd}
        data-link={id}
        style={{
          stroke: STROKE[kind],
          strokeWidth: kind === 'start' ? 1 : 1.5,
          ...(kind === 'completes_after' && { strokeDasharray: '6 5' }),
        }}
      />
      {label !== '' && (
        <EdgeLabelRenderer>
          <div
            data-link={id}
            dir="auto"
            title={label}
            className="nodrag nopan pointer-events-auto absolute max-w-32 truncate rounded-md border border-border/80 bg-background/95 px-1.5 py-0.5 text-[11px] leading-4 text-muted-foreground shadow-2xs backdrop-blur-xs"
            style={{ transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)` }}
          >
            {label}
          </div>
        </EdgeLabelRenderer>
      )}
    </>
  )
}
