// Display projections of the server's phase order and exact dependency previews.
import type { Phase, PhaseEndpoint, Plan } from '@/api/workflows'
import dagre from '@dagrejs/dagre'
import { handles, layoutPlan, placeGraph, type PlanEdge, type PlanNode } from './layout'
import type { PhaseNode, TaskGroupNode, BoundaryNode } from './phase-node'

export type PhaseGraphNode = PlanNode | PhaseNode | TaskGroupNode | BoundaryNode
export const PHASE_WIDTH = 256
export const PHASE_HEIGHT = 132

export function phaseNodeId(endpoint: PhaseEndpoint): string {
  return `${endpoint.workflow_id}:v${endpoint.version}:t${endpoint.task}:phase:${endpoint.phase_id}`
}

export function phaseEndpoint(plan: Plan, task: number, phase: Phase): PhaseEndpoint {
  return { plan_id: plan.plan_id, workflow_id: plan.id, version: plan.version,
    plan_title: plan.title, task, phase_id: phase.id, phase_number: phase.number, title: phase.title }
}

function phaseNode(endpoint: PhaseEndpoint, phase: Phase | undefined, foreign: boolean): PhaseNode {
  return { id: phaseNodeId(endpoint), type: 'phase', position: { x: 0, y: 0 },
    width: PHASE_WIDTH, height: PHASE_HEIGHT, data: { endpoint, phase, foreign } }
}

function addPhaseEdge(edges: Map<string, PlanEdge>, source: string, target: string,
  kind: 'phase_order' | 'phase_wait', broken = false): void {
  const id = `${kind}:${source}:${target}`
  edges.set(id, { id, source, target, type: 'link', data: {
    kind, label: kind === 'phase_order' ? 'then' : 'waits for', broken,
  } })
}

/** Both surfaces consume the same server previews without resolving foreign versions. */
function addPreviews(plan: Plan, nodes: Map<string, PhaseGraphNode>, edges: Map<string, PlanEdge>, task?: number): void {
  const ensure = (endpoint: PhaseEndpoint) => {
    const id = phaseNodeId(endpoint)
    if (!nodes.has(id)) nodes.set(id, phaseNode(endpoint, undefined, true))
    return id
  }
  for (const preview of plan.linked_phases ?? []) {
    if (task !== undefined) {
      const sourceHere = preview.source.workflow_id === plan.id && preview.source.task === task
      const targetHere = preview.target?.workflow_id === plan.id && preview.target.task === task
      if (!sourceHere && !targetHere) continue
    }
    const waiting = ensure(preview.source)
    let prerequisite: string
    if (preview.target) prerequisite = ensure(preview.target)
    else {
      prerequisite = `missing:${waiting}:${JSON.stringify(preview.reference)}`
      nodes.set(prerequisite, { id: prerequisite, type: 'phase', position: { x: 0, y: 0 },
        width: PHASE_WIDTH, height: PHASE_HEIGHT,
        data: { foreign: true, broken: preview.broken ?? 'Phase is unavailable' } })
    }
    addPhaseEdge(edges, prerequisite, waiting, 'phase_wait', preview.broken != null)
  }
}

export function layoutTask(plan: Plan, number: number): { nodes: PhaseGraphNode[]; edges: PlanEdge[] } {
  const task = plan.tasks.find(t => t.number === number)
  const root = layoutPlan(plan).nodes.find(n => n.type === 'task' && n.data.task.number === number)
  if (!task || !root) return { nodes: [], edges: [] }
  const nodes = new Map<string, PhaseGraphNode>([[root.id, root]])
  const edges = new Map<string, PlanEdge>()
  let previous = root.id
  for (const phase of task.phases ?? []) {
    const node = phaseNode(phaseEndpoint(plan, number, phase), phase, false)
    nodes.set(node.id, node)
    addPhaseEdge(edges, previous, node.id, 'phase_order')
    previous = node.id
  }
  for (const phase of task.phases ?? []) {
    for (const reference of phase.waits_for) {
      if (reference.kind !== 'local' || reference.task !== number) continue
      const target = task.phases?.find(p => p.id === reference.phase)
      if (target) addPhaseEdge(edges, phaseNodeId(phaseEndpoint(plan, number, target)),
        phaseNodeId(phaseEndpoint(plan, number, phase)), 'phase_wait')
    }
  }
  addPreviews(plan, nodes, edges, number)
  const result = { nodes: [...nodes.values()], edges: [...edges.values()] }
  placeGraph(result.nodes, result.edges)
  return result
}

