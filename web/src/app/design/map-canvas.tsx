import '@xyflow/react/dist/style.css'
import { Controls, type Edge, MarkerType, MiniMap, type Node, Panel, ReactFlow } from '@xyflow/react'
import { Link } from '@tanstack/react-router'
import { type CSSProperties, useMemo } from 'react'
import type { EffectiveStandards, Part } from '@/api/design'

type PartKind = EffectiveStandards['base']['parts'][number]

/** React Flow's own colours, taken from Shadows' theme tokens, as the plan graph sets them. */
const THEME = {
  '--xy-background-color': 'var(--background)',
  '--xy-node-background-color': 'var(--card)',
  '--xy-node-color': 'var(--foreground)',
  '--xy-node-border': '1px solid var(--border)',
  '--xy-edge-label-background-color': 'var(--background)',
  '--xy-minimap-background-color': 'var(--sidebar)',
  '--xy-minimap-node-background-color': 'var(--muted)',
  '--xy-controls-button-background-color': 'var(--card)',
  '--xy-controls-button-background-color-hover': 'var(--muted)',
  '--xy-controls-button-color': 'var(--muted-foreground)',
  '--xy-controls-button-color-hover': 'var(--foreground)',
  '--xy-controls-button-border-color': 'var(--border)',
  '--xy-attribution-background-color': 'transparent',
} as CSSProperties

const ARROW = { type: MarkerType.ArrowClosed, width: 14, height: 14, color: 'var(--faint-foreground)' }
const DASH = '6 5'

/** One permission between two shown parts: which of the user's layers may use which of the target's. */
type Allowed = { from: Part; to: Part; layers: string[] }

/**
 * The visible ownership scope. Data and pagination belong to its caller. `allowed` is the
 * standards' part catalog when the caller shows the permission overlay, and undefined otherwise.
 */
export function MapCanvas({parts,projectId,selected,allowed}: {
  parts:Part[];projectId:string;selected?:string;allowed?:PartKind[]
}) {
  const permissions = useMemo(() => allowed ? permitted(parts,allowed) : [],[parts,allowed])
  const nodes = useMemo<Node[]>(() => {
    // Persistence sits on the bottom rank; edge meaning never depends on position.
    const shown = [...parts].sort((a,b) => Number(a.content.kind === 'database') - Number(b.content.kind === 'database'))
    return shown.map((part,index) => ({id:part.id,
      position:{x:(index % 2)*310,y:Math.floor(index/2)*225},
      selected:part.id === selected,
      style:{width:280,borderRadius:16,padding:0,textAlign:'left'},
      data:{label:<article data-map-part={part.id} className="space-y-2 p-4 text-sm">
        <Link to="/projects/$projectId/workspace" params={{projectId}} search={{view:'map',part:part.id}}
          className="nodrag font-semibold text-accent-line hover:underline" dir="auto">{part.content.title}</Link>
        <p className="text-xs text-muted-foreground">{part.content.kind || 'No kind'}</p>
        <p className="break-all font-mono text-xs">{part.content.folder || 'No folder'}</p>
        <p className="text-xs">Serves: {(part.content.serves ?? []).join(', ') || 'No linked items'}</p>
        <ul aria-label={`${part.content.title} layers`} className="space-y-1 text-xs">{part.content.layers?.map(layer => <li key={layer.layer}>
          <span className="font-medium">{layer.layer}</span> · <span className="break-all text-muted-foreground">
            {layer.not_used != null ? `Not used: ${layer.not_used}` : layer.place}
          </span>
        </li>)}</ul>
      </article>},
    }))
  },[parts,projectId,selected])
  const edges = useMemo<Edge[]>(() => permissions.map(({from,to}) => ({
    id:`allowed:${from.id}:${to.id}`, source:from.id, target:to.id, focusable:false,
    style:{stroke:'var(--muted-foreground)',strokeDasharray:DASH}, markerEnd:ARROW,
  })),[permissions])
  return <section aria-label="Project map canvas" data-project-map className="space-y-3">
    <div className="hidden h-[32rem] min-w-0 overflow-hidden rounded-2xl border border-border bg-background sm:block">
      <ReactFlow nodes={nodes} edges={edges} nodesDraggable={false} nodesConnectable={false}
        edgesFocusable={false} colorMode="dark" fitView fitViewOptions={{padding:0.2,maxZoom:1}} minZoom={0.15}
        style={THEME}>
        <Controls showInteractive={false} />
        <MiniMap pannable zoomable />
        {allowed && <Panel position="top-right"><Legend /></Panel>}
      </ReactFlow>
    </div>
    <p className="text-xs text-muted-foreground">Dependencies not recorded. Recorded elements will show the
      dependencies between parts. Open a part to browse its children.</p>
    {allowed && <section aria-label="Allowed dependencies" className="space-y-1 text-xs">
      <Legend />
      {permissions.length === 0 ? <p>No allowed dependencies between the shown parts.</p>
        : <ul className="space-y-1">{permissions.map(({from,to,layers}) => <li key={`${from.id}:${to.id}`} dir="auto">
          {from.content.title} may use {to.content.title}: {layers.join(', ')}
        </li>)}</ul>}
    </section>}
    <nav aria-label="Map parts list">
      <ul className="space-y-1 text-sm">{parts.map(part => <li key={part.id} className="flex flex-wrap items-baseline gap-x-2">
        <Link to="/projects/$projectId/workspace" params={{projectId}} search={{view:'map',part:part.id}}
          aria-current={part.id === selected ? 'page' : undefined}
          className="rounded px-1 text-accent-line hover:underline" dir="auto">{part.content.title}</Link>
        <span className="text-xs text-muted-foreground">{part.content.kind || 'No kind'}</span>
        <span className="break-all font-mono text-xs">{part.content.folder || 'No folder'}</span>
      </li>)}</ul>
    </nav>
  </section>
}

function Legend() {
  return <div data-map-legend className="flex items-center gap-1.5 rounded-md border border-border bg-background/90 px-2 py-1 text-[11px] text-muted-foreground">
    <svg width="22" height="6" aria-hidden>
      <line x1="0" y1="3" x2="22" y2="3" stroke="var(--muted-foreground)" strokeWidth="1.5" strokeDasharray={DASH} />
    </svg>
    Allowed dependencies: what the standards permit, not dependencies that exist
  </div>
}

/** The catalog's cross-part `may_use` between the shown parts' used layers, one entry per part pair. */
function permitted(parts: Part[], kinds: PartKind[]): Allowed[] {
  const used = (part: Part) => (part.content.layers ?? []).filter(layer => layer.not_used == null).map(layer => layer.layer)
  const out: Allowed[] = []
  for (const from of parts) {
    const kind = kinds.find(k => k.name === from.content.kind?.trim())
    for (const to of parts) {
      if (to.id === from.id) continue
      const layers = used(from).flatMap(id => (kind?.layers.find(spec => spec.id === id)?.may_use ?? [])
        .filter(ref => ref.part_kind != null && ref.part_kind === to.content.kind?.trim() && used(to).includes(ref.layer))
        .map(ref => `${id} → ${ref.part_kind}.${ref.layer}`))
      if (layers.length > 0) out.push({from,to,layers})
    }
  }
  return out
}
