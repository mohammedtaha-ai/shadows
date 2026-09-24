// One job: writing to the Planner — the prompt and the settings it runs with,
// Send, and Stop while a turn runs.

import { useMutation } from '@tanstack/react-query'
import { ArrowUp, Square } from 'lucide-react'
import { type ReactNode, useRef, useState } from 'react'
import { type SessionChoices, type TurnSettings, startTurn, stopTurn } from '@/api/client'
import { type Attempt, attemptFor } from '@/api/command-id'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'
import { ComposerBar } from './composer-bar'
import { afterOptions, initialSettings, sendable } from './turn-settings'
import type { Turn } from './turn-state'
import type { SessionView } from './use-session'

interface Send {
  commandId: string
  text: string
  settings: TurnSettings
}

export function Composer({
  threadId,
  harnessLabel,
  session,
  running,
  directory,
  known,
  ring,
  onStarted,
}: {
  threadId: string
  harnessLabel: string
  session: SessionView
  running: Turn | null
  directory: string | null | undefined
  /** Whether a turn is running is known yet; Send waits for it. */
  known: boolean
  ring?: ReactNode
  /** The start route answered: the turn exists before its events arrive. */
  onStarted: (operationId: string) => void
}) {
  const [prompt, setPrompt] = useState('')

  // The command id belongs to a pending send (spec §12.7): made when Send is
  // pressed with no pending send, reused by a retry of that same send, and
  // dropped when it succeeds or when the prompt or a setting changes, so a
  // changed request is a new command rather than a COMMAND_CONFLICT.
  const pending = useRef<Attempt | null>(null)

  const send = useMutation({
    mutationFn: ({ commandId, text, settings }: Send) =>
      startTurn(threadId, commandId, text, settings),
    onSuccess: (operationId) => {
      pending.current = null
      setPrompt('')
      onStarted(operationId)
    },
  })

  const choices = session.state === 'ready' ? session.choices : null
  const { settings, note, choose } = useTurnSettings(
    choices,
    send.isPending || running !== null,
    () => {
      pending.current = null
    },
  )
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

  const ready = known && choices !== null && settings !== null && sendable(choices, settings)

  const submit = () => {
    const text = prompt.trim()
    if (text === '' || !ready || settings === null || send.isPending || running !== null) return
    pending.current = attemptFor(pending.current, { text, settings })
    send.mutate({ commandId: pending.current.commandId, text, settings })
  }

  const error = send.error ?? stop.error

  return (
    <div className="border-t border-border bg-background px-6 pt-3 pb-4">
      <div className="mx-auto max-w-3xl space-y-2">
        <div className="flex items-end gap-2 rounded-xl border border-accent-line/40 bg-input-background p-2 focus-within:border-accent-line focus-within:ring-3 focus-within:ring-ring/30">
          <textarea
            value={prompt}
            onChange={(e) => {
              pending.current = null
              setPrompt(e.target.value)
            }}
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
              disabled={prompt.trim() === '' || !ready || send.isPending}
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
        <ComposerBar
          harnessLabel={harnessLabel}
          session={session}
          settings={settings}
          onSettings={choose}
          directory={directory}
          note={note}
          ring={ring}
        />
        {error !== null && <ErrorLine error={error} />}
      </div>
    </div>
  )
}

/** The settings the next turn runs with: the session's own at first, then the
 * person's, following the session's reports (`afterOptions`) as they arrive.
 * `changed` runs whenever the person changes one. */
function useTurnSettings(
  choices: SessionChoices | null,
  busy: boolean,
  changed: () => void,
): { settings: TurnSettings | null; note: string | null; choose: (next: TurnSettings) => void } {
  const [held, setHeld] = useState<{
    choices: SessionChoices
    settings: TurnSettings
    note: string | null
  } | null>(null)

  // Adjusted during render, React's pattern for following a changed input:
  // the session's choices are replaced wholesale on every report.
  if (choices === null && held !== null) setHeld(null)
  if (choices !== null && held?.choices !== choices) {
    if (held === null) {
      setHeld({ choices, settings: initialSettings(choices), note: null })
    } else {
      const next = afterOptions(held.choices, choices, held.settings, busy)
      // A note stays until the person changes a setting.
      setHeld({ choices, settings: next.settings, note: next.note ?? held.note })
    }
  }

  return {
    settings: held?.settings ?? null,
    note: held?.note ?? null,
    choose: (next) => {
      changed()
      setHeld((h) => (h === null ? h : { ...h, settings: next, note: null }))
    },
  }
}
