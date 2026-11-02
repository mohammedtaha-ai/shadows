// One job: an open project's conversations in the sidebar.

import { useQuery } from '@tanstack/react-query'
import { Link } from '@tanstack/react-router'
import { Ellipsis, MessageSquare, Trash2 } from 'lucide-react'
import { useState } from 'react'
import type { PlanningThread } from '@/api/threads'
import { threadsQuery } from '@/api/threads'
import { agentThreadsQuery } from '@/api/aylul-profiles'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { DeleteThreadDialog } from './delete-thread-dialog'

export function ThreadList({ projectId, selected, profile }: { projectId: string; selected?: string; profile?: string }) {
  const { data: threads } = useQuery(profile === undefined ? threadsQuery(projectId) : agentThreadsQuery(profile))
  const [deleting, setDeleting] = useState<PlanningThread | null>(null)
  const [open, setOpen] = useState(false)

  return (
    <>
      <ul className="mt-1 mb-1.5 ml-4 space-y-0.5 border-l border-sidebar-border/80 pl-2">
        {threads?.map((thread) => {
          const isSelected = thread.id === selected
          return (
            <li key={thread.id} className="group/thread flex items-center gap-0.5">
              <Link
                to="/projects/$projectId/threads/$threadId"
                params={{ projectId, threadId: thread.id }}
                aria-current={isSelected ? 'page' : undefined}
                className={`flex min-w-0 flex-1 items-center gap-2 rounded-r-lg border-l-2 px-2.5 py-1.5 text-xs transition-all ${
                  isSelected
                    ? 'border-accent-line bg-gradient-to-r from-accent/35 to-secondary/70 font-medium text-secondary-foreground'
                    : 'border-transparent text-muted-foreground hover:bg-sidebar-accent/70 hover:text-sidebar-foreground'
                }`}
              >
                <MessageSquare className={`size-3.5 shrink-0 ${isSelected ? 'text-accent-line' : ''}`} />
                <span dir="auto" className="min-w-0 flex-1 truncate text-start">{thread.title}</span>
              </Link>
              <DropdownMenu>
                <DropdownMenuTrigger render={
                  <Button variant="ghost" size="icon" className="size-6 shrink-0 opacity-70 transition-opacity group-hover/thread:opacity-100"
                    aria-label={`Conversation options: ${thread.title}`} />
                }>
                  <Ellipsis className="size-3.5" />
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end">
                  <DropdownMenuItem onClick={() => { setDeleting(thread); setOpen(true) }}>
                    <Trash2 /> Delete
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </li>
          )
        })}
      </ul>
      {deleting !== null && (
        <DeleteThreadDialog key={deleting.id} thread={deleting} open={open}
          onOpenChange={setOpen} isCurrent={selected === deleting.id} />
      )}
    </>
  )
}
