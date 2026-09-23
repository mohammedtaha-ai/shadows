// One job: the main pane when no project is open.

import { useQuery } from '@tanstack/react-query'
import { FolderPlus } from 'lucide-react'
import { projectsQuery } from '@/api/queries'
import { Button } from '@/components/ui/button'
import { DaemonProblem } from './daemon-status'
import { useOpenNewProject } from './new-project/open-new-project'

export function Home() {
  const { status, data, error, refetch, isFetching } = useQuery(projectsQuery)
  const openNewProject = useOpenNewProject()

  if (status === 'error') {
    return (
      <div className="flex flex-1 items-center justify-center p-8">
        <DaemonProblem error={error} retry={() => void refetch()} retrying={isFetching} />
      </div>
    )
  }

  if (status === 'success' && data.length === 0) {
    return (
      <div className="flex flex-1 flex-col items-center justify-center gap-4 p-8 text-center">
        <FolderPlus className="size-8 text-accent-line" />
        <div className="space-y-1">
          <h1 className="text-base font-medium">Create your first project</h1>
          <p className="max-w-sm text-sm text-muted-foreground">
            A project is a folder on this machine. The Planner reads it and runs there.
          </p>
        </div>
        <Button onClick={openNewProject}>New project</Button>
      </div>
    )
  }

  return (
    <div className="flex flex-1 items-center justify-center p-8">
      <p className="text-sm text-faint-foreground">
        {status === 'pending' ? 'Looking for the daemon…' : 'Choose a project.'}
      </p>
    </div>
  )
}
