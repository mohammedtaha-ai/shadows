// One job: place a project's tables or screens as React Flow nodes and edges, with each node's
// and label's size decided here so the box dagre spaced is the box drawn and edges draw without measuring.

import dagre from '@dagrejs/dagre'
import { type Edge, type Node, type NodeHandle, Position } from '@xyflow/react'
import type { ElementView } from '@/api/project-state'

export interface ElementNodeData extends Record<string, unknown> {
  view: ElementView
  /** The text lines the node shows under its name, decided here so its height is known. */
  rows: string[]
}
export type ElementNode = Node<ElementNodeData, 'element'>
export interface DiagramEdgeData extends Record<string, unknown> {
  label: string
  /** The drawn path in flow coordinates: from the source's side, through the label, to the target's side. */
  path: string
  /** The label's centre, where dagre placed it. */
  labelX: number
  labelY: number
}
export type DiagramEdge = Edge<DiagramEdgeData, 'diagram'>

/**
 * The height in pixels of every line a node draws. The node renders each line at exactly this
 * height, and its layout height is their sum, so a node never clips its rows.
 */
export const NODE_LINES = {
  /** 1px border and 8px padding, top and bottom each. */
  frame: 18,
  name: 20,
  meta: 16,
  badge: 20,
  /** Shown only when the element is behind its built revision. */
  behind: 16,
  /** The gap above the rows. */
  gap: 4,
  row: 18,
} as const
/** The box an edge's label is drawn in; dagre spaces the ranks around it. */
export const EDGE_LABEL = { width: 168, height: 40 } as const

const TABLE_WIDTH = 260
const SCREEN_WIDTH = 240
/** How far an edge end stays from a node's top or bottom corner. */
const END_INSET = 12

function handles(width: number, height: number): NodeHandle[] {
  return [
    { type: 'target', position: Position.Left, x: 0, y: height / 2, width: 1, height: 1 },
    { type: 'source', position: Position.Right, x: width - 1, y: height / 2, width: 1, height: 1 },
  ]
}

/** The node's height: the sum of the lines it renders. */
function nodeHeight(view: ElementView, rows: number): number {
  const behind = view.state.behind && view.state.built_revision !== null ? NODE_LINES.behind : 0
  return NODE_LINES.frame + NODE_LINES.name + NODE_LINES.meta + NODE_LINES.badge + behind
    + NODE_LINES.gap + rows * NODE_LINES.row
}

function node(view: ElementView, rows: string[], width: number): ElementNode {
  return {
    id: view.element.id, type: 'element', position: { x: 0, y: 0 }, data: { view, rows },
    width, height: nodeHeight(view, rows.length),
  }
}

interface Box { x: number; y: number; width: number; height: number }

/** Where an edge leaves or enters a node: the side facing the label, at the label's height when it can. */
function end(box: Box, toward: { x: number; y: number }): { x: number; y: number; dir: number } {
  const right = toward.x >= box.x + box.width / 2
  const top = box.y + Math.min(END_INSET, box.height / 2)
  const bottom = box.y + box.height - Math.min(END_INSET, box.height / 2)
  return { x: right ? box.x + box.width : box.x, y: Math.min(Math.max(toward.y, top), bottom), dir: right ? 1 : -1 }
}

const round = (value: number) => Math.round(value * 10) / 10

/** A curve from the source's side through the label's centre to the target's side, level at each point. */
function edgePath(source: Box, target: Box, label: { x: number; y: number }): string {
  const from = end(source, label)
  const to = end(target, label)
  const out = Math.max(24, Math.abs(label.x - from.x) / 2)
  const into = Math.max(24, Math.abs(to.x - label.x) / 2)
  // The curve passes the label level, in the direction from source to target.
  const along = to.x >= from.x ? 1 : -1
  return [
    `M${round(from.x)},${round(from.y)}`,
    `C${round(from.x + from.dir * out)},${round(from.y)} ${round(label.x - along * out)},${round(label.y)} ${round(label.x)},${round(label.y)}`,
    `C${round(label.x + along * into)},${round(label.y)} ${round(to.x + to.dir * into)},${round(to.y)} ${round(to.x)},${round(to.y)}`,
  ].join(' ')
}

