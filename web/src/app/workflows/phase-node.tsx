import { Handle, type Node, type NodeProps, Position } from '@xyflow/react'
import { cn } from 'cn'
import type { Phase, PhaseEndpoint } from '@/api/workflows'

export interface PhaseData extends Record<string, unknown> {
  endpoint?: PhaseEndpoint
  phase?: Phase
  foreign: boolean
  broken?: string
}
export type PhaseNode = Node<PhaseData, 'phase'>

export function PhaseNodeView({ data, selected }: NodeProps<PhaseNode>) {
  const endpoint = data.endpoint
  return (
    <div data-phase-node data-foreign={data.foreign || undefined}
      className={cn('box-border h-full w-full overflow-hidden rounded-xl border bg-card px-3 py-2.5',
        data.foreign ? 'cursor-pointer border-dashed border-muted-foreground/60' : 'border-accent-line/30',
        selected && 'ring-2 ring-accent-line', data.broken && 'border-destructive')}>
      <span dir="auto" className="font-mono text-2xs text-accent-line">
        {endpoint ? `T${endpoint.task} · P${endpoint.phase_number}` : 'Missing phase'}
      </span>
      <p dir="auto" className="mt-1 truncate text-sm font-semibold" title={endpoint?.title}>
        {endpoint?.title ?? 'Unavailable dependency'}
      </p>
      <p dir="auto" className="mt-1 line-clamp-2 text-xs text-muted-foreground">
        {data.broken ?? data.phase?.goal ?? `${endpoint?.plan_title} · v${endpoint?.version}`}
      </p>
      {data.phase && <p dir="auto" className="mt-1 truncate text-2xs text-faint-foreground">
        Acceptance {data.phase.acceptance.join(', ') || '—'}
      </p>}
      <Handle type="target" position={Position.Left} isConnectable={false} className="opacity-0" />
      <Handle type="source" position={Position.Right} isConnectable={false} className="opacity-0" />
    </div>
  )
}
