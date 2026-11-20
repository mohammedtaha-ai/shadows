// One job: draw laid-out tables or screens on a React Flow canvas, with a list below that is always shown.
import '@xyflow/react/dist/style.css'
import { Controls, type Edge, Handle, MarkerType, MiniMap, type NodeMouseHandler, type NodeProps, Position, ReactFlow } from '@xyflow/react'
import { useMemo } from 'react'
import { THEME } from '@/app/workflows/flow-theme'
import { StateBadge } from './state-badge'
import type { DiagramEdge, ElementNode } from './diagram-layout'

const ARROW = { type: MarkerType.ArrowClosed, width: 14, height: 14, color: 'var(--faint-foreground)' }

function ElementNodeView({ data }: NodeProps<ElementNode>) {
  const { element, state } = data.view
  return (
    <article data-element={element.id} data-element-kind={element.content.kind}
      className="box-border h-full w-full overflow-hidden rounded-lg border border-border bg-card px-3 py-2 text-left">
      <p dir="auto" className="truncate text-sm font-medium">{element.content.name}</p>
      <p className="text-[11px] text-faint-foreground">{element.id} r{element.revision} · as planned</p>
      <StateBadge state={state.state} />
      {state.behind && state.built_revision !== null && (
        <p className="text-[11px] text-muted-foreground">built r{state.built_revision}, newer planned</p>
      )}
      <ul className="mt-1">
        {data.rows.map((row, index) => <li key={index} dir="auto" className="h-[18px] truncate text-[11px] text-muted-foreground">{row}</li>)}
      </ul>
      <Handle type="target" position={Position.Left} isConnectable={false} className="opacity-0" />
      <Handle type="source" position={Position.Right} isConnectable={false} className="opacity-0" />
    </article>
  )
}

const nodeTypes = { element: ElementNodeView }

export function ElementCanvas({ label, nodes, edges, selected, onOpen }: {
  /** The section's aria-label: "ER diagram" or "Screen flow". */
  label: string
  nodes: ElementNode[]
  edges: DiagramEdge[]
  /** The open element's id, drawn selected. */
  selected?: string
  /** Called from onNodeClick. */
  onOpen: (elementId: string) => void
}) {
  const shownNodes = useMemo(() => nodes.map(item => ({ ...item, selected: item.id === selected })), [nodes, selected])
  const shownEdges = useMemo<Edge[]>(() => edges.map(edge => ({
    ...edge, label: edge.data?.label, focusable: false, markerEnd: ARROW,
    style: { stroke: 'var(--muted-foreground)' },
  })), [edges])
  const onNodeClick: NodeMouseHandler = (_event, item) => onOpen(item.id)
  return (
    <section aria-label={label} className="space-y-3">
      <div className="hidden h-[32rem] min-w-0 overflow-hidden rounded-2xl border border-border bg-background sm:block">
        <ReactFlow nodes={shownNodes} edges={shownEdges} nodeTypes={nodeTypes} onNodeClick={onNodeClick}
          nodesDraggable={false} nodesConnectable={false} edgesFocusable={false} colorMode="dark" fitView
          fitViewOptions={{ padding: 0.2, maxZoom: 1 }} minZoom={0.15} style={THEME}>
          <Controls showInteractive={false} />
          <MiniMap pannable zoomable />
        </ReactFlow>
      </div>
    </section>
  )
}