/**
 * Runs dagre over the fixed sizes and writes each node's top-left corner, size and handles, and
 * each edge's label centre and path. Every edge is its own dagre edge with a sized label, so two
 * links between the same nodes, either way round, get two label slots and two paths.
 */
function place(nodes: ElementNode[], edges: DiagramEdge[]): void {
  const graph = new dagre.graphlib.Graph({ multigraph: true })
  graph.setGraph({ rankdir: 'LR', nodesep: 32, ranksep: 96, marginx: 0, marginy: 0 })
  for (const item of nodes) graph.setNode(item.id, { width: item.width, height: item.height })
  for (const edge of edges) {
    graph.setEdge(edge.source, edge.target, { width: EDGE_LABEL.width, height: EDGE_LABEL.height, labelpos: 'c' }, edge.id)
  }
  dagre.layout(graph)
  const boxes = new Map<string, Box>()
  for (const item of nodes) {
    const width = item.width ?? 0
    const height = item.height ?? 0
    const { x, y } = graph.node(item.id)
    item.measured = { width, height }
    item.position = { x: x - width / 2, y: y - height / 2 }
    item.handles = handles(width, height)
    boxes.set(item.id, { ...item.position, width, height })
  }
  for (const edge of edges) {
    const { x, y } = graph.edge(edge.source, edge.target, edge.id)
    const source = boxes.get(edge.source)
    const target = boxes.get(edge.target)
    if (edge.data === undefined || source === undefined || target === undefined) continue
    edge.data.labelX = round(x)
    edge.data.labelY = round(y)
    edge.data.path = edgePath(source, target, { x, y })
  }
}

function edge(id: string, source: string, target: string, label: string): DiagramEdge {
  return { id, type: 'diagram', source, target, data: { label, path: '', labelX: 0, labelY: 0 } }
}

/** Tables: one node per table, one edge per foreign key between two shown tables. */
export function tableDiagram(views: ElementView[]): { nodes: ElementNode[]; edges: DiagramEdge[] } {
  const tables = views.filter(view => view.element.content.kind === 'table')
  const shown = new Set(tables.map(view => view.element.id))
  const nameOf = (id: string) => {
    const found = tables.find(view => view.element.id === id)
    return found === undefined ? id : found.element.content.name
  }
  const nodes: ElementNode[] = []
  const edges: DiagramEdge[] = []
  for (const view of tables) {
    const content = view.element.content
    if (content.kind !== 'table') continue
    const source = view.element.id
    const rows = content.columns.map(column => {
      const key = column.primary_key === true ? ' · key' : ''
      const ref = column.references != null ? ` → ${nameOf(column.references.table)}.${column.references.column}` : ''
      return `${column.name} ${column.type_name}${key}${ref}`
    })
    nodes.push(node(view, rows, TABLE_WIDTH))
    for (const column of content.columns) {
      const target = column.references?.table
      if (column.references == null || target === undefined || target === source || !shown.has(target)) continue
      edges.push(edge(`fk:${source}:${column.name}`, source, target,
        `${column.name} → ${nameOf(target)}.${column.references.column}`))
    }
  }
  place(nodes, edges)
  return { nodes, edges }
}

/** Screens: one node per screen, one edge per `leads_to` between two shown screens. */
export function screenDiagram(views: ElementView[]): { nodes: ElementNode[]; edges: DiagramEdge[] } {
  const screens = views.filter(view => view.element.content.kind === 'screen')
  const shown = new Set(screens.map(view => view.element.id))
  const nodes: ElementNode[] = []
  const edges: DiagramEdge[] = []
  for (const view of screens) {
    const content = view.element.content
    if (content.kind !== 'screen') continue
    const source = view.element.id
    // A link to a screen that is not shown cannot be drawn, so its node lists it.
    const hidden = (content.leads_to ?? []).filter(link => !shown.has(link.screen))
      .map(link => `→ ${link.screen} when ${link.when}`)
    nodes.push(node(view, [content.route ?? 'No route', content.purpose, ...hidden], SCREEN_WIDTH))
    ;(content.leads_to ?? []).forEach((link, index) => {
      if (link.screen === source || !shown.has(link.screen)) return
      edges.push(edge(`link:${source}:${index}`, source, link.screen, link.when))
    })
  }
  place(nodes, edges)
  return { nodes, edges }
}
