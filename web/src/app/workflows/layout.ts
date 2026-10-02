// One job: placing a plan's nodes and edges, left to right, with dagre
// (§13.11: laid out automatically, recomputed on every change, never dragged).
//
// Every node's size is decided here, from the lines it will show, and handed
// to React Flow as a fixed `width` and `height`: the box dagre spaced is the
// box drawn, so nothing overlaps, and the edges can be drawn before (or, in a
// test, without) the browser measures anything.

import dagre from '@dagrejs/dagre'
import { type Edge, type Node, type NodeHandle, Position } from '@xyflow/react'
import type { Plan, PlanLink, PlanTask } from '@/api/client'

export type LinkKind = PlanLink['kind']

export interface StartData extends Record<string, unknown> {
  title: string
  goal: string
}

/** What a task node shows, decided here so its height is known. */
export interface TaskData extends Record<string, unknown> {
  task: PlanTask
  /** The last edit changed it (§13.10's `last_edit.changed_tasks`). */
  changed: boolean
  /** Set by `PlanGraph` for the task it centres; never by the layout. */
  focused?: boolean
  /** The paths it writes, at most `MAX_WRITES` of them. */
  writes: string[]
  /** How many more paths it writes than `writes` shows. */
  moreWrites: number
  /** "needs T1, T2", or `null` when it needs nothing. */
  needs: string | null
  /** One "N part(s) wait for T8" per `completes_after` link. */
  waits: string[]
}

export type StartNode = Node<StartData, 'start'>
export type TaskNode = Node<TaskData, 'task'>
export type PlanNode = StartNode | TaskNode

export interface LinkData extends Record<string, unknown> {
  /** `start` for the faint edge from the start node to a task nothing precedes. */
  kind: LinkKind | 'start'
  label: string
  sourceYOffset?: number
  targetYOffset?: number
  stepPosition?: number
}

export type PlanEdge = Edge<LinkData, 'link'>

export const TASK_WIDTH = 256
export const START_WIDTH = 256
/** Border, padding, the `T4` row, the title and two lines of goal. */
const TASK_BASE_HEIGHT = 98
/** The gap above the detail lines. */
const DETAILS_GAP = 6
/** One detail line: a path, the needs, one wait. */
export const LINE_HEIGHT = 16
/** Border, padding, the "Plan" row, the title and three lines of goal. */
const START_HEIGHT = 114
const MAX_WRITES = 3

export function taskNodeId(number: number): string {
  return `t${number}`
}

export function linkId(link: Pick<PlanLink, 'kind' | 'after' | 'task'>): string {
  return `${link.kind}-${link.after}-${link.task}`
}

/** "1 part waits for T8", "2 parts wait for T8". */
export function waitsFor(count: number, after: number): string {
  return count === 1 ? `1 part waits for T${after}` : `${count} parts wait for T${after}`
}

/** The plan's nodes and edges, placed left to right: a task is always right
 * of every task it waits for. Deterministic: the same plan lays out the same. */
export function layoutPlan(plan: Plan): { nodes: PlanNode[]; edges: PlanEdge[] } {
  const changed = new Set(plan.last_edit?.changed_tasks ?? [])
  const start: StartNode = {
    id: 'start',
    type: 'start',
    position: { x: 0, y: 0 },
    data: { title: plan.title, goal: plan.goal },
    width: START_WIDTH,
    height: START_HEIGHT,
  }
  const tasks = [...plan.tasks].sort((a, b) => a.number - b.number).map((task) =>
    taskNode(task, plan.links, changed.has(task.number)),
  )

  const known = new Set(plan.tasks.map((t) => t.number))
  const links = plan.links.filter((l) => known.has(l.task) && known.has(l.after))
  const preceded = new Set(links.map((l) => l.task))
  const edges: PlanEdge[] = [
    ...plan.tasks
      .filter((t) => !preceded.has(t.number))
      .map((t) => edge('start', taskNodeId(t.number), `start-${t.number}`, 'start', '')),
    ...links.map((l) =>
      edge(taskNodeId(l.after), taskNodeId(l.task), linkId(l), l.kind, l.label),
    ),
  ]

  const nodes: PlanNode[] = [start, ...tasks]
  place(nodes, edges)
  return { nodes, edges }
}

function taskNode(task: PlanTask, links: PlanLink[], changed: boolean): TaskNode {
  const own = links.filter((l) => l.task === task.number)
  const needs = own.filter((l) => l.kind === 'needs').map((l) => `T${l.after}`)
  const waits = own
    .filter((l) => l.kind === 'completes_after')
    .map((l) => waitsFor(Math.max(l.waiting_items?.length ?? 0, 1), l.after))
  const writes = task.writes.slice(0, MAX_WRITES)
  const moreWrites = task.writes.length - writes.length
  const data: TaskData = {
    task,
    changed,
    writes,
    moreWrites,
    needs: needs.length === 0 ? null : `needs ${needs.join(', ')}`,
    waits,
  }
  const lines = writes.length + (moreWrites > 0 ? 1 : 0) + (data.needs === null ? 0 : 1) + waits.length
  return {
    id: taskNodeId(task.number),
    type: 'task',
    position: { x: 0, y: 0 },
    data,
    width: TASK_WIDTH,
    height: TASK_BASE_HEIGHT + (lines === 0 ? 0 : DETAILS_GAP + lines * LINE_HEIGHT),
  }
}

