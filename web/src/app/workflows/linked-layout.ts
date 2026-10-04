// Related tasks keep plan-qualified identities in the current plan's graph.

import type { Node } from '@xyflow/react'
import type { LinkedTask, Plan, PlanLink } from '@/api/client'
import type { PlanEdge } from './layout'

export interface LinkedData extends Record<string, unknown> {
  view: LinkedTask
  foreign: boolean
}
export type LinkedNode = Node<LinkedData, 'linked'>
export interface MissingData extends Record<string, unknown> { number: number; reason: string }
export type MissingNode = Node<MissingData, 'missing'>

export function parentName(after: PlanLink['after'], views: LinkedTask[] = []): string {
  if (typeof after === 'number') return `T${after}`
  const view = views.find(view => !view.incoming && view.plan_id === after.plan_id)
  return `${view?.plan_title ?? `plan ${after.plan_id}`} · T${after.task}`
}

export function linkedLayout(plan: Plan): { nodes: (LinkedNode | MissingNode)[]; edges: PlanEdge[] } {
  const nodes = new Map<string, LinkedNode>()
  const missing = new Map<string, MissingNode>()
  const edges: PlanEdge[] = []
  const known = new Set(plan.tasks.map(task => task.number))
  for (const view of plan.linked_tasks ?? []) {
    const number = view.incoming ? view.link.task :
      typeof view.link.after === 'number' ? view.link.after : view.link.after.task
    const own = view.incoming ?
      typeof view.link.after === 'number' ? view.link.after : view.link.after.task : view.link.task
    if (!known.has(own)) {
      if (!view.incoming) continue
      missing.set(`t${own}`, {
        id: `t${own}`, type: 'missing', position: { x: 0, y: 0 }, width: 256, height: 130,
        data: { number: own, reason: view.broken ?? `T${own} is missing from this version` },
      })
    }
    const id = `p${view.plan_id}-t${number}`
    const existing = nodes.get(id)
    // Several edges may share a task; retain any broken explanation.
    if (!existing || (!existing.data.view.broken && view.broken)) nodes.set(id, {
      id, type: 'linked', position: { x: 0, y: 0 }, width: 256, height: 190,
      data: { view, foreign: view.project_id !== plan.project_id },
    })
    edges.push({
      id: `${view.incoming ? 'in' : 'out'}-${id}-${own}-${view.link.kind}`,
      source: view.incoming ? `t${own}` : id,
      target: view.incoming ? id : `t${own}`,
      type: 'link', data: { kind: view.link.kind, label: view.link.label, broken: view.broken !== null },
    })
  }
  return { nodes: [...nodes.values(), ...missing.values()], edges }
}
