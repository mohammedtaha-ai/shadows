// The project workspace's first functioning view: Vision.
import { useParams } from '@tanstack/react-router'
import { VisionEditor } from './vision-editor'

export function WorkspacePage() {
  const { projectId } = useParams({ from: '/projects/$projectId/workspace' })
  return (
    <main className="flex-1 overflow-y-auto p-6">
      <div className="mx-auto max-w-3xl space-y-5">
        <h1 className="text-lg font-semibold">Workspace</h1>
        <h2 className="text-sm font-medium">Vision</h2>
        <VisionEditor key={projectId} projectId={projectId} />
      </div>
    </main>
  )
}
