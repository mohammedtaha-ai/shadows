// One job: the project settings page (§13.11) — the Planner's instructions
// for this project, and the external agents allowed on it.

import { useQuery } from '@tanstack/react-query'
import { getRouteApi } from '@tanstack/react-router'
import { projectsQuery } from '@/api/queries'
import { ExternalAgents } from './external-agents'
import { InstructionsEditor } from './instructions-editor'

const route = getRouteApi('/projects/$projectId/settings')

export function ProjectSettings() {
  const { projectId } = route.useParams()
  const project = useQuery(projectsQuery).data?.find((p) => p.id === projectId)

  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto max-w-2xl space-y-8 px-6 py-8">
        <header className="space-y-1">
          <h1 className="text-base font-medium">Project settings</h1>
          {project !== undefined && (
            <p className="text-xs text-faint-foreground">
              {project.name}
              {project.directory != null && (
                <span className="ml-2 font-mono">{project.directory}</span>
              )}
            </p>
          )}
        </header>
        {/* The router keeps this page mounted when only the project changes, so
            each section is keyed by it: one project's unsaved draft, pending
            command id or one-time Connect command never shows under another. */}
        <InstructionsEditor key={`instructions-${projectId}`} projectId={projectId} />
        <ExternalAgents key={`agents-${projectId}`} projectId={projectId} />
      </div>
    </div>
  )
}
