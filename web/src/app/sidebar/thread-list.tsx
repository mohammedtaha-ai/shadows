// One job: an open project's conversations in the sidebar.

import { useQuery } from '@tanstack/react-query'
import { Link } from '@tanstack/react-router'
import { Ellipsis, MessageSquare, Trash2 } from 'lucide-react'
import { useState } from 'react'
import type { PlanningThread } from '@/api/client'
import { threadsQuery } from '@/api/queries'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { DeleteThreadDialog } from './delete-thread-dialog'

export function ThreadList({ projectId, selected }: { projectId: string; selected?: string }) {
  const { data: threads } = useQuery(threadsQuery(projectId))
  const [deleting, setDeleting] = useState<PlanningThread | null>(null)
  const [open, setOpen] = useState(false)

  return (
    <>
      <ul className="mt-0.5 mb-1.5 ml-4 space-y-0.5 border-l border-sidebar-border pl-2">
        {threads?.map((thread) => {
          const isSelected = thread.id === selected
          return (
            <li key={thread.id} className="flex items-center gap-0.5">
              <Link
                to="/projects/$projectId/threads/$threadId"
                params={{ projectId, threadId: thread.id }}
                aria-current={isSelected ? 'page' : undefined}
                className={`flex min-w-0 flex-1 items-center gap-2 rounded-r-md border-l-2 px-2 py-1 text-sm transition-colors ${
                  isSelected
                    ? 'border-accent-line bg-secondary text-secondary-foreground'
                    : 'border-transparent text-muted-foreground hover:bg-sidebar-accent hover:text-sidebar-foreground'
                }`}
              >
                <MessageSquare className="size-3.5 shrink-0" />
                <span dir="auto" className="min-w-0 flex-1 truncate text-start">{thread.title}</span>
              </Link>
              <DropdownMenu>
                <DropdownMenuTrigger render={
                  <Button variant="ghost" size="icon" className="size-6 shrink-0"
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
