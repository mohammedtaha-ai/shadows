import '@xyflow/react/dist/style.css'
import {
  Controls,
  MarkerType,
  MiniMap,
  type NodeMouseHandler,
  Panel,
  ReactFlow,
  type Viewport,
} from '@xyflow/react'
import { useMemo, useRef, useState } from 'react'
import type { PhaseEndpoint, Plan, PlanTask } from '@/api/workflows'
import { type PlanEdge, layoutPlan } from './layout'
import { layoutExpandedPlan, type PhaseGraphNode } from './phase-layout'
import { BoundaryNodeView, PhaseNodeView, TaskGroupView } from './phase-node'
import { type CanvasSize, MIN_ZOOM, WHOLE_PLAN, firstFit, useCamera, useCanvasSize } from './plan-camera'
import { THEME } from './flow-theme'
import { LinkEdge } from './plan-edge'
import { StartNodeView, TaskNodeView } from './task-node'
import { LinkedNodeView, MissingTaskNodeView } from './linked-node'

const nodeTypes = { start: StartNodeView, task: TaskNodeView, linked: LinkedNodeView,
  missing: MissingTaskNodeView, phase: PhaseNodeView, task_group: TaskGroupView, boundary: BoundaryNodeView }
const edgeTypes = { link: LinkEdge }


const ARROW = { type: MarkerType.ArrowClosed, width: 14, height: 14, color: 'var(--faint-foreground)' }

export function PlanGraph({
  plan,
  compact = false,
  focusTask,
  onSelectTask,
  onOpenTask,
  showPhases = false,
  initialViewport,
  onViewportChange,
}: {
  plan: Plan
  /** A card: no minimap, fixed height. */
  compact?: boolean
  /** Centre and outline this task. */
  focusTask?: number
  onSelectTask?: (task: PlanTask) => void
  onOpenTask?: (endpoint: PhaseEndpoint) => void
  showPhases?: boolean
  initialViewport?: Viewport
  onViewportChange?: (viewport: Viewport) => void
}) {
  // Recompute positions from the plan so every surface shows the same automatic layout.
  const { nodes, edges } = useMemo(() => showPhases ? layoutExpandedPlan(plan) : layoutPlan(plan), [plan, showPhases])
  const shown = useMemo(
    () =>
      focusTask === undefined
        ? nodes
        : nodes.map((n): PhaseGraphNode => {
            if (n.type === 'task' && n.data.task.number === focusTask) {
              return { ...n, data: { ...n.data, focused: true } }
            }
            if (n.type === 'task_group' && n.data.task.number === focusTask) {
              return { ...n, data: { ...n.data, focused: true } }
            }
            return n
          }),
    [nodes, focusTask],
  )
  const [lastClicked, setLastClicked] = useState<number | null>(null)
  const onNodeClick: NodeMouseHandler<PhaseGraphNode> = (_event, node) => {
    if (node.type === 'phase' && node.data.foreign && node.data.endpoint) {
      onOpenTask?.(node.data.endpoint)
      return
    }
    const number = node.type === 'task' || node.type === 'task_group' ? node.data.task.number
      : node.type === 'phase' ? node.data.endpoint?.task
      : node.type === 'boundary' ? node.data.task : undefined
    const task = plan.tasks.find(t => t.number === number)
    if (!task) return
    setLastClicked(task.number)
    onSelectTask?.(task)
  }
  const canvas = useRef<HTMLDivElement>(null)
  const size = useCanvasSize(canvas)

  const flow = (
    <div ref={canvas} data-plan-canvas className={compact ? 'h-72 w-full' : 'h-full w-full'}>
      <ReactFlow<PhaseGraphNode, PlanEdge>
        nodes={shown}
        edges={edges}
        nodeTypes={nodeTypes}
        edgeTypes={edgeTypes}
        defaultEdgeOptions={{ markerEnd: ARROW }}
        onNodeClick={onNodeClick}
        nodesDraggable={false}
        nodesConnectable={false}
        edgesFocusable={false}
        defaultViewport={initialViewport}
        onMoveEnd={(_event, viewport) => onViewportChange?.(viewport)}
        colorMode="dark"
        fitView={initialViewport === undefined}
        fitViewOptions={firstFit(compact, focusTask)}
        minZoom={MIN_ZOOM}
        style={THEME}
      >
        <Controls showInteractive={false} fitViewOptions={WHOLE_PLAN} />
        {!compact && <MiniMap pannable zoomable />}
        {!compact && (
          <Panel position="top-right">
            <PlanLegend phases={showPhases} className="rounded-md border border-border bg-background/90 px-2 py-1" />
          </Panel>
        )}
        <Camera size={size} compact={compact} focusTask={focusTask} lastClicked={lastClicked}
          nodes={shown} restored={initialViewport !== undefined} />
      </ReactFlow>
    </div>
  )
  if (!compact) return flow
  // A card is too short for a floating legend: over its zoomed-in graph it
  // would hide the nodes at its top right, so it sits under the canvas.
  return (
    <div>
      {flow}
      <PlanLegend phases={showPhases} className="border-t border-border px-3 py-1.5" />
    </div>
  )
}

export function PlanLegend({ className, phases = false }: { className: string; phases?: boolean }) {
  return (
    <div data-plan-legend className={className}>
      <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-[11px] text-muted-foreground">
        {phases && <span>then: phase order · waits for: phase prerequisite</span>}
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
  nodes: PhaseGraphNode[]
  restored: boolean
}) {
  useCamera(props)
  return null
}
