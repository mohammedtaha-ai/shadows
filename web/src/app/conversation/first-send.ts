// Orchestrate the draft's first send.

import { useQueryClient } from '@tanstack/react-query'
import { useNavigate } from '@tanstack/react-router'
import { useEffect, useRef } from 'react'
import { type TurnSettings, changeModel, openSession } from '@/api/sessions'
import { createThread, startTurn, threadsQuery } from '@/api/threads'
import { type Attempt, attemptFor } from '@/api/command-id'
import { ApiError } from '@/api/error'
import { agentThreadsQuery } from '@/api/aylul-profiles'
import {noteCompanionSend} from '@/desktop'
import { tabId } from '@/stream'
import { carrySend } from './carried-send'
import { uploadImages, type PendingImage } from './upload-images'

/** The API requires a title; the first message and harness supply its meaningful replacements. */
const DRAFT_TITLE = 'New conversation'

/** Send from a draft: create the thread, open its session for the model it
 * holds, start the turn with the chosen mode, then replace the draft's URL
 * with the thread's. A failed create stays in the draft for a retry with the
 * same command id; a turn that fails once the thread exists goes to the
 * thread with its text and error, so a retry never makes a second thread.
 * A person who left the draft while it sent stays where they went: the
 * thread shows in the sidebar, and nothing pulls them back to it. */
export function useFirstSend(projectId: string, harness: string, planId?: string, profile?: string) {
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  const create = useRef<Attempt | null>(null)
  const shown = useRef(true)
  useEffect(() => {
    shown.current = true
    return () => {
      shown.current = false
    }
  }, [])

  return async (commandId: string, text: string, chosen: TurnSettings, images: PendingImage[], changed: () => void): Promise<string> => {
    const request = { title: DRAFT_TITLE, harness, ...(profile === undefined ? {} : { profile }) }
    create.current = attemptFor(create.current, request)
    const thread = await createThread(projectId, { ...request, command_id: create.current.commandId })
    await noteCompanionSend(thread.id)
    void queryClient.invalidateQueries({ queryKey: threadsQuery(projectId).queryKey })
    if (profile !== undefined) void queryClient.invalidateQueries({ queryKey: agentThreadsQuery(profile).queryKey })
    const open = async () => {
      if (!shown.current) return
      await navigate({
        to: '/projects/$projectId/threads/$threadId',
        params: { projectId, threadId: thread.id },
        replace: true,
      })
    }

    let settings: TurnSettings | null = null
    try {
      let choices = await openSession(thread.id)
      if (profile !== undefined && chosen.model !== choices.current.model) {
        choices = await changeModel(thread.id, chosen.model)
      }
      settings = { mode: chosen.mode, model: choices.current.model, effort: choices.current.effort }
      if (images.length > 0 && !choices.capabilities.image) {
        throw new Error(choices.capabilities.image_reason ?? 'This session does not support images.')
      }
      const attachments = await uploadImages(thread.id, images, changed)
      const operationId = await startTurn(thread.id, commandId, text, settings, {
        plan: planId ?? null,
        clientTab: tabId(),
        attachments,
      })
      void open()
      return operationId
    } catch (error) {
      // The thread page's composer fingerprints what it sends the same way,
      // so sending the same text and settings there replays this command.
      const request = { text, settings: settings ?? chosen, ...(images.length === 0 ? {} : { attachments: images.map(image => image.key) }), focus: null, ...(planId === undefined ? {} : { plan: planId }) }
      carrySend(thread.id, {
        text,
        images,
        attempt: attemptFor(null, request, error instanceof ApiError && error.problem.kind === 'daemon' && error.problem.code === 'COMMAND_CONFLICT' ? undefined : () => commandId),
        error: error instanceof Error ? error : new Error(String(error)),
        planId,
      })
      void open()
      throw error
    }
  }
}
