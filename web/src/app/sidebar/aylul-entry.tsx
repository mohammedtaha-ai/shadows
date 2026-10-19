// One job: the sidebar's AYLUL entry, above Projects — it opens the assistant
// project's latest conversation, or its new-conversation page when it has none.

import { useQuery } from '@tanstack/react-query'
import { Link, useParams } from '@tanstack/react-router'
import { Sparkles } from 'lucide-react'
import type { PlanningThread } from '@/api/client'
import { projectsQuery, threadsQuery } from '@/api/queries'

export function AylulEntry() {
  const project = useQuery(projectsQuery).data?.find((p) => p.assistant)
  const { projectId } = useParams({ strict: false })
  const threads = useQuery({ ...threadsQuery(project?.id ?? ''), enabled: project !== undefined })
  if (project === undefined) return null

  const latest = (threads.data ?? [])
    .filter((t) => t.removed_at === null)
    .reduce<PlanningThread | null>(
      (best, t) => (best === null || t.created_at > best.created_at ? t : best),
      null,
    )
  const current = projectId === project.id
  const className = `flex items-center gap-2 rounded-md px-2 py-1.5 text-sm transition-colors ${
    current
      ? 'bg-secondary text-secondary-foreground'
      : 'text-muted-foreground hover:bg-sidebar-accent hover:text-sidebar-foreground'
  }`
  const icon = <Sparkles aria-hidden className="size-4 shrink-0 text-accent-line" />

  return (
    <div className="mb-2">
      {latest === null ? (
        <Link
          to="/projects/$projectId/new"
          params={{ projectId: project.id }}
          aria-current={current ? 'page' : undefined}
          className={className}
        >
          {icon}
          AYLUL
        </Link>
      ) : (
        <Link
          to="/projects/$projectId/threads/$threadId"
          params={{ projectId: project.id, threadId: latest.id }}
          aria-current={current ? 'page' : undefined}
          className={className}
        >
          {icon}
          AYLUL
        </Link>
      )}
    </div>
  )
}
