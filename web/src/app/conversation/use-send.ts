// Own delivery attempts so retries keep the command for the same message.

import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useCallback, useRef } from 'react'
import type { TurnSettings } from '@/api/sessions'
import { type Focus, queueMessage, startTurn, queuedQuery } from '@/api/threads'
import { type Attempt, attemptFor } from '@/api/command-id'
import { ApiError } from '@/api/error'
import { noteCompanionSend } from '@/desktop'
import { tabId } from '@/stream'
import { uploadImages, type PendingImage } from './carried-send'
import type { SendTo } from './composer'
import type { PointedTask } from './focus-chip'
import type { Turn } from './turn-state'

interface Send {
  images: PendingImage[]
  commandId: string
  text: string
  settings: TurnSettings
  pointed: PointedTask | null
  planId: string | undefined
}

export function useSend({
  to, threadId, running, initialAttempt = null, imagesChanged, dropImages,
  clearPrompt, clearContinuedPlan, onPointed, onStarted,
}: {
  to: SendTo
  threadId: string | null
  running: Turn | null
  initialAttempt?: Attempt | null
  imagesChanged: () => void
  dropImages: (sent: PendingImage[]) => void
  clearPrompt: () => void
  clearContinuedPlan: () => void
  onPointed: (done: PointedTask) => void
  onStarted: (operationId: string) => void
}) {
  // The command id belongs to a pending send: made when Send is
  // pressed with no pending send, reused by a retry of that same send, and
  // dropped when it succeeds or when the prompt or a setting changes, so a
  // changed request is a new command rather than a COMMAND_CONFLICT.
  const pending = useRef<Attempt | null>(initialAttempt)

  const clearAttempt = useCallback(() => { pending.current = null }, [])

  const send = useMutation({
    mutationFn: async ({ commandId, text, settings, pointed, planId, images }: Send) =>{
      if('draft' in to)return to.draft(commandId,text,settings,images,imagesChanged)
      const attachments = await uploadImages(to.threadId, images, imagesChanged)
      await noteCompanionSend(to.threadId)
      return startTurn(to.threadId,commandId,text,settings,{
        focus:focusOf(pointed),plan:planId??null,clientTab:tabId(),attachments,
      })
    },
    onError: error => {
      if (error instanceof ApiError && error.problem.kind === 'daemon' && error.problem.code === 'COMMAND_CONFLICT') pending.current = null
    },
    onSuccess: (operationId, { pointed, images: sent }) => {
      dropImages(sent)
      pending.current = null
      clearContinuedPlan()
      clearPrompt()
      if (pointed !== null) onPointed(pointed)
      onStarted(operationId)
    },
  })

  // While a turn runs Enter queues the message with the settings the menus
  // hold; it is sent when the turn ends or by Send now.
  const queryClient = useQueryClient()
  const queue = useMutation({
    mutationFn: async ({commandId,text,settings,pointed,planId,images}:Send)=>{
      const attachments = await uploadImages(threadId as string, images, imagesChanged)
      await noteCompanionSend(threadId as string)
      return queueMessage(threadId as string,commandId,text,settings,{
        focus:focusOf(pointed),plan:planId??null,clientTab:tabId(),attachments,
      })
    },
    // Keep the complete message until Core accepts it, so a refusal cannot
    // lose its text while the images remain selected.
    onSuccess: (_answer, { images: sent }) => {
      dropImages(sent)
      clearPrompt()
      pending.current = null
      void queryClient.invalidateQueries({ queryKey: queuedQuery(threadId as string).queryKey })
    },
    onError: (error) => {
      if (error instanceof ApiError && error.problem.kind === 'daemon' && error.problem.code === 'COMMAND_CONFLICT') pending.current = null
    },
  })

  const deliver = ({ text, settings, pointed, planId, images }: Omit<Send, 'commandId'>) => {
    // The focus is part of the command: pointing elsewhere is a new one.
    pending.current = attemptFor(pending.current, { text, settings, ...(images.length === 0 ? {} : { attachments: images.map(image => image.key) }), focus: focusOf(pointed), ...(planId === undefined ? {} : { plan: planId }) })
    const args = { commandId: pending.current.commandId, text, settings, pointed, planId, images: [...images] }
    if (running !== null && threadId !== null) {
      queue.mutate(args)
    } else send.mutate(args)
  }

  return {
    deliver,
    clearAttempt,
    sending: send.isPending,
    inFlight: send.isPending || queue.isPending,
    error: send.error ?? queue.error,
  }
}

/** What the turn sends: the task's id, never its number. */
function focusOf(pointed: PointedTask | null): Focus | null {
  if (pointed === null) return null
  return { workflow_id: pointed.workflowId, task_id: pointed.task.id, revision: pointed.revision }
}
