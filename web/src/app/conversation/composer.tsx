// One job: writing to the Planner — the prompt box, Send, and Stop while a
// turn runs.

import { useMutation } from '@tanstack/react-query'
import { ArrowUp, Square } from 'lucide-react'
import { useState } from 'react'
import { type TurnSettings, startTurn, stopTurn } from '@/api/client'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'
import type { Turn } from './turn-state'

export function Composer({
  threadId,
  running,
  directory,
  known,
  settings,
  onStarted,
}: {
  threadId: string
  /** What the turn runs with; `null` until the session has answered. */
  settings: TurnSettings | null
  running: Turn | null
  directory: string | null | undefined
  /** Whether a turn is running is known yet; Send waits for it. */
  known: boolean
  /** The start route answered: the turn exists before its events arrive. */
  onStarted: (operationId: string) => void
}) {
  const [prompt, setPrompt] = useState('')

  const send = useMutation({
    mutationFn: ({ text, chosen }: { text: string; chosen: TurnSettings }) =>
      startTurn(threadId, crypto.randomUUID(), text, chosen),
    onSuccess: (operationId) => {
      setPrompt('')
      onStarted(operationId)
    },
  })
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

  const submit = () => {
    const text = prompt.trim()
    if (text === '' || !known || settings === null || send.isPending || running !== null) return
    send.mutate({ text, chosen: settings })
  }

  const error = send.error ?? stop.error

  return (
    <div className="border-t border-border bg-background px-6 pt-3 pb-4">
      <div className="mx-auto max-w-3xl space-y-2">
        <div className="flex items-end gap-2 rounded-xl border border-accent-line/40 bg-input-background p-2 focus-within:border-accent-line focus-within:ring-3 focus-within:ring-ring/30">
          <textarea
            value={prompt}
            onChange={(e) => setPrompt(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && !e.shiftKey && !e.nativeEvent.isComposing) {
                e.preventDefault()
                submit()
              }
            }}
            rows={2}
            placeholder="Ask the Planner…"
            aria-label="Message"
            className="max-h-48 min-h-10 flex-1 resize-none bg-transparent px-2 py-1.5 text-sm text-foreground outline-none placeholder:text-faint-foreground"
          />
          {running === null ? (
            <Button
              onClick={submit}
              disabled={prompt.trim() === '' || !known || settings === null || send.isPending}
              size="icon"
              aria-label="Send"
            >
              <ArrowUp />
            </Button>
          ) : (
            <Button
              variant="outline"
              onClick={() => stop.mutate(running.id)}
              disabled={stop.isPending}
              className="border-destructive-border text-destructive-foreground hover:bg-destructive/10 hover:text-destructive-foreground"
            >
              <Square className="size-3 fill-current" />
              {stopLabel}
            </Button>
          )}
        </div>
        {error !== null && <ErrorLine error={error} />}
        {directory != null && (
          <p className="px-1 font-mono text-[11px] text-faint-foreground">Runs in {directory}</p>
        )}
      </div>
    </div>
  )
}
