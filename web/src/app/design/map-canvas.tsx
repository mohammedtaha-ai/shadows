import '@xyflow/react/dist/style.css'
import { Controls, ReactFlow, type Node } from '@xyflow/react'
import { Link } from '@tanstack/react-router'
import { type CSSProperties, useMemo } from 'react'
import type { Part } from '@/api/design'

const theme = {
  '--xy-background-color':'var(--background)',
  '--xy-node-background-color':'var(--card)',
  '--xy-node-color':'var(--foreground)',
  '--xy-node-border':'1px solid var(--border)',
  '--xy-controls-button-background-color':'var(--card)',
  '--xy-controls-button-color':'var(--foreground)',
  '--xy-controls-button-border-color':'var(--border)',
  '--xy-attribution-background-color':'transparent',
} as CSSProperties

/** The visible ownership scope. Data and pagination belong to its caller. */
export function MapCanvas({parts,projectId,selected}: {parts:Part[];projectId:string;selected?:string}) {
  const nodes = useMemo<Node[]>(() => {
    const shown = [...parts].sort((a,b) => Number(a.content.kind === 'database') - Number(b.content.kind === 'database'))
    return shown.map((part,index) => ({id:part.id,
      position:{x:(index % 2)*310,y:Math.floor(index/2)*225},
      selected:part.id === selected,
      style:{width:280,borderRadius:16,padding:0,textAlign:'left'},
      data:{label:<article data-map-part={part.id} className="space-y-2 p-4 text-sm">
        <Link to="/projects/$projectId/workspace" params={{projectId}} search={{view:'map',part:part.id}}
          className="nodrag font-semibold text-accent-line hover:underline" dir="auto">{part.content.title}</Link>
        <p className="text-xs text-muted-foreground">{part.content.kind || 'Part'}</p>
        <p className="break-all font-mono text-xs">{part.content.folder}</p>
        <p className="text-xs">Serves: {(part.content.serves ?? []).join(', ') || 'No linked items'}</p>
        <ul className="space-y-1 text-xs">{part.content.layers?.map(layer => <li key={layer.layer}>
          <span className="font-medium">{layer.layer}</span> · <span className="break-all text-muted-foreground">
            {layer.not_used != null ? `Not used: ${layer.not_used}` : layer.place}
          </span>
        </li>)}</ul>
      </article>},
    }))
  },[parts,projectId,selected])
  return <section aria-label="Project map canvas" data-project-map className="space-y-3">
    <div className="h-[32rem] min-w-0 overflow-hidden rounded-2xl border border-border bg-background">
      <ReactFlow nodes={nodes} edges={[]} nodesDraggable={false} nodesConnectable={false}
        edgesFocusable={false} fitView fitViewOptions={{padding:0.2,maxZoom:1}} minZoom={0.15}
        colorMode="system" style={theme}>
        <Controls showInteractive={false} />
      </ReactFlow>
    </div>
    <p className="text-xs text-muted-foreground">Dependencies appear when elements are recorded. Open a part to browse its children.</p>
    <nav aria-label="Map keyboard navigation" className="flex flex-wrap gap-3 text-sm">
      {parts.map(part => <Link key={part.id} to="/projects/$projectId/workspace" params={{projectId}}
        search={{view:'map',part:part.id}} aria-current={part.id === selected ? 'page' : undefined}
        className="rounded border border-border px-3 py-2 text-accent-line hover:bg-muted" dir="auto">{part.content.title}</Link>)}
    </nav>
  </section>
}
