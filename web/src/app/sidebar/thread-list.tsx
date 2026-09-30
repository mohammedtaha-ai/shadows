// One job: an open project's conversations in the sidebar.

import { useQuery } from '@tanstack/react-query'
import { Link } from '@tanstack/react-router'
import { MessageSquare } from 'lucide-react'
import { threadsQuery } from '@/api/queries'

export function ThreadList({ projectId, selected }: { projectId: string; selected?: string }) {
  const { data: threads } = useQuery(threadsQuery(projectId))

  return (
    <ul className="mt-0.5 mb-1.5 ml-4 space-y-0.5 border-l border-sidebar-border pl-2">
      {threads?.map((thread) => {
        const isSelected = thread.id === selected
        return (
          <li key={thread.id}>
            <Link
              to="/projects/$projectId/threads/$threadId"
              params={{ projectId, threadId: thread.id }}
              aria-current={isSelected ? 'page' : undefined}
              className={`flex items-center gap-2 rounded-r-md border-l-2 px-2 py-1 text-sm transition-colors ${
                isSelected
                  ? 'border-accent-line bg-secondary text-secondary-foreground'
                  : 'border-transparent text-muted-foreground hover:bg-sidebar-accent hover:text-sidebar-foreground'
              }`}
            >
              <MessageSquare className="size-3.5 shrink-0" />
              <span dir="auto" className="min-w-0 flex-1 truncate text-start">
                {thread.title}
              </span>
            </Link>
          </li>
        )
      })}
    </ul>
  )
}
