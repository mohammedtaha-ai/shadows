// The project workspace's first functioning view: Vision.
import { Link, useParams, useSearch } from '@tanstack/react-router'
import { VisionEditor } from './vision-editor'
import { PartsView } from './parts-view'

export function WorkspacePage() {
  const { projectId } = useParams({ from: '/projects/$projectId/workspace' })
  const { view, part } = useSearch({ from: '/projects/$projectId/workspace' })
  return (
    <main className="flex-1 overflow-y-auto p-6">
      <div className="mx-auto max-w-6xl space-y-5">
        <h1 className="text-lg font-semibold">Workspace</h1>
        <nav className="flex gap-4 text-sm" aria-label="Workspace sections">
          <Link to="/projects/$projectId/workspace" params={{ projectId }} search={{ view: 'vision' }}>Vision</Link>
          <Link to="/projects/$projectId/workspace" params={{ projectId }} search={{ view: 'map' }}>Project map</Link>
        </nav>
        {view === 'map' ? <PartsView key={projectId} projectId={projectId} selected={part} /> : <VisionEditor key={projectId} projectId={projectId} />}
      </div>
    </main>
  )
}
