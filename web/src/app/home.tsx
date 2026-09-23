// One job: the main pane when no thread is open.

import { useQuery } from '@tanstack/react-query'
import { projectsQuery } from '@/api/queries'
import { DaemonProblem } from './daemon-status'

export function Home() {
  const { status, error, refetch, isFetching } = useQuery(projectsQuery)

  if (status === 'error') {
    return (
      <div className="flex flex-1 items-center justify-center p-8">
        <DaemonProblem error={error} retry={() => void refetch()} retrying={isFetching} />
      </div>
    )
  }

  return (
    <div className="flex flex-1 items-center justify-center p-8">
      <p className="text-sm text-faint-foreground">
        {status === 'pending' ? 'Looking for the daemon…' : 'No thread open.'}
      </p>
    </div>
  )
}
