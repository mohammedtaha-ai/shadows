// Display projections of the server's phase order and exact dependency previews.
import type { Phase, PhaseEndpoint, Plan } from '@/api/workflows'
import { layoutPlan, placeGraph, type PlanEdge, type PlanNode } from './layout'
import type { PhaseNode } from './phase-node'

export type PhaseGraphNode = PlanNode | PhaseNode
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

export function layoutTask(plan: Plan, number: number): { nodes: PhaseGraphNode[]; edges: PlanEdge[] } {
  const task = plan.tasks.find(t => t.number === number)
  const root = layoutPlan(plan).nodes.find(n => n.type === 'task' && n.data.task.number === number)
  if (!task || !root) return { nodes: [], edges: [] }
  const nodes = new Map<string, PhaseGraphNode>([[root.id, root]])
  const edges = new Map<string, PlanEdge>()
  const addEdge = (source: string, target: string, kind: 'phase_order' | 'phase_wait', broken = false) => {
    const id = `${kind}:${source}:${target}`
    edges.set(id, { id, source, target, type: 'link', data: {
      kind, label: kind === 'phase_order' ? 'then' : 'waits for', broken,
    } })
  }
  let previous = root.id
  for (const phase of task.phases ?? []) {
    const node = phaseNode(phaseEndpoint(plan, number, phase), phase, false)
    nodes.set(node.id, node)
    addEdge(previous, node.id, 'phase_order')
    previous = node.id
  }
  for (const phase of task.phases ?? []) {
    for (const reference of phase.waits_for) {
      if (reference.kind !== 'local' || reference.task !== number) continue
      const target = task.phases?.find(p => p.id === reference.phase)
      if (target) addEdge(phaseNodeId(phaseEndpoint(plan, number, target)),
        phaseNodeId(phaseEndpoint(plan, number, phase)), 'phase_wait')
    }
  }
  const ensure = (endpoint: PhaseEndpoint) => {
    const id = phaseNodeId(endpoint)
    if (!nodes.has(id)) nodes.set(id, phaseNode(endpoint, undefined, true))
    return id
  }
  for (const preview of plan.linked_phases ?? []) {
    const sourceHere = preview.source.workflow_id === plan.id && preview.source.task === number
    const targetHere = preview.target?.workflow_id === plan.id && preview.target.task === number
    if (!sourceHere && !targetHere) continue
    const source = ensure(preview.source)
    let target: string
    if (preview.target) target = ensure(preview.target)
    else {
      target = `missing:${source}:${JSON.stringify(preview.reference)}`
      nodes.set(target, { id: target, type: 'phase', position: { x: 0, y: 0 },
        width: PHASE_WIDTH, height: PHASE_HEIGHT,
        data: { foreign: true, broken: preview.broken ?? 'Phase is unavailable' } })
    }
    addEdge(target, source, 'phase_wait', preview.broken != null)
  }
  const result = { nodes: [...nodes.values()], edges: [...edges.values()] }
  placeGraph(result.nodes, result.edges)
  return result
}
