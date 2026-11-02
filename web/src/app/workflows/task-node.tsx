// Each node fills the fixed box `layout.ts` sized for it; the rows here have
// the fixed heights that sizing assumed. Every text element carries
// `dir="auto"`, so an Arabic title, goal or path reads right to left inside
// its row.

import { Handle, type NodeProps, Position } from '@xyflow/react'
import type { StartNode, TaskNode as TaskNodeType } from './layout'

/** Edges attach here; nobody drags a new link out of a plan. */
function Handles() {
  return (
    <>
      <Handle type="target" position={Position.Left} isConnectable={false} className="opacity-0" />
      <Handle type="source" position={Position.Right} isConnectable={false} className="opacity-0" />
    </>
  )
}

export function StartNodeView({ data }: NodeProps<StartNode>) {
  return (
    <div className="box-border flex h-full w-full flex-col overflow-hidden rounded-xl surface-glow px-3 py-2.5">
      <span dir="auto" className="h-4 text-2xs leading-4 font-semibold tracking-wider text-accent-line uppercase">
        Plan
      </span>
      <p dir="auto" className="mt-1 h-5 truncate text-sm leading-5 font-semibold text-foreground">
        {data.title}
      </p>
      <p dir="auto" className="mt-1 line-clamp-3 h-12 text-xs leading-4 text-muted-foreground">
        {data.goal}
      </p>
      <Handles />
    </div>
  )
}

export function TaskNodeView({ data, selected }: NodeProps<TaskNodeType>) {
  const { task, changed, focused } = data
  const outline = focused
    ? 'border-accent-line ring-2 ring-accent-line/60 shadow-lg'
    : changed
      ? 'border-accent-line/80'
      : selected
        ? 'border-accent-line/70'
        : 'border-accent-line/25 hover:border-accent-line/55'
  return (
    <div
      data-changed={changed || undefined}
      className={`box-border flex h-full w-full cursor-pointer flex-col overflow-hidden rounded-xl border px-3 py-2.5 transition-colors ${
        changed ? 'bg-secondary' : 'bg-card'
      } ${outline}`}
    >
      <div className="flex h-4 items-center justify-between text-2xs leading-4">
        <span dir="auto" className="rounded bg-accent-line/15 px-1.5 font-mono font-semibold text-accent-line">
          T{task.number}
        </span>
        {changed && (
          <span dir="auto" className="rounded bg-accent-line/20 px-1.5 text-secondary-foreground">
            changed
          </span>
        )}
      </div>
      <p dir="auto" title={task.title} className="mt-1 h-5 truncate text-sm leading-5 font-semibold text-foreground">
        {task.title}
      </p>
      <p dir="auto" className="mt-1 line-clamp-2 h-8 text-xs leading-4 text-muted-foreground">
        {task.goal}
      </p>
      {(data.writes.length > 0 || data.needs !== null || data.waits.length > 0) && (
        <div className="mt-1.5 text-2xs leading-4">
          {data.writes.map((path) => (
            <p key={path} dir="auto" title={path} className="h-4 truncate font-mono text-faint-foreground">
              {path}
            </p>
          ))}
          {data.moreWrites > 0 && (
            <p dir="auto" className="h-4 text-faint-foreground">
              +{data.moreWrites} more
            </p>
          )}
          {data.needs !== null && (
            <p dir="auto" className="h-4 truncate text-success">
              {data.needs}
            </p>
          )}
          {data.waits.map((wait) => (
            <p key={wait} dir="auto" className="h-4 truncate text-warning italic">
              {wait}
            </p>
          ))}
        </div>
      )}
      <Handles />
    </div>
  )
}
