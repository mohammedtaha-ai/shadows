// Approval attribution with a link to the request's own conversation.
import { Link } from '@tanstack/react-router'
import type { Plan } from '@/api/client'

export function ApprovedBy({ by, projectId }: {
  by: Plan['approved_by'] | null | undefined
  projectId: string
}) {
  if (by == null) return null
  if (by.agent === 'human') return <span>Approved by you</span>
  return <span>Approved by the Planner at your request
    {by.request && <> · <Link to="/projects/$projectId/threads/$threadId"
      params={{ projectId, threadId: by.request.thread_id }}
      hash={`entry-${by.request.ordinal}`}>your message</Link></>}
  </span>
}
