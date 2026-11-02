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
    borderRadius: 14,
  })
  const kind = data?.kind ?? 'needs'
  const label = data?.label ?? ''
  const strokeColor = data?.broken ? 'var(--destructive)' : STROKE[kind]
  return (
    <>
      {kind === 'needs' && !data?.broken && (
        <path d={path} fill="none" stroke="var(--accent-line)" strokeWidth={5} strokeOpacity={0.14} className="pointer-events-none" />
      )}
      <BaseEdge
        id={id}
        path={path}
        markerEnd={kind === 'start' ? undefined : markerEnd}
        data-link={id}
        style={{
          stroke: strokeColor,
          strokeWidth: kind === 'start' ? 1.25 : 1.85,
          ...(kind === 'completes_after' && { strokeDasharray: '6 5' }),
        }}
      />
      {kind !== 'start' && (
        <>
          <circle cx={sourceX} cy={actualSourceY} r={2.75} fill="var(--background)" stroke={strokeColor} strokeWidth={1.5} className="pointer-events-none" />
          <circle cx={targetX} cy={actualTargetY} r={2.75} fill={strokeColor} className="pointer-events-none" />
        </>
      )}
      {label !== '' && (
        <EdgeLabelRenderer>
          <div
            data-link={id}
            dir="auto"
            title={label}
            className="nodrag nopan pointer-events-auto absolute max-w-36 truncate rounded-md border border-accent-line/30 bg-background/95 px-2 py-0.5 font-mono text-2xs leading-4 text-accent-line shadow-2xs backdrop-blur-xs"
            style={{ transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)` }}
          >
            {label}
          </div>
        </EdgeLabelRenderer>
      )}
    </>
  )
}
