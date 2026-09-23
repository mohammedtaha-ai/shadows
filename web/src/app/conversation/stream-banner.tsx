// One job: telling a person the conversation's live stream is not live, and
// offering to reconnect when it has given up.

import { CircleAlert, LoaderCircle } from 'lucide-react'
import { Button } from '@/components/ui/button'
import type { Connection } from '@/stream/thread-stream'

export function StreamBanner({
  connection,
  problem,
  retry,
}: {
  connection: Connection
  problem: string | null
  retry: () => void
}) {
  if (connection === 'reconnecting') {
    return (
      <p className="flex items-center justify-center gap-2 border-b border-border py-1.5 text-xs text-faint-foreground">
        <LoaderCircle className="size-3 animate-spin motion-reduce:animate-none" />
        Reconnecting
      </p>
    )
  }
  if (connection !== 'failed') return null
  return (
    <div
      role="alert"
      className="flex items-center justify-center gap-3 border-b border-destructive-border bg-accent-softer px-4 py-2 text-xs text-muted-foreground"
    >
      <CircleAlert className="size-3.5 shrink-0 text-destructive-foreground" />
      <span className="truncate">Live updates stopped{problem === null ? '' : `: ${problem}`}</span>
      <Button variant="outline" size="xs" onClick={retry}>
        Retry
      </Button>
    </div>
  )
}
