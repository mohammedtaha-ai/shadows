// A related plan's task, including broken dependencies and navigation.

import { Handle, type NodeProps, Position } from '@xyflow/react'
import { Link } from '@tanstack/react-router'
import type { LinkedNode, MissingNode } from './linked-layout'

export function MissingTaskNodeView({ data }: NodeProps<MissingNode>) {
  return <div className="box-border h-full w-full rounded-lg border border-destructive bg-card px-3 py-2.5">
    <p className="text-[11px] text-destructive">T{data.number} · unavailable</p>
    <p dir="auto" role="note" className="mt-2 text-xs text-destructive">{data.reason}</p>
    <Handle type="target" position={Position.Left} isConnectable={false} className="opacity-0" />
    <Handle type="source" position={Position.Right} isConnectable={false} className="opacity-0" />
  </div>
}

export function LinkedNodeView({ data }: NodeProps<LinkedNode>) {
  const { view, foreign } = data
  const number = view.task?.number ?? (view.incoming ? view.link.task :
    typeof view.link.after === 'number' ? view.link.after : view.link.after.task)
  const content = <>
    {foreign && <p dir="auto" className="h-4 truncate text-[11px] text-faint-foreground">{view.project_name ?? 'Unavailable project'}</p>}
    <p dir="auto" className="h-4 truncate text-[11px] text-accent-line">{view.plan_title ?? 'Unavailable plan'} · T{number}</p>
    <p dir="auto" className="mt-1 truncate text-sm font-medium">{view.task?.title ?? `T${number}`}</p>
    <p dir="auto" className="mt-1 line-clamp-2 h-8 text-xs text-muted-foreground">{view.task?.goal}</p>
    <p className="mt-1 text-[11px] text-faint-foreground">{view.task?.state ?? view.state ?? 'Unavailable'}{view.version ? ` · v${view.version}` : ''}</p>
    {view.broken && <p dir="auto" role="note" className="mt-1 line-clamp-3 text-xs text-destructive">{view.broken}</p>}
    <Handle type="target" position={Position.Left} isConnectable={false} className="opacity-0" />
    <Handle type="source" position={Position.Right} isConnectable={false} className="opacity-0" />
  </>
  const className = `box-border block h-full w-full rounded-lg border bg-card px-3 py-2.5 ${foreign ? 'border-dashed' : ''} ${view.broken ? 'border-destructive' : 'border-border'}`
  return view.project_id && view.workflow_id ? <Link to="/projects/$projectId/workflows/$workflowId"
    params={{ projectId: view.project_id, workflowId: view.workflow_id }}
    aria-label={`Open ${view.plan_title ?? 'related plan'} · T${number}`} className={className}>{content}</Link> :
    <div className={className}>{content}</div>
}
