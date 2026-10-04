// The project workspace's first functioning view: Vision.
import { Link, useParams, useSearch } from '@tanstack/react-router'
import { VisionEditor } from './vision-editor'
import { PartsView } from './parts-view'
import { RoadmapView } from './roadmap-view'
import { WorkspacePlans } from './workspace-plans'

export function WorkspacePage() {
  const { projectId } = useParams({ from: '/projects/$projectId/workspace' })
  const { view, part, outcome } = useSearch({ from: '/projects/$projectId/workspace' })
  return (
    <main className="flex-1 overflow-y-auto p-6">
      <div className="mx-auto max-w-6xl space-y-5">
        <h1 className="text-lg font-semibold">Workspace</h1>
        <nav className="flex gap-4 text-sm" aria-label="Workspace sections">
          <Link to="/projects/$projectId/workspace" params={{ projectId }} search={{ view: 'vision' }}>Vision</Link>
          <Link to="/projects/$projectId/workspace" params={{ projectId }} search={{ view: 'map' }}>Project map</Link>
          <Link to="/projects/$projectId/workspace" params={{ projectId }} search={{ view: 'roadmap' }}>Roadmap</Link>
          <Link to="/projects/$projectId/workspace" params={{ projectId }} search={{ view: 'plans' }}>Plans</Link>
        </nav>
        {view === 'map' ? <PartsView key={projectId} projectId={projectId} selected={part} /> : view === 'roadmap' ? <RoadmapView key={projectId} projectId={projectId} selected={outcome} /> : view === 'plans' ? <WorkspacePlans key={projectId} projectId={projectId} /> : <VisionEditor key={projectId} projectId={projectId} />}
      </div>
    </main>
  )
}
