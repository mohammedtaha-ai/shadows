// Own the selected images and their upload lifecycle.

import { useRef, useState } from 'react'
import type { SessionChoices } from '@/api/sessions'
import { uploadImages, retryImageUpload, type PendingImage } from './carried-send'

export function useImages(
  choices: SessionChoices | null,
  threadId: string | null,
  inFlight: boolean,
  changed: () => void,
  refuse: (error: Error) => void,
  initialImages: PendingImage[] = [],
) {
  const filePicker = useRef<HTMLInputElement | null>(null)
  const [images, setImages] = useState<PendingImage[]>(initialImages)
  const imagesChanged = () => setImages(now => [...now])
  const imageReason = choices === null ? 'The session is not ready yet.'
    : !choices.capabilities.image ? choices.capabilities.image_reason ?? 'This session does not support images.' : null
  const add = (files: File[]) => {
    if (imageReason !== null) {
      refuse(new Error(imageReason))
      return
    }
    if (inFlight) return
    const added = files.map(file => ({ key: crypto.randomUUID(), file }))
    changed()
    setImages(now => [...now, ...added])
    if (threadId !== null) void uploadImages(threadId, added, imagesChanged).catch(() => {})
  }
  const pick = () => {
    if (imageReason === null && !inFlight) filePicker.current?.click()
    else if (imageReason !== null) refuse(new Error(imageReason))
  }

  const retry = (image: PendingImage) => {
    if (threadId !== null) void retryImageUpload(threadId, image, imagesChanged).catch(() => {})
  }
  const remove = (image: PendingImage) => {
    changed()
    setImages(now => now.filter(item => item !== image))
  }
  const drop = (sent: PendingImage[]) => setImages(now => now.filter(image => !sent.includes(image)))

  return { images, imageReason, filePicker, add, pick, retry, remove, drop, imagesChanged }
}