function edge(source: string, target: string, id: string, kind: LinkData['kind'], label: string): PlanEdge {
  return { id, source, target, type: 'link', data: { kind, label } }
}

/** Runs dagre over the fixed sizes and writes each node's top-left corner and
 * its handles (in the middle of its left and right sides). */
function place(nodes: PlanNode[], edges: PlanEdge[]): void {
  const graph = new dagre.graphlib.Graph()
  // Wide rank gaps: an edge's label sits halfway between two columns.
  graph.setGraph({ rankdir: 'LR', nodesep: 48, ranksep: 160, marginx: 0, marginy: 0 })
  graph.setDefaultEdgeLabel(() => ({}))
  for (const node of nodes) graph.setNode(node.id, { width: node.width, height: node.height })
  for (const e of edges) graph.setEdge(e.source, e.target)
  dagre.layout(graph)

  const nodeMap = new Map<string, { x: number; y: number; width: number; height: number }>()
  for (const node of nodes) {
    const width = node.width ?? 0
    const height = node.height ?? 0
    const { x, y } = graph.node(node.id)
    node.position = { x: x - width / 2, y: y - height / 2 }
    node.handles = handles(width, height)
    nodeMap.set(node.id, { x, y, width, height })
  }

  // 1. Multi-pin outgoing Y offsets (distribute departure points along source node's height)
  const outgoing = new Map<string, PlanEdge[]>()
  for (const e of edges) {
    const list = outgoing.get(e.source) ?? []
    list.push(e)
    outgoing.set(e.source, list)
  }
  for (const [sourceId, list] of outgoing.entries()) {
    list.sort((a, b) => {
      const targetA = nodeMap.get(a.target)?.y ?? 0
      const targetB = nodeMap.get(b.target)?.y ?? 0
      return targetA - targetB
    })
    const count = list.length
    const sourceNode = nodeMap.get(sourceId)
    const maxHeight = (sourceNode?.height ?? 100) * 0.7
    const step = count > 1 ? Math.min(20, maxHeight / (count - 1)) : 0
    list.forEach((e, idx) => {
      if (e.data) {
        e.data.sourceYOffset = (idx - (count - 1) / 2) * step
      }
    })
  }

  // 2. Multi-pin incoming Y offsets (distribute arrival points along target node's height)
  const incoming = new Map<string, PlanEdge[]>()
  for (const e of edges) {
    const list = incoming.get(e.target) ?? []
    list.push(e)
    incoming.set(e.target, list)
  }
  for (const [targetId, list] of incoming.entries()) {
    list.sort((a, b) => {
      const sourceA = nodeMap.get(a.source)?.y ?? 0
      const sourceB = nodeMap.get(b.source)?.y ?? 0
      return sourceA - sourceB
    })
    const count = list.length
    const targetNode = nodeMap.get(targetId)
    const maxHeight = (targetNode?.height ?? 100) * 0.7
    const step = count > 1 ? Math.min(20, maxHeight / (count - 1)) : 0
    list.forEach((e, idx) => {
      if (e.data) {
        e.data.targetYOffset = (idx - (count - 1) / 2) * step
      }
    })
  }

  // 3. Channel lanes: distribute vertical turn position across edges in each column gap
  const channels = new Map<string, PlanEdge[]>()
  for (const e of edges) {
    const s = nodeMap.get(e.source)
    const t = nodeMap.get(e.target)
    if (!s || !t) continue
    const key = `${Math.round(s.x)}->${Math.round(t.x)}`
    const list = channels.get(key) ?? []
    list.push(e)
    channels.set(key, list)
  }
  for (const list of channels.values()) {
    list.sort((a, b) => {
      const sa = nodeMap.get(a.source)?.y ?? 0
      const ta = nodeMap.get(a.target)?.y ?? 0
      const sb = nodeMap.get(b.source)?.y ?? 0
      const tb = nodeMap.get(b.target)?.y ?? 0
      return (sa + ta) - (sb + tb)
    })
    const count = list.length
    list.forEach((e, idx) => {
      if (e.data) {
        e.data.stepPosition = count === 1 ? 0.5 : 0.2 + (0.6 * (idx + 0.5)) / count
      }
    })
  }
}

function handles(width: number, height: number): NodeHandle[] {
  return [
    { type: 'target', position: Position.Left, x: 0, y: height / 2, width: 1, height: 1 },
    { type: 'source', position: Position.Right, x: width - 1, y: height / 2, width: 1, height: 1 },
  ]
}
