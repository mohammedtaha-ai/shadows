// Own the settings held for the next turn and their session mutations.

import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useEffect, useRef, useState } from 'react'
import { type SessionChoices, type TurnSettings, changeEffort, changeModel, changeMode, saveSessionAnswer } from '@/api/sessions'
import { type Attempt, attemptFor } from '@/api/command-id'
import { ApiError } from '@/api/error'
import { afterOptions, initialSettings, withModel } from './turn-settings'

export function useSessionSettings(
  choices: SessionChoices | null,
  busy: boolean,
  threadId: string | null,
  changed: () => void,
) {
  const queryClient = useQueryClient()
  const { settings, note, choose: chooseLocal, refuse, keepEffort } = useTurnSettings(choices, busy, threadId !== null, changed)

  // A lost answer may already have saved the mode. Keep its replay key until
  // Core answers, or the person chooses a different request.
  const modeAttempt = useRef<Attempt | null>(null)
  const switchMode = useMutation({
    mutationFn: ({ threadId, commandId, mode }: { threadId: string; commandId: string; mode: string }) =>
      changeMode(threadId, commandId, mode),
    onSuccess: async (answer, { threadId }) => {
      modeAttempt.current = null
      await saveSessionAnswer(queryClient, threadId, answer)
    },
    onError: (error) => {
      if (error instanceof ApiError && error.problem.kind === 'daemon') modeAttempt.current = null
    },
  })
  const choose = (next: TurnSettings) => {
    if (threadId !== null && choices !== null && next.mode !== choices.current.mode) {
      changed()
      modeAttempt.current = attemptFor(modeAttempt.current, { threadId, mode: next.mode })
      switchMode.mutate({ threadId, commandId: modeAttempt.current.commandId, mode: next.mode })
    } else {
      modeAttempt.current = null
      chooseLocal(next)
    }
  }

  // A picked model is set on the session at once, so its efforts
  // are known before Send. The answer is the session's new choices; a refusal
  // puts the session's model back and says why. A turn's session is not
  // changed while it runs: the change is asked once the turn has ended.
  const switchModel = useMutation({
    // A draft has no session to change: Send opens it.
    mutationFn: ({ threadId, id }: { threadId: string; id: string }) => changeModel(threadId, id),
    onSuccess: async (answer, { threadId, id }) => {
      await saveSessionAnswer(queryClient, threadId, answer)
      // Asked again it would be answered the same: say so rather than ask.
      if (answer.current.model !== id) refuse(`The session kept ${answer.current.model}`, answer)
      // The answer is the effort the move ended at (the remembered one);
      // a report streamed during the move may still carry the previous one.
      else keepEffort(answer)
    },
    onError: (error) => refuse(error.message),
  })
  const wanted = settings?.model
  const held = choices?.current.model
  const requestModel = switchModel.mutate
  const changing = switchModel.isPending
  useEffect(() => {
    if (busy || changing || wanted === undefined || held === undefined || wanted === held) return
    if (threadId !== null) requestModel({ threadId, id: wanted })
  }, [busy, changing, wanted, held, requestModel, threadId])

  // The same for a picked effort, once the session holds the chosen model:
  // the session then reports the effort the turn will run at. A refusal puts
  // the session's effort back, and the error line says why.
  const switchEffort = useMutation({
    mutationFn: ({ threadId, id }: { threadId: string; id: string }) => changeEffort(threadId, id),
    onSuccess: async (answer, { threadId, id }) => {
      await saveSessionAnswer(queryClient, threadId, answer)
      if (answer.current.effort !== id) keepEffort(answer)
    },
    onError: () => keepEffort(),
  })
  const wantedEffort = settings?.effort ?? null
  const heldEffort = choices?.current.effort ?? null
  const requestEffort = switchEffort.mutate
  const changingEffort = switchEffort.isPending
  useEffect(() => {
    if (busy || changing || changingEffort || threadId === null) return
    if (wanted !== held || wantedEffort === null || wantedEffort === heldEffort) return
    requestEffort({ threadId, id: wantedEffort })
  }, [busy, changing, changingEffort, wanted, held, wantedEffort, heldEffort, requestEffort, threadId])

  return {
    settings,
    note,
    choose,
    switching: changing || changingEffort || switchMode.isPending,
    error: switchMode.error ?? switchEffort.error,
    resetError: switchEffort.reset,
  }
}

/** The settings the next turn runs with: the session's own at first, then the
 * person's, following the session's reports (`afterOptions`) as they arrive.
 * `changed` runs whenever the person changes one; `refuse` puts the model
 * back to the one the session holds (`session`, when it just answered) and
 * shows why; `keepEffort` puts the effort back to the session's. */
function useTurnSettings(
  choices: SessionChoices | null,
  busy: boolean,
  saved: boolean,
  changed: () => void,
): {
  settings: TurnSettings | null
  note: string | null
  choose: (next: TurnSettings) => void
  refuse: (why: string, session?: SessionChoices) => void
  keepEffort: (session?: SessionChoices) => void
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
      setHeld({ choices, settings: saved ? choices.current : initialSettings(choices), note: null })
    } else {
      const next = afterOptions(held.choices, choices, held.settings, busy)
      // A note stays until the person changes a setting.
      setHeld({ choices, settings: saved ? { ...next.settings, mode: choices.current.mode } : next.settings, note: next.note ?? held.note })
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
    keepEffort: (session) => {
      changed()
      setHeld((h) => {
        if (h === null) return h
        const effort = (session ?? h.choices).current.effort
        return { ...h, settings: { ...h.settings, effort } }
      })
    },
  }
}

