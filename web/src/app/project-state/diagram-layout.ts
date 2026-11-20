// One job: place a project's tables or screens as React Flow nodes and edges, with each node's
// size decided here so the box dagre spaced is the box drawn and edges draw without measuring.

import dagre from '@dagrejs/dagre'
import { type Edge, type Node, type NodeHandle, Position } from '@xyflow/react'
import type { ElementView } from '@/api/project-state'

export interface ElementNodeData extends Record<string, unknown> {
  view: ElementView
  /** The text lines the node shows under its name, decided here so its height is known. */
  rows: string[]
}
export type ElementNode = Node<ElementNodeData, 'element'>
export type DiagramEdge = Edge<{ label: string }>

const TABLE_WIDTH = 260
const SCREEN_WIDTH = 240
const BASE_HEIGHT = 64
const ROW_HEIGHT = 18

function handles(width: number, height: number): NodeHandle[] {
  return [
    { type: 'target', position: Position.Left, x: 0, y: height / 2, width: 1, height: 1 },
    { type: 'source', position: Position.Right, x: width - 1, y: height / 2, width: 1, height: 1 },
  ]
}

function node(view: ElementView, rows: string[], width: number): ElementNode {
  return {
    id: view.element.id, type: 'element', position: { x: 0, y: 0 }, data: { view, rows },
    width, height: BASE_HEIGHT + rows.length * ROW_HEIGHT,
  }
}

/** Runs dagre over the fixed sizes and writes each node's top-left corner, size and handles. */
function place(nodes: ElementNode[], edges: DiagramEdge[]): void {
  const graph = new dagre.graphlib.Graph()
  graph.setGraph({ rankdir: 'LR', nodesep: 48, ranksep: 120, marginx: 0, marginy: 0 })
  graph.setDefaultEdgeLabel(() => ({}))
  for (const item of nodes) graph.setNode(item.id, { width: item.width, height: item.height })
  for (const edge of edges) graph.setEdge(edge.source, edge.target)
  dagre.layout(graph)
  for (const item of nodes) {
    const width = item.width ?? 0
    const height = item.height ?? 0
    const { x, y } = graph.node(item.id)
    item.measured = { width, height }
    item.position = { x: x - width / 2, y: y - height / 2 }
    item.handles = handles(width, height)
  }
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
      edges.push({
        id: `fk:${source}:${column.name}`, source, target,
        data: { label: `${column.name} → ${nameOf(target)}.${column.references.column}` },
      })
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
    nodes.push(node(view, [content.route ?? 'No route', content.purpose], SCREEN_WIDTH))
    ;(content.leads_to ?? []).forEach((link, index) => {
      if (link.screen === source || !shown.has(link.screen)) return
      edges.push({ id: `link:${source}:${index}`, source, target: link.screen, data: { label: link.when } })
    })
  }
  place(nodes, edges)
  return { nodes, edges }
}
