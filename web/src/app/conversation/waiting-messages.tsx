// One job: a conversation's waiting messages (§20), each with Send now (Send
// when no turn runs) and Remove, and the reason a send failed.

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useRef } from 'react'
import { attachmentUrl, sendQueuedNow, unqueueMessage } from '@/api/threads'
import { ApiError } from '@/api/error'
import { queuedQuery } from '@/api/threads'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'

export function WaitingMessages({ threadId, running }: { threadId: string; running: boolean }) {
  const queue = useQuery(queuedQuery(threadId))
  const queryClient = useQueryClient()
  const refetch = () =>
    queryClient.invalidateQueries({ queryKey: queuedQuery(threadId).queryKey })
  const sendCommands = useRef(new Map<string, string>())
  const sendNow = useMutation({
    mutationFn: (id: string) => {
      const command = sendCommands.current.get(id) ?? crypto.randomUUID()
      sendCommands.current.set(id, command)
      return sendQueuedNow(threadId, id, command)
    },
    onSuccess: (_answer, id) => { sendCommands.current.delete(id) },
    onError: (error, id) => {
      if (error instanceof ApiError && error.problem.kind === 'daemon' && error.problem.code === 'COMMAND_CONFLICT') sendCommands.current.delete(id)
    },
    onSettled: refetch,
  })
  const remove = useMutation({
    mutationFn: (id: string) => unqueueMessage(threadId, id, crypto.randomUUID()),
    onSettled: refetch,
  })
  const waiting = queue.data ?? []
  if (waiting.length === 0) return null
  const error = sendNow.error ?? remove.error
  return (
    <div className="mx-auto w-full max-w-3xl space-y-2 px-6 pb-2">
      <ul className="space-y-2" aria-label="Waiting messages">
        {waiting.map((m) => (
          <li
            key={m.id}
            className="ml-auto max-w-[80%] rounded-xl border border-border/60 bg-muted/40 px-3 py-2 opacity-70"
          >
            {(m.attachments ?? []).map(id => <img key={id} src={attachmentUrl(threadId, id)} alt="Queued image" className="size-24 rounded object-cover" />)}
            <p dir="auto" className="whitespace-pre-wrap text-sm text-foreground">
              {m.prompt}
            </p>
            <div className="mt-1 flex items-center gap-2 text-xs text-muted-foreground">
              <span>Waiting</span>
              {m.last_error != null && (
                <span className="text-destructive-foreground">{m.last_error}</span>
              )}
              <Button
                size="sm"
                variant="ghost"
                onClick={() => sendNow.mutate(m.id)}
                disabled={sendNow.isPending}
              >
                {running ? 'Send now' : 'Send'}
              </Button>
              <Button
                size="sm"
                variant="ghost"
                onClick={() => remove.mutate(m.id)}
                disabled={remove.isPending}
              >
                Remove
              </Button>
            </div>
          </li>
        ))}
      </ul>
      {error !== null && <ErrorLine error={error} />}
    </div>
  )
}