export function boundaryNodeId(task: number, event: 'start' | 'complete'): string {
  return `t${task}:${event}`
}

/** Compound layout keeps phase edges at phase endpoints, including interleaved tasks. */
export function layoutExpandedPlan(plan: Plan): { nodes: PhaseGraphNode[]; edges: PlanEdge[] } {
  const base = layoutPlan(plan)
  const groups = new Set(plan.tasks.filter(t => (t.phases?.length ?? 0) > 0).map(t => t.number))
  const nodes = new Map<string, PhaseGraphNode>(base.nodes.map(n => [n.id, n]))
  const edges = new Map<string, PlanEdge>()
  for (const task of plan.tasks) {
    if (!groups.has(task.number)) continue
    const parent = `t${task.number}`
    nodes.set(parent, { id: parent, type: 'task_group', position: { x: 0, y: 0 }, data: { task } })
    for (const event of ['start', 'complete'] as const) {
      const id = boundaryNodeId(task.number, event)
      nodes.set(id, { id, type: 'boundary', parentId: parent, extent: 'parent',
        position: { x: 0, y: 0 }, width: 80, height: 28, data: { task: task.number, event } })
    }
    let previous = boundaryNodeId(task.number, 'start')
    for (const phase of task.phases ?? []) {
      const node = { ...phaseNode(phaseEndpoint(plan, task.number, phase), phase, false),
        parentId: parent, extent: 'parent' as const }
      nodes.set(node.id, node)
      addPhaseEdge(edges, previous, node.id, 'phase_order')
      previous = node.id
    }
    addPhaseEdge(edges, previous, boundaryNodeId(task.number, 'complete'), 'phase_order')
  }
  for (const edge of base.edges) {
    const source = base.nodes.find(n => n.id === edge.source)
    const target = base.nodes.find(n => n.id === edge.target)
    edges.set(edge.id, { ...edge,
      source: source?.type === 'task' && groups.has(source.data.task.number)
        ? boundaryNodeId(source.data.task.number, 'complete') : edge.source,
      target: target?.type === 'task' && groups.has(target.data.task.number)
        ? boundaryNodeId(target.data.task.number, edge.data?.kind === 'completes_after' ? 'complete' : 'start') : edge.target,
      data: { ...edge.data!, sourceYOffset: 0, targetYOffset: 0, stepPosition: 0.5 },
    })
  }
  for (const task of plan.tasks) for (const phase of task.phases ?? []) {
    for (const reference of phase.waits_for) {
      if (reference.kind !== 'local') continue
      const target = plan.tasks.find(t => t.number === reference.task)?.phases?.find(p => p.id === reference.phase)
      if (target) addPhaseEdge(edges, phaseNodeId(phaseEndpoint(plan, reference.task, target)),
        phaseNodeId(phaseEndpoint(plan, task.number, phase)), 'phase_wait')
    }
  }
  addPreviews(plan, nodes, edges)
  const result = { nodes: [...nodes.values()], edges: [...edges.values()] }
  placeCompound(result.nodes, result.edges)
  return result
}

function placeCompound(nodes: PhaseGraphNode[], edges: PlanEdge[]): void {
  const graph = new dagre.graphlib.Graph({ compound: true })
  graph.setGraph({ rankdir: 'LR', nodesep: 80, ranksep: 120, marginx: 0, marginy: 0 })
  graph.setDefaultEdgeLabel(() => ({}))
  for (const node of nodes) graph.setNode(node.id, node.type === 'task_group'
    ? {} : { width: node.width, height: node.height })
  for (const node of nodes) if (node.parentId) graph.setParent(node.id, node.parentId)
  for (const edge of edges) graph.setEdge(edge.source, edge.target)
  dagre.layout(graph)
  for (const node of nodes) {
    const box = graph.node(node.id)
    node.width = box.width
    node.height = box.height
    node.measured = { width: box.width, height: box.height }
    let x = box.x - box.width / 2
    let y = box.y - box.height / 2
    if (node.parentId) {
      const parent = graph.node(node.parentId)
      x -= parent.x - parent.width / 2
      y -= parent.y - parent.height / 2
    }
    node.position = { x, y }
    node.handles = handles(box.width, box.height)
  }
}
