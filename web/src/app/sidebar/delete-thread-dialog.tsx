// One job: confirming a conversation's removal as an idempotent command (§16.8).

import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { useLayoutEffect, useRef } from 'react'
import { type PlanningThread, removeThread } from '@/api/client'
import { type Attempt, attemptFor } from '@/api/command-id'
import { threadQuery, threadsQuery, workflowsKey } from '@/api/queries'
import { Button } from '@/components/ui/button'
import {
  Dialog, DialogClose, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle,
} from '@/components/ui/dialog'
import { ErrorLine } from '../error-line'

export function DeleteThreadDialog({ thread, open, onOpenChange, isCurrent }: {
  thread: PlanningThread
  open: boolean
  onOpenChange: (open: boolean) => void
  isCurrent: boolean
}) {
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  // A lost response retried after reopening still carries the same command.
  const pending = useRef<Attempt | null>(null)
  const current = useRef(isCurrent)
  useLayoutEffect(() => { current.current = isCurrent }, [isCurrent])
  const remove = useMutation({
    mutationFn: (commandId: string) => removeThread(thread.id, commandId),
    onSuccess: async (removed) => {
      pending.current = null
      onOpenChange(false)
      if (current.current) {
        await navigate({ to: '/projects/$projectId/new', params: { projectId: thread.project_id } })
      }
      queryClient.setQueryData(threadQuery(thread.id).queryKey, removed)
      queryClient.setQueryData<PlanningThread[]>(threadsQuery(thread.project_id).queryKey,
        (threads) => threads?.filter((t) => t.id !== thread.id))
      void queryClient.invalidateQueries({ queryKey: threadsQuery(thread.project_id).queryKey })
      void queryClient.invalidateQueries({ queryKey: workflowsKey })
    },
  })
  const changeOpen = (value: boolean) => {
    if (remove.isPending) return
    if (!value) remove.reset()
    onOpenChange(value)
  }
  const confirm = () => {
    pending.current = attemptFor(pending.current, { remove: thread.id })
    remove.mutate(pending.current.commandId)
  }
  return (
    <Dialog open={open} onOpenChange={changeOpen}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Delete "<span dir="auto">{thread.title}</span>"?</DialogTitle>
          <DialogDescription>
            It leaves the list for good. Its plans stay in the project, and their history still names it.
          </DialogDescription>
        </DialogHeader>
        {remove.error !== null && <ErrorLine error={remove.error} />}
        <DialogFooter>
          <DialogClose render={<Button type="button" variant="outline" disabled={remove.isPending} />}>Cancel</DialogClose>
          <Button variant="destructive" onClick={confirm} disabled={remove.isPending}>
            {remove.isPending ? 'Deleting…' : 'Delete'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
