// The project workspace's first functioning view: Vision.
import { Link, useParams, useSearch } from '@tanstack/react-router'
import { Compass } from 'lucide-react'
import { Panel } from '@/components/ui/panel'
import { VisionEditor } from './vision-editor'
import { PartsView } from './parts-view'
import { RoadmapView } from './roadmap-view'
import { WorkspacePlans } from './workspace-plans'
import { StandardsView } from './standards-view'

// The open section reads as the selected pill; an unknown or missing view is Vision.
const tab = (active: boolean) =>
  `rounded-lg px-3 py-1.5 transition-all ${active ? 'pill-active' : 'text-muted-foreground hover:bg-accent-line/10 hover:text-foreground'}`

export function WorkspacePage() {
  const { projectId } = useParams({ from: '/projects/$projectId/workspace' })
  const { view, part, outcome } = useSearch({ from: '/projects/$projectId/workspace' })
  const activeView = view ?? 'vision'
  return (
    <main className="flex-1 overflow-y-auto bg-background p-6">
      <div className="mx-auto max-w-6xl space-y-6">
        <Panel className="flex flex-wrap items-center justify-between gap-4 px-5 py-4">
          <div className="flex items-center gap-3">
            <div className="flex size-9 items-center justify-center rounded-xl border border-accent-line/35 bg-accent-line/15">
              <Compass aria-hidden className="size-4 text-accent-line" />
            </div>
            <h1 className="text-lg font-semibold tracking-tight text-foreground">Workspace</h1>
          </div>
          <nav className="flex flex-wrap items-center gap-1.5 rounded-xl border border-border bg-input-background p-1 text-xs font-medium" aria-label="Workspace sections">
            <Link to="/projects/$projectId/agreements" params={{ projectId }} className={tab(false)}>Contracts</Link>
            <Link to="/projects/$projectId/workspace" params={{ projectId }} search={{ view: 'vision' }} className={tab(activeView === 'vision')}>Vision</Link>
            <Link to="/projects/$projectId/workspace" params={{ projectId }} search={{ view: 'map' }} className={tab(activeView === 'map')}>Project map</Link>
            <Link to="/projects/$projectId/workspace" params={{ projectId }} search={{ view: 'standards' }} className={tab(activeView === 'standards')}>Standards</Link>
            <Link to="/projects/$projectId/workspace" params={{ projectId }} search={{ view: 'roadmap' }} className={tab(activeView === 'roadmap')}>Roadmap</Link>
            <Link to="/projects/$projectId/workspace" params={{ projectId }} search={{ view: 'plans' }} className={tab(activeView === 'plans')}>Plans</Link>
          </nav>
        </Panel>
        {view === 'standards' ? <StandardsView key={projectId} projectId={projectId} /> : view === 'map' ? <PartsView key={projectId} projectId={projectId} selected={part} /> : view === 'roadmap' ? <RoadmapView key={projectId} projectId={projectId} selected={outcome} /> : view === 'plans' ? <WorkspacePlans key={projectId} projectId={projectId} /> : <VisionEditor key={projectId} projectId={projectId} />}
      </div>
    </main>
  )
}
