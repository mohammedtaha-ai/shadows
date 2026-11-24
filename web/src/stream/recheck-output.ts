// One job: holding one project recheck batch's live output stream while a component is mounted,
// and folding its frames into the text each check has printed so far.

import { useQueryClient } from '@tanstack/react-query'
import { useEffect, useState } from 'react'
import { holdRecheckStream, invalidateRecheck, recheckOutputUrl } from '@/api/runs'

export interface RecheckOutput {
  /** Text printed so far, by `rowKey`; only rows this connection saw start. */
  text: Record<string, string>
  /** The row printing now, as a `rowKey`. */
  running: string | null
  /** The batch finished and the stream is closed. */
  ended: boolean
  /** The client fell behind and missed output; the stored output is the whole of it. */
  lagged: boolean
}

interface Frame { task?: number; attempt_id?: string; check_id?: string; text?: string }

/** The key a row has in `RecheckOutput.text`; the same for a stored row and a frame. */
export function rowKey(row: { task: number; attempt_id: string; check_id: string }): string {
  return `${row.task}/${row.attempt_id}/${row.check_id}`
}

const START: RecheckOutput = { text: {}, running: null, ended: false, lagged: false }

/** Folds one frame in. A reconnect resends a running row's `started` and its output so far, so
 * `started` restarts the row's text. */
export function foldFrame(state: RecheckOutput, type: string, data: Frame): RecheckOutput {
  const key = typeof data.task === 'number' && data.attempt_id !== undefined && data.check_id !== undefined
    ? rowKey({ task: data.task, attempt_id: data.attempt_id, check_id: data.check_id })
    : null
  switch (type) {
    case 'started':
      return key === null ? state : { ...state, text: { ...state.text, [key]: '' }, running: key }
    case 'chunk':
      return key === null ? state
        : { ...state, text: { ...state.text, [key]: (state.text[key] ?? '') + (data.text ?? '') }, running: key }
    case 'finished':
      return { ...state, running: state.running === key ? null : state.running }
    case 'lagged':
      return { ...state, lagged: true }
    case 'end':
      return { ...state, running: null, ended: true }
    default:
      return state
  }
}

/** The part of the browser's `EventSource` this module uses. */
export interface OutputSource {
  addEventListener(type: string, listener: (event: Event) => void): void
  close(): void
}

const open = (url: string): OutputSource => new EventSource(url)

/** Follows a batch's output while mounted. Each finished row and the end refetch what Core stores
 * for that workflow and batch only; while it follows, the page does not poll the batch. */
export function useRecheckOutput(
  recheckId: string | undefined,
  workflowId: string,
  connect: (url: string) => OutputSource = open,
): RecheckOutput {
  const queryClient = useQueryClient()
  // Keyed by batch, so a new batch starts empty without a reset inside the effect.
  const [held, setHeld] = useState<{ id: string | undefined; state: RecheckOutput }>({ id: recheckId, state: START })
  useEffect(() => {
    if (recheckId === undefined) return
    const release = holdRecheckStream(recheckId)
    const source = connect(recheckOutputUrl(recheckId))
    for (const type of ['started', 'chunk', 'finished', 'lagged', 'end']) {
      source.addEventListener(type, (event) => {
        let data: Frame = {}
        try { data = JSON.parse((event as MessageEvent<string>).data) as Frame } catch { /* an empty frame */ }
        setHeld((previous) => ({
          id: recheckId, state: foldFrame(previous.id === recheckId ? previous.state : START, type, data),
        }))
        if (type === 'finished' || type === 'end') invalidateRecheck(queryClient, workflowId, recheckId)
        // An ended batch answers `end` once; closing keeps EventSource from reconnecting to it.
        if (type === 'end') source.close()
      })
    }
    return () => { source.close(); release() }
  }, [recheckId, workflowId, queryClient, connect])
  return held.id === recheckId ? held.state : START
}
