// One job: the main pane when a project is open and no conversation is.

import { useQuery } from '@tanstack/react-query'
import { getRouteApi } from '@tanstack/react-router'
import { MessageSquarePlus } from 'lucide-react'
import { projectsQuery, threadsQuery } from '@/api/queries'
import { Button } from '@/components/ui/button'
import { ErrorLine } from './error-line'
import { ProjectModes } from './project-modes'
import { useNewConversation } from './use-new-conversation'

const route = getRouteApi('/projects/$projectId')

export function ProjectPage() {
  const { projectId } = route.useParams()
  const project = useQuery(projectsQuery).data?.find((p) => p.id === projectId)
  const threads = useQuery(threadsQuery(projectId)).data
  const newConversation = useNewConversation(projectId)

  const empty = threads !== undefined && threads.length === 0

  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-4 p-8 text-center">
      <MessageSquarePlus className="size-8 text-accent-line" />
      <div className="space-y-1">
        <h1 className="text-base font-medium">
          {empty ? 'Start a conversation' : 'Choose a conversation'}
        </h1>
        {project?.directory != null && (
          <p className="font-mono text-xs text-faint-foreground">{project.directory}</p>
        )}
      </div>
      <Button onClick={newConversation.start} disabled={newConversation.pending || threads === undefined}>
        {empty ? 'Start a conversation' : 'New conversation'}
      </Button>
      {newConversation.error !== null && <ErrorLine error={newConversation.error} />}
      {project !== undefined && (
        <div className="mt-6 border-t border-border pt-6">
          <ProjectModes project={project} />
        </div>
      )}
    </div>
  )
}
