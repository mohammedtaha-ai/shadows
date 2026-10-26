// Present selected images and release their local previews when they leave the screen.

import { useCallback } from 'react'
import { attachmentUrl } from '@/api/threads'
import type { PendingImage } from './upload-images'

export function PendingImages({ images, threadId, inFlight, onRetry, onRemove }: {
  images: PendingImage[]
  threadId: string | null
  inFlight: boolean
  onRetry: (image: PendingImage) => void
  onRemove: (image: PendingImage) => void
}) {
  if (images.length === 0) return null
  return (
    <ul aria-label="Images" className="flex flex-wrap gap-2">
      {images.map(image => <li key={image.key} className="max-w-40 space-y-1 text-xs">
        <ImagePreview image={image} threadId={threadId} />
        <p>{image.error?.message ?? (image.attachment ? 'Uploaded' : threadId === null ? 'Ready to upload on send' : 'Uploading...')}</p>
        {image.error && threadId !== null && <button type="button" disabled={inFlight}
          onClick={() => onRetry(image)}>Retry {image.file.name}</button>}
        <button type="button" aria-label={`Remove ${image.file.name}`} disabled={inFlight}
          onClick={() => onRemove(image)}>Remove</button>
      </li>)}
    </ul>
  )
}

function ImagePreview({ image, threadId }: { image: PendingImage; threadId: string | null }) {
  const file = image.file
  const saved = image.attachment?.id
  const preview = useCallback((node: HTMLImageElement | null) => {
    if (node === null) return
    if (saved !== undefined && threadId !== null) {
      node.src = attachmentUrl(threadId, saved)
      return
    }
    const local = URL.createObjectURL(file)
    node.src = local
    return () => URL.revokeObjectURL(local)
  }, [file, saved, threadId])
  return <img ref={preview} alt={file.name} className="size-24 rounded object-cover" />
}
