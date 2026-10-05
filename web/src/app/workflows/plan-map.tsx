// A project's latest-plan map, reusing the workflow graph's layout and edges.

import '@xyflow/react/dist/style.css'
import { useQuery } from '@tanstack/react-query'
import { Link, getRouteApi } from '@tanstack/react-router'
import { Controls, Handle, MarkerType, MiniMap, type Node, type NodeProps, Position, ReactFlow } from '@xyflow/react'
import { useMemo } from 'react'
import type { MapPlan, PlanMap } from '@/api/client'
import { planMapQuery } from '@/api/queries'
import { ErrorLine } from '../error-line'
import { type PlanEdge, placeGraph } from './layout'
import { LinkEdge } from './plan-edge'
import { stateLabel } from './plan-state'

const route = getRouteApi('/projects/$projectId/map')
interface MapData extends Record<string, unknown> { plan: MapPlan; foreign: boolean }
type MapNode = Node<MapData, 'plan'>
const nodeTypes = { plan: MapNodeView }
const edgeTypes = { link: LinkEdge }

export function PlanMapPage() {
  const { projectId } = route.useParams()
  const query = useQuery(planMapQuery(projectId))
  return <section className="flex h-full flex-col" aria-label="Plan map">
    <header className="border-b border-border px-6 py-4">
      <h1 className="text-lg font-semibold">Plan map</h1>
      <p className="mt-1 text-xs text-muted-foreground">Latest plans and their task dependencies. Select a plan to open it.</p>
    </header>
    {query.data === undefined ? <div className="p-6 text-sm text-faint-foreground">
      {query.isError ? <ErrorLine error={query.error} /> : 'Reading the map…'}
    </div> : query.data.plans.length === 0 ? <p className="p-6 text-sm text-faint-foreground">No active plans yet.</p> :
      <MapGraph map={query.data} />}
  </section>
}

function MapGraph({ map }: { map: PlanMap }) {
  const { nodes, edges } = useMemo(() => {
    const nodes: MapNode[] = map.plans.map(plan => ({
      id: plan.plan_id, type: 'plan', position: { x: 0, y: 0 }, width: 256, height: 170,
      data: { plan, foreign: plan.project_id !== map.project_id },
    }))
    const edges: PlanEdge[] = map.links.map(link => ({
      id: `${link.plan_id}-${link.after}`, source: link.after, target: link.plan_id, type: 'link',
      data: { kind: 'needs', label: `${link.count} task link${link.count === 1 ? '' : 's'}${link.broken ? ' · broken' : ''}`, broken: link.broken },
    }))
    placeGraph(nodes, edges)
    return { nodes, edges }
  }, [map])
  return <div data-plan-map className="min-h-0 flex-1">
    <ReactFlow nodes={nodes} edges={edges} nodeTypes={nodeTypes} edgeTypes={edgeTypes}
      nodesDraggable={false} nodesConnectable={false} edgesFocusable={false} colorMode="dark"
      fitView fitViewOptions={{ minZoom: 0.5, maxZoom: 1, padding: 0.2 }} minZoom={0.01}
      defaultEdgeOptions={{ markerEnd: { type: MarkerType.ArrowClosed, color: 'var(--muted-foreground)' } }}>
      <Controls showInteractive={false} />
      <MiniMap pannable zoomable />
    </ReactFlow>
  </div>
}

function MapNodeView({ data }: NodeProps<MapNode>) {
  const { plan, foreign } = data
  const state = plan.state && plan.version != null ? stateLabel({ state: plan.state, version: plan.version }) : 'Unavailable'
  const content = <>
    {foreign && <p dir="auto" className="h-4 truncate text-[11px] text-faint-foreground">{plan.project_name}</p>}
    <p dir="auto" className="mt-1 truncate text-sm font-medium">{plan.title}</p>
    <p dir="auto" className="mt-1 line-clamp-3 h-12 text-xs text-muted-foreground">{plan.goal}</p>
    <p className="mt-2 text-[11px] text-faint-foreground">{state} · {plan.task_count} {plan.task_count === 1 ? 'task' : 'tasks'}</p>
    {plan.plan_state === 'Archived' && <p className="mt-1 text-xs text-faint-foreground">Archived · read only</p>}
    {plan.removed && <p className="mt-1 text-xs text-destructive">Project removed</p>}
    <Handle type="target" position={Position.Left} isConnectable={false} className="opacity-0" />
    <Handle type="source" position={Position.Right} isConnectable={false} className="opacity-0" />
  </>
  const className = `box-border block h-full w-full rounded-lg border bg-card px-3 py-2.5 ${foreign ? 'border-dashed' : ''} ${plan.removed ? 'border-destructive' : 'border-border'}`
  return plan.workflow_id ? <Link to="/projects/$projectId/workflows/$workflowId"
    params={{ projectId: plan.project_id, workflowId: plan.workflow_id }}
    aria-label={`Open plan ${plan.title}`} className={className}>{content}</Link> : <div className={className}>{content}</div>
}
