// One job: show one conversation image as a thumbnail that opens the whole image in a dialog.

import { useRef, useState } from 'react'
import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog'

export function ImageZoom({ alt, src, image, className, loading }: {
  alt: string
  /** A URL from attachmentUrl; or omit and pass `image`. */
  src?: string
  /** Sets and cleans up a node's src (the composer's local object URL). Used for both images. */
  image?: (node: HTMLImageElement | null) => void | (() => void)
  /** Classes for the thumbnail image (its size and fit). */
  className?: string
  /** Defers loading the thumbnail only. */
  loading?: 'lazy'
}) {
  const [open, setOpen] = useState(false)
  const opener = useRef<HTMLButtonElement>(null)
  // A saved image lives on the daemon, another origin: only a CORS request carries the Origin its guard needs.
  return (
    <>
      <button ref={opener} type="button" aria-label={`View ${alt}`} onClick={() => setOpen(true)}
        className="block cursor-zoom-in rounded-lg focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none">
        <img src={src} ref={image} crossOrigin="anonymous" alt={alt} loading={loading} className={className} />
      </button>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent finalFocus={opener}
          className="w-auto max-w-none border-0 bg-transparent p-0 shadow-none sm:max-w-none">
          <DialogTitle className="sr-only">{alt}</DialogTitle>
          <img src={src} ref={image} crossOrigin="anonymous" alt={alt}
            className="max-h-[calc(100dvh-4rem)] max-w-[calc(100vw-4rem)] rounded-lg object-contain" />
        </DialogContent>
      </Dialog>
    </>
  )
}
