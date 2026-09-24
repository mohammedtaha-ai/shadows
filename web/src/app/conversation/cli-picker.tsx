// One job: the conversation header's choice of CLI (spec §12.6) —
// changeable until the conversation's first turn, shown with a lock after.

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ChevronDown, Lock } from 'lucide-react'
import { useRef } from 'react'
import { type PlanningThread, setThreadHarness } from '@/api/client'
import { type Attempt, attemptFor } from '@/api/command-id'
import { harnessesQuery, threadsQuery } from '@/api/queries'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { ErrorLine } from '../error-line'
import { reopenSession } from './use-session'

export function CliPicker({
  projectId,
  threadId,
  harness,
  label,
  locked,
}: {
  projectId: string
  threadId: string
  /** The thread's harness kind, and how it reads. */
  harness: string
  label: string
  /** The thread has run a turn, or is a fork: its harness is fixed
   * (`HARNESS_LOCKED`). */
  locked: boolean
}) {
  const queryClient = useQueryClient()
  const harnesses = useQuery(harnessesQuery).data
  const pending = useRef<Attempt | null>(null)

  const change = useMutation({
    mutationFn: ({ commandId, kind }: { commandId: string; kind: string }) =>
      setThreadHarness(threadId, commandId, kind),
    onSuccess: (thread) => {
      pending.current = null
      queryClient.setQueryData<PlanningThread[]>(threadsQuery(projectId).queryKey, (threads) =>
        threads?.map((t) => (t.id === thread.id ? thread : t)),
      )
      reopenSession(queryClient, threadId)
    },
  })

  if (locked) {
    return (
      <Tooltip>
        <TooltipTrigger
          render={
            <span
              aria-label="CLI locked for this conversation"
              className="flex items-center gap-1 px-1.5 text-xs text-muted-foreground"
            />
          }
        >
          <Lock aria-hidden className="size-3" />
          {label}
        </TooltipTrigger>
        <TooltipContent>
          The CLI is fixed once a conversation has run a turn; a fork keeps its source’s.
        </TooltipContent>
      </Tooltip>
    )
  }

  const pick = (kind: string) => {
    if (kind === harness || change.isPending) return
    pending.current = attemptFor(pending.current, { harness: kind })
    change.mutate({ commandId: pending.current.commandId, kind })
  }

  return (
    <span className="flex items-center gap-2">
      <DropdownMenu>
        <DropdownMenuTrigger
          render={<Button variant="ghost" size="xs" title="CLI" disabled={change.isPending} />}
        >
          {label}
          <ChevronDown aria-hidden />
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end">
          <DropdownMenuRadioGroup value={harness} onValueChange={(v: string) => pick(v)}>
            {(harnesses ?? []).map((h) => (
              <DropdownMenuRadioItem key={h.kind} value={h.kind} disabled={!h.available}>
                <span>{h.label}</span>
                {!h.available && <span className="text-xs text-muted-foreground">coming</span>}
              </DropdownMenuRadioItem>
            ))}
          </DropdownMenuRadioGroup>
        </DropdownMenuContent>
      </DropdownMenu>
      {change.error !== null && <ErrorLine error={change.error} />}
    </span>
  )
}
