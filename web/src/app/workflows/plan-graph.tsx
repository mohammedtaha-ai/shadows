// One job: a plan version drawn as its graph — the canvas the Workflows page,
// the conversation's plan cards and its side panel all show (§13.9, §13.11).

import '@xyflow/react/dist/style.css'
import {
  Controls,
  MarkerType,
  MiniMap,
  type NodeMouseHandler,
  Panel,
  ReactFlow,
} from '@xyflow/react'
import { type CSSProperties, useMemo, useRef, useState } from 'react'
import type { Plan, PlanTask } from '@/api/client'
import { type PlanNode, layoutPlan } from './layout'
import { type CanvasSize, MIN_ZOOM, WHOLE_PLAN, firstFit, useCamera, useCanvasSize } from './plan-camera'
import { LinkEdge } from './plan-edge'
import { StartNodeView, TaskNodeView } from './task-node'
import { LinkedNodeView, MissingTaskNodeView } from './linked-node'

const nodeTypes = { start: StartNodeView, task: TaskNodeView, linked: LinkedNodeView, missing: MissingTaskNodeView }
const edgeTypes = { link: LinkEdge }

/** React Flow's own colours, taken from Shadows' theme tokens. */
const THEME = {
  '--xy-background-color': 'var(--background)',
  '--xy-edge-label-background-color': 'var(--background)',
  '--xy-minimap-background-color': 'var(--sidebar)',
  '--xy-minimap-node-background-color': 'var(--muted)',
  '--xy-controls-button-background-color': 'var(--card)',
  '--xy-controls-button-background-color-hover': 'var(--muted)',
  '--xy-controls-button-color': 'var(--muted-foreground)',
  '--xy-controls-button-color-hover': 'var(--foreground)',
  '--xy-controls-button-border-color': 'var(--border)',
  '--xy-attribution-background-color': 'transparent',
} as CSSProperties

const ARROW = { type: MarkerType.ArrowClosed, width: 14, height: 14, color: 'var(--faint-foreground)' }

export function PlanGraph({
  plan,
  compact = false,
  focusTask,
  onSelectTask,
}: {
  plan: Plan
  /** A card: no minimap, fixed height. */
  compact?: boolean
  /** Centre and outline this task. */
  focusTask?: number
  onSelectTask?: (task: PlanTask) => void
}) {
  // Laid out again on every change (§13.11); nodes are never dragged.
  const { nodes, edges } = useMemo(() => layoutPlan(plan), [plan])
  const shown = useMemo(
    () =>
      focusTask === undefined
        ? nodes
        : nodes.map((n): PlanNode =>
            n.type === 'task' && n.data.task.number === focusTask
              ? { ...n, data: { ...n.data, focused: true } }
              : n,
          ),
    [nodes, focusTask],
  )
  const [lastClicked, setLastClicked] = useState<number | null>(null)
  const onNodeClick: NodeMouseHandler<PlanNode> = (_event, node) => {
    if (node.type !== 'task') return
    setLastClicked(node.data.task.number)
    onSelectTask?.(node.data.task)
  }
  const canvas = useRef<HTMLDivElement>(null)
  const size = useCanvasSize(canvas)

  const flow = (
    <div ref={canvas} data-plan-canvas className={compact ? 'h-72 w-full' : 'h-full w-full'}>
      <ReactFlow
        nodes={shown}
        edges={edges}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        defaultEdgeOptions={{ markerEnd: ARROW }}
        onNodeClick={onNodeClick}
        nodesDraggable={false}
        nodesConnectable={false}
        edgesFocusable={false}
        colorMode="dark"
        fitView
        fitViewOptions={firstFit(compact, focusTask)}
        minZoom={MIN_ZOOM}
        style={THEME}
      >
        <Controls showInteractive={false} fitViewOptions={WHOLE_PLAN} />
        {!compact && <MiniMap pannable zoomable />}
        {!compact && (
          <Panel position="top-right">
            <Legend className="rounded-md border border-border bg-background/90 px-2 py-1" />
          </Panel>
        )}
        <Camera size={size} compact={compact} focusTask={focusTask} lastClicked={lastClicked} nodes={shown} />
      </ReactFlow>
    </div>
  )
  if (!compact) return flow
  // A card is too short for a floating legend: over its zoomed-in graph it
  // would hide the nodes at its top right, so it sits under the canvas.
  return (
    <div>
      {flow}
      <Legend className="border-t border-border px-3 py-1.5" />
    </div>
  )
}

/** Explains the two kinds of link (§13.11). */
function Legend({ className }: { className: string }) {
  return (
    <div data-plan-legend className={className}>
      <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-[11px] text-muted-foreground">
        <span className="flex items-center gap-1.5">
          <svg width="22" height="6" aria-hidden>
            <line x1="0" y1="3" x2="22" y2="3" stroke="var(--accent-line)" strokeWidth="1.5" />
          </svg>
          needs: starts after
        </span>
        <span className="flex items-center gap-1.5">
          <svg width="22" height="6" aria-hidden>
            <line
              x1="0"
              y1="3"
              x2="22"
              y2="3"
              stroke="var(--muted-foreground)"
              strokeWidth="1.5"
              strokeDasharray="6 5"
            />
          </svg>
          completes after: parts wait
        </span>
      </div>
    </div>
  )
}

/** The view's behaviour, where `useReactFlow` can reach the canvas. */
function Camera(props: {
  size: CanvasSize | null
  compact: boolean
  focusTask: number | undefined
  lastClicked: number | null
  nodes: PlanNode[]
}) {
  useCamera(props)
  return null
}
