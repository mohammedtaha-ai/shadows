// One job: carrying a draft's first message to its thread's page when the
// thread was made and the turn failed (spec §13.11), so the page shows the
// text and the error, and a retry reuses the turn's command id.

import { useEffect, useState } from 'react'
import { uploadAttachment, type Attachment } from '@/api/client'
import { ApiError } from '@/api/error'
import type { Attempt } from '@/api/command-id'

/** A picked file retains its upload command and answer across a draft handoff. */
export interface PendingImage {
  readonly key: string
  readonly file: File
  commandId?: string
  attachment?: Attachment
  error?: Error
  upload?: Promise<void>
}

export interface CarriedSend {
  readonly images?: PendingImage[]
  readonly text: string
  /** The failed turn's command, as `attemptFor` would have made it. */
  readonly attempt: Attempt
  readonly error: Error
  /** The selected plan remains part of a retry's command and prompt context. */
  readonly planId?: string
}

const carried = new Map<string, CarriedSend>()

export function carrySend(threadId: string, send: CarriedSend): void {
  carried.set(threadId, send)
}

/** What was carried to `threadId`, read once: taken on the first render and
 * dropped after it, so a later visit starts empty. */
export function useCarriedSend(threadId: string): CarriedSend | null {
  const [send] = useState(() => carried.get(threadId) ?? null)
  useEffect(() => {
    carried.delete(threadId)
  }, [threadId])
  return send
}

/** Shares uploads between immediate picking and Send without uploading twice. */
export async function uploadImages(threadId: string, images: PendingImage[], changed: () => void): Promise<string[]> {
  await Promise.all(images.map(image => {
    if (image.attachment !== undefined || image.error !== undefined) return
    image.upload ??= uploadAttachment(threadId, image.commandId ??= crypto.randomUUID(), image.file).then(attachment => {
      image.attachment = attachment
    }).catch((error: unknown) => {
      image.error = error instanceof Error ? error : new Error(String(error))
      if (error instanceof ApiError && error.problem.kind === 'daemon' && error.problem.code === 'COMMAND_CONFLICT') image.commandId = undefined
    }).finally(changed)
    return image.upload
  }))
  const failed = images.find(image => image.error !== undefined)
  if (failed !== undefined) throw failed.error
  return images.map(image => image.attachment!.id)
}

/** An explicit retry keeps the immutable file and its replayable upload command. */
export function retryImageUpload(threadId: string, image: PendingImage, changed: () => void): Promise<string[]> {
  image.error = undefined
  image.upload = undefined
  changed()
  return uploadImages(threadId, [image], changed)
}
