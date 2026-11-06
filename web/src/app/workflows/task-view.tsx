import { Controls, MarkerType, MiniMap, Panel, ReactFlow } from '@xyflow/react'
import { useEffect, useMemo, useRef } from 'react'
import type { PhaseEndpoint, Plan, PlanTask } from '@/api/workflows'
import { Button } from '@/components/ui/button'
import { InspectPanel } from './inspect-panel'
import { layoutTask, type PhaseGraphNode } from './phase-layout'
import { PhaseNodeView, type PhaseNode } from './phase-node'
import { StartNodeView, TaskNodeView } from './task-node'
import { LinkedNodeView, MissingTaskNodeView } from './linked-node'
import { LinkEdge } from './plan-edge'
import { PlanLegend } from './plan-graph'
import { THEME } from './flow-theme'
import { MIN_ZOOM, WHOLE_PLAN, useCamera, useCanvasSize, type CanvasSize } from './plan-camera'
import type { PlanEdge } from './layout'

const nodeTypes = { phase: PhaseNodeView, task: TaskNodeView, start: StartNodeView,
  linked: LinkedNodeView, missing: MissingTaskNodeView }
const edgeTypes = { link: LinkEdge }

export function TaskView({ plan, task, onBack, onOpenTask }: {
  plan: Plan; task: PlanTask; onBack: () => void; onOpenTask: (endpoint: PhaseEndpoint) => void
}) {
  const graph = useMemo(() => layoutTask(plan, task.number), [plan, task.number])
  const back = useRef<HTMLButtonElement>(null)
  const canvas = useRef<HTMLDivElement>(null)
  const size = useCanvasSize(canvas)
  useEffect(() => { back.current?.focus() }, [task.number])
  const related = graph.nodes.filter((n): n is PhaseNode => n.type === 'phase' && n.data.foreign)
  return (
    <section aria-label={`Task T${task.number} phases`} className="flex h-full min-h-0 flex-col bg-background"
      onKeyDown={event => { if (event.key === 'Escape') { event.stopPropagation(); onBack() } }}>
      <header className="flex flex-wrap items-center gap-3 border-b border-border px-4 py-3">
        <Button ref={back} variant="outline" size="sm" onClick={onBack}>Back to plan</Button>
        <h2 dir="auto" className="min-w-0 truncate text-sm font-semibold">T{task.number} · {task.title}</h2>
      </header>
      <div className="flex min-h-0 flex-1 flex-col overflow-y-auto lg:flex-row lg:overflow-hidden">
        <div className="flex min-h-96 min-w-0 flex-1 flex-col lg:min-h-0">
          {(task.phases?.length ?? 0) === 0 ? (
            <p className="px-4 py-3 text-sm text-muted-foreground">This task has no phases</p>
          ) : <ol aria-label="Phase order" className="flex flex-wrap gap-3 border-b border-border px-4 py-2 text-xs">
            {task.phases?.map(phase => <li key={phase.id} dir="auto">
              <span className="font-mono text-accent-line">T{task.number} · P{phase.number}</span> {phase.title}
            </li>)}
          </ol>}
          {related.length > 0 && <nav aria-label="Phase dependencies" className="flex flex-wrap gap-2 px-4 py-2 text-xs">
            {related.map(node => node.data.endpoint ? <Button key={node.id} size="sm" variant="outline"
              className="border-dashed" onClick={() => node.data.endpoint && onOpenTask(node.data.endpoint)}>
              Open T{node.data.endpoint.task} · P{node.data.endpoint.phase_number} · {node.data.endpoint.plan_title} v{node.data.endpoint.version}
            </Button> : <p role="alert" key={node.id} className="text-destructive">{node.data.broken}</p>)}
          </nav>}
          <div ref={canvas} className="min-h-72 flex-1" data-task-canvas>
            <ReactFlow<PhaseGraphNode, PlanEdge> nodes={graph.nodes} edges={graph.edges}
              nodeTypes={nodeTypes} edgeTypes={edgeTypes} style={THEME} colorMode="dark"
              nodesDraggable={false} nodesConnectable={false} minZoom={MIN_ZOOM}
              fitView fitViewOptions={{ minZoom: 0.5, maxZoom: 1, padding: 0.2 }}
              defaultEdgeOptions={{ markerEnd: { type: MarkerType.ArrowClosed } }}
              onNodeClick={(_event, node) => {
                if (node.type === 'phase' && node.data.foreign && node.data.endpoint) onOpenTask(node.data.endpoint)
              }}>
              <Controls showInteractive={false} fitViewOptions={WHOLE_PLAN} />
              <MiniMap pannable zoomable />
              <TaskCamera size={size} nodes={graph.nodes} />
              <Panel position="top-right"><PlanLegend phases className="rounded-md border border-border bg-background/90 px-2 py-1" /></Panel>
            </ReactFlow>
          </div>
        </div>
        <div className="shrink-0 lg:h-full lg:w-80 [&>aside]:h-full [&>aside]:w-full">
          <InspectPanel plan={plan} task={task} onClose={onBack} />
        </div>
      </div>
    </section>
  )
}

function TaskCamera({ size, nodes }: { size: CanvasSize | null; nodes: PhaseGraphNode[] }) {
  useCamera({ size, nodes, compact: false, focusTask: undefined, lastClicked: null })
  return null
}
