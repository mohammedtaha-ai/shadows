// One job: writing to the Planner — the prompt and the settings it runs with,
// Send, and Stop while a turn runs.

import { useMutation, useQueryClient } from '@tanstack/react-query'
import { ArrowUp, Square } from 'lucide-react'
import { type ReactNode, useEffect, useRef, useState } from 'react'
import {
  type Focus,
  type SessionChoices,
  type TurnSettings,
  changeModel,
  startTurn,
  stopTurn,
} from '@/api/client'
import { type Attempt, attemptFor } from '@/api/command-id'
import { Button } from '@/components/ui/button'
import { tabId } from '@/stream/tab-id'
import { ErrorLine } from '../error-line'
import { ComposerBar } from './composer-bar'
import { FocusChip, type PointedTask } from './focus-chip'
import { afterOptions, effortsKnown, initialSettings, sendable, withModel } from './turn-settings'
import type { Turn } from './turn-state'
import { type SessionView, sessionKey } from './use-session'

interface Send {
  commandId: string
  text: string
  settings: TurnSettings
  pointed: PointedTask | null
}

export function Composer({
  threadId,
  harness,
  harnessLabel,
  session,
  running,
  directory,
  known,
  ring,
  onStarted,
  pointed,
  onPointed,
}: {
  threadId: string
  /** The thread's harness kind. */
  harness: string
  harnessLabel: string
  session: SessionView
  running: Turn | null
  directory: string | null | undefined
  /** Whether a turn is running is known yet; Send waits for it. */
  known: boolean
  ring?: ReactNode
  /** The start route answered: the turn exists before its events arrive. */
  onStarted: (operationId: string) => void
  /** The task the person points at (§13.9), sent with the next turn. */
  pointed: PointedTask | null
  /** Clears the chip: pressed ×, or the turn that carried it started. */
  onPointed: (done: PointedTask) => void
}) {
  const [prompt, setPrompt] = useState('')

  // The command id belongs to a pending send (spec §12.7): made when Send is
  // pressed with no pending send, reused by a retry of that same send, and
  // dropped when it succeeds or when the prompt or a setting changes, so a
  // changed request is a new command rather than a COMMAND_CONFLICT.
  const pending = useRef<Attempt | null>(null)

  const send = useMutation({
    mutationFn: ({ commandId, text, settings, pointed }: Send) =>
      startTurn(threadId, commandId, text, settings, {
        focus: focusOf(pointed),
        clientTab: tabId(),
      }),
    onSuccess: (operationId, { pointed }) => {
      pending.current = null
      setPrompt('')
      if (pointed !== null) onPointed(pointed)
      onStarted(operationId)
    },
  })

  const choices = session.state === 'ready' ? session.choices : null
  const busy = send.isPending || running !== null
  const { settings, note, choose, refuse } = useTurnSettings(choices, busy, () => {
    pending.current = null
  })
  const stop = useMutation({ mutationFn: stopTurn })

  // Spec §12.7: a picked model is set on the session at once, so its efforts
  // are known before Send. The answer is the session's new choices; a refusal
  // puts the session's model back and says why. A turn's session is not
  // changed while it runs: the change is asked once the turn has ended.
  const queryClient = useQueryClient()
  const switchModel = useMutation({
    mutationFn: (id: string) => changeModel(threadId, id),
    onSuccess: (answer, id) => {
      queryClient.setQueryData(sessionKey(threadId), answer)
      // Asked again it would be answered the same: say so rather than ask.
      if (answer.current.model !== id) refuse(`The session kept ${answer.current.model}`, answer)
    },
    onError: (error) => refuse(error.message),
  })
  const wanted = settings?.model
  const held = choices?.current.model
  const requestModel = switchModel.mutate
  const changing = switchModel.isPending
  useEffect(() => {
    if (busy || changing || wanted === undefined || held === undefined || wanted === held) return
    requestModel(wanted)
  }, [busy, changing, wanted, held, requestModel])

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

  // Send waits until the session holds the chosen model: only then are the
  // efforts it runs with that model's.
  const ready =
    known &&
    !changing &&
    choices !== null &&
    settings !== null &&
    effortsKnown(choices, settings.model) &&
    sendable(choices, settings)

  const submit = () => {
    const text = prompt.trim()
    if (text === '' || !ready || settings === null || send.isPending || running !== null) return
    // The focus is part of the command (§13.10): pointing elsewhere is a new one.
    pending.current = attemptFor(pending.current, { text, settings, focus: focusOf(pointed) })
    send.mutate({ commandId: pending.current.commandId, text, settings, pointed })
  }

  const error = send.error ?? stop.error

  return (
    <div className="border-t border-border bg-background px-6 pt-3 pb-4">
      <div className="mx-auto max-w-3xl space-y-2">
        {pointed !== null && <FocusChip pointed={pointed} onClear={() => onPointed(pointed)} />}
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
          changingModel={changing}
          harness={harness}
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
 * `changed` runs whenever the person changes one; `refuse` puts the model
 * back to the one the session holds (`session`, when it just answered) and
 * shows why. */
function useTurnSettings(
  choices: SessionChoices | null,
  busy: boolean,
  changed: () => void,
): {
  settings: TurnSettings | null
  note: string | null
  choose: (next: TurnSettings) => void
  refuse: (why: string, session?: SessionChoices) => void
} {
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
    refuse: (why, session) => {
      changed()
      setHeld((h) => {
        if (h === null) return h
        const back = (session ?? h.choices).current.model
        return { ...h, settings: withModel(h.choices, h.settings, back), note: why }
      })
    },
  }
}

/** What the turn sends: the task's id, never its number (§13.10). */
function focusOf(pointed: PointedTask | null): Focus | null {
  if (pointed === null) return null
  return { workflow_id: pointed.workflowId, task_id: pointed.task.id, revision: pointed.revision }
}
