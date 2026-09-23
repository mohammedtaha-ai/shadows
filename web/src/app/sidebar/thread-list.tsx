// One job: the open project's conversations in the sidebar, and the row that
// starts a new one.

import { useQuery } from '@tanstack/react-query'
import { Link } from '@tanstack/react-router'
import { MessageSquare, Plus } from 'lucide-react'
import { threadsQuery } from '@/api/queries'
import { ErrorLine } from '../error-line'
import { useNewConversation } from '../use-new-conversation'

export function ThreadList({ projectId, selected }: { projectId: string; selected?: string }) {
  const { data: threads } = useQuery(threadsQuery(projectId))
  const newConversation = useNewConversation(projectId)

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
              <span className="truncate">{thread.title}</span>
            </Link>
          </li>
        )
      })}
      <li>
        <button
          type="button"
          onClick={newConversation.start}
          disabled={newConversation.pending}
          className="flex w-full items-center gap-2 rounded-md px-2 py-1 text-left text-sm text-faint-foreground transition-colors hover:bg-sidebar-accent hover:text-sidebar-foreground disabled:opacity-50"
        >
          <Plus className="size-3.5" />
          New conversation
        </button>
        {newConversation.error !== null && (
          <div className="px-2 py-1">
            <ErrorLine error={newConversation.error} />
          </div>
        )}
      </li>
    </ul>
  )
}
