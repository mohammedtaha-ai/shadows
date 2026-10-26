// A stop answer does not end the turn; the running state owns that fact.

import { useMutation } from '@tanstack/react-query'
import { stopTurn } from '@/api/threads'
import type { Turn } from './turn-state'

export function useStop(running: Turn | null) {
  const stop = useMutation({ mutationFn: stopTurn })

  // "Stopping" until the durable ending arrives and `running` clears: the
  // stop call answering is not the turn ending. A stop this client saw fail
  // (PROCESS_TERMINATION_FAILED: the tree may still be running) offers Stop
  // again, even though the request is durable — asking again is safe, and a
  // button stuck on "Stopping…" over a live tree could never be retried. Only
  // a stop in flight disables it; a "Stopping…" learned from the daemon (after
  // a reload, say) stays pressable for the same reason.
  const mine = running !== null && stop.variables === running.id
  const failed = mine && stop.isError
  const stopping = running !== null && !failed && (running.stopRequested || mine)
  const stopLabel = failed ? 'Stop again' : stopping ? 'Stopping…' : 'Stop'

  return { stop: stop.mutate, isPending: stop.isPending, error: stop.error, label: stopLabel }
}
