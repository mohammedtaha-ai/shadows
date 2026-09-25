// One job: a button that puts a text on the clipboard and says, briefly, that
// it did.

import { Check, Copy } from 'lucide-react'
import { type ComponentProps, useEffect, useState } from 'react'
import { Button } from '@/components/ui/button'

/** How long "Copied" shows after a copy. */
const COPIED_MS = 1500

export function CopyButton({
  text,
  size,
}: {
  text: string
  size: ComponentProps<typeof Button>['size']
}) {
  const [copied, setCopied] = useState(false)
  useEffect(() => {
    if (!copied) return
    const timer = setTimeout(() => setCopied(false), COPIED_MS)
    return () => clearTimeout(timer)
  }, [copied])

  const copy = () => {
    navigator.clipboard.writeText(text).then(
      () => setCopied(true),
      // The browser refused (no permission, or the page is not focused):
      // nothing was copied, and the button does not say it was.
      () => setCopied(false),
    )
  }

  return (
    <Button
      variant="ghost"
      size={size}
      onClick={copy}
      aria-label={copied ? 'Copied' : 'Copy'}
      title={copied ? 'Copied' : 'Copy'}
    >
      {copied ? <Check /> : <Copy />}
    </Button>
  )
}
