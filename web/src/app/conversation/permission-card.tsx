import { useQuery } from '@tanstack/react-query'
import { waitingPermissionsQuery } from '@/api/permissions'
import type { ThreadEntry } from '@/api/threads'
import { useProjectEvents } from '@/stream'
import { ErrorLine } from '../error-line'
import { PermissionAnswer } from './permission-answer'

export function PermissionCard({ entry, projectId }: { entry: ThreadEntry; projectId: string }) {
  const requestId = typeof entry.card?.request_id === 'string' ? entry.card.request_id : null
  useProjectEvents(projectId)
  const waiting = useQuery({ ...waitingPermissionsQuery, enabled: requestId !== null })
  const request = waiting.error === null ? waiting.data?.find(value => value.thread_id === entry.thread_id && value.request_id === requestId) : undefined
  return (
    <section aria-label="Permission request" className="surface-glow space-y-3 rounded-lg p-4">
      <PermissionAnswer key={requestId} request={request}>
        <p dir="auto" className="text-sm font-medium whitespace-pre-wrap">{entry.body}</p>
        {waiting.isPending && requestId !== null && (
          <p role="status" className="text-xs text-muted-foreground">Checking request…</p>
        )}
        {!request && waiting.isSuccess && (
          <p className="text-xs text-muted-foreground">This request is no longer waiting.</p>
        )}
        {waiting.error && <ErrorLine error={waiting.error} />}
      </PermissionAnswer>
    </section>
  )
}
