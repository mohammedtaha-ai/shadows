// Present selected images and release their local previews when they leave the screen.

import { Loader2, RotateCw, X } from 'lucide-react'
import { useCallback } from 'react'
import { attachmentUrl } from '@/api/threads'
import { cn } from 'cn'
import { ImageZoom } from './image-zoom'
import type { PendingImage } from './upload-images'

const corner = 'absolute flex items-center justify-center rounded-full bg-foreground text-background focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none disabled:opacity-50'

export function PendingImages({ images, threadId, inFlight, onRetry, onRemove }: {
  images: PendingImage[]
  threadId: string | null
  inFlight: boolean
  onRetry: (image: PendingImage) => void
  onRemove: (image: PendingImage) => void
}) {
  if (images.length === 0) return null
  return (
    <div className="space-y-1">
      <ul aria-label="Images" className="flex flex-wrap gap-2">
        {images.map(image => <Tile key={image.key} image={image} threadId={threadId} inFlight={inFlight}
          onRetry={onRetry} onRemove={onRemove} />)}
      </ul>
      {images.map(image => image.error && <p key={image.key} className="text-xs text-destructive-foreground">
        {image.file.name}: {image.error.message}
      </p>)}
    </div>
  )
}

function Tile({ image, threadId, inFlight, onRetry, onRemove }: {
  image: PendingImage
  threadId: string | null
  inFlight: boolean
  onRetry: (image: PendingImage) => void
  onRemove: (image: PendingImage) => void
}) {
  const preview = usePreview(image, threadId)
  const upload = image.error ? 'failed' : image.attachment ? 'uploaded' : threadId === null ? 'local' : 'uploading'
  return (
    <li data-upload={upload} aria-busy={upload === 'uploading' || undefined} className="group relative size-16">
      <div className={cn('size-full overflow-hidden rounded-lg', upload === 'uploading' && 'opacity-60',
        upload === 'failed' && 'ring-2 ring-destructive-foreground')}>
        <ImageZoom image={preview} alt={image.file.name} className="size-16 object-cover" />
      </div>
      {upload === 'uploading' && <Loader2 aria-hidden
        className="pointer-events-none absolute inset-0 m-auto size-4 animate-spin motion-reduce:animate-none" />}
      {upload === 'failed' && threadId !== null && <button type="button" disabled={inFlight}
        aria-label={`Retry ${image.file.name}`} title={image.error?.message}
        onClick={() => onRetry(image)} className={cn(corner, 'inset-0 m-auto size-6')}>
        <RotateCw aria-hidden className="size-3.5" />
      </button>}
      <button type="button" aria-label={`Remove ${image.file.name}`} disabled={inFlight}
        onClick={() => onRemove(image)}
        className={cn(corner, '-top-1.5 -end-1.5 size-5 opacity-0 group-hover:opacity-100 group-focus-within:opacity-100')}>
        <X aria-hidden className="size-3" />
      </button>
    </li>
  )
}

/** Sets an image's src to its saved bytes, or to a local object URL it revokes when the image leaves. */
function usePreview(image: PendingImage, threadId: string | null) {
  const file = image.file
  const saved = image.attachment?.id
  return useCallback((node: HTMLImageElement | null) => {
    if (node === null) return
    if (saved !== undefined && threadId !== null) {
      node.src = attachmentUrl(threadId, saved)
      return
    }
    const local = URL.createObjectURL(file)
    node.src = local
    return () => URL.revokeObjectURL(local)
  }, [file, saved, threadId])
}
