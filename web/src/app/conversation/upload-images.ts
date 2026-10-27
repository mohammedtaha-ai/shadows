// Keep upload identity and answers across selection, Send and draft handoff.

import { uploadAttachment, type Attachment } from '@/api/threads'
import { ApiError } from '@/api/error'

/** A picked file retains its upload command and answer across a draft handoff. */
export interface PendingImage {
  readonly key: string
  readonly file: File
  commandId?: string
  attachment?: Attachment
  error?: Error
  upload?: Promise<void>
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
