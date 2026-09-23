// One job: what the client knows about a thread's turns — which is running,
// whether it was asked to stop, and how the latest one ended.
//
// Two sources feed it, in either order: the operations route (a snapshot, read
// on opening the thread and again at each `caught-up`) and the stream's durable
// operation events, which name their operation. A status only moves forward —
// Pending, then Running, then one terminal status that never changes — so
// whichever source is behind cannot pull a turn back: a snapshot read before a
// turn completed does not un-complete it, and a replayed `OperationStarted`
// does not restart a turn the snapshot already saw end.

import type { Operation } from '@/api/client'
import type { DurableEvent } from '@/stream/frames'

export type Status = 'Pending' | 'Running' | 'Completed' | 'Failed' | 'Cancelled' | 'Interrupted'

export interface Turn {
  readonly id: string
  readonly status: Status
  /** A stop was asked for and the turn has not ended yet, or ended since. */
  readonly stopRequested: boolean
  /** When this client learned the turn had ended (ms), else `null`. */
  readonly endedAt: number | null
}

/** Newest first. */
export type TurnState = readonly Turn[]

export const initialTurnState: TurnState = []

export type TurnAction =
  | { type: 'snapshot'; operations: readonly Operation[]; now: number }
  | { type: 'event'; event: DurableEvent; now: number }
  /** The start route answered with this operation before its events arrived. */
  | { type: 'started'; id: string }

const TERMINAL: ReadonlySet<Status> = new Set(['Completed', 'Failed', 'Cancelled', 'Interrupted'])

const BY_EVENT: Readonly<Record<string, Status>> = {
  OperationCreated: 'Pending',
  OperationStarted: 'Running',
  OperationCompleted: 'Completed',
  OperationFailed: 'Failed',
  OperationCancelled: 'Cancelled',
  OperationInterrupted: 'Interrupted',
}

export function isTerminal(status: Status): boolean {
  return TERMINAL.has(status)
}

export function turnReducer(state: TurnState, action: TurnAction): TurnState {
  switch (action.type) {
    case 'started':
      return upsert(state, action.id, 'Pending', false, 0)
    case 'event': {
      const { event, now } = action
      if (event.operationId === null) return state
      if (event.kind === 'OperationCancellationRequested') {
        return upsert(state, event.operationId, 'Pending', true, now)
      }
      const status = BY_EVENT[event.kind]
      return status === undefined ? state : upsert(state, event.operationId, status, false, now)
    }
    case 'snapshot': {
      const listed = new Set(action.operations.map((op) => op.id))
      // Turns known only from events began after the snapshot was read, so
      // they are newer than all of it.
      const newer = state.filter((turn) => !listed.has(turn.id))
      const merged = action.operations.map((op) =>
        merge(
          state.find((turn) => turn.id === op.id),
          op.id,
          asStatus(op.status_kind),
          op.cancel_requested_at != null,
          action.now,
        ),
      )
      return [...newer, ...merged]
    }
  }
}

/** The turn running now, if any: the newest one that has not ended. */
export function runningTurn(state: TurnState): Turn | null {
  return state.find((turn) => !isTerminal(turn.status)) ?? null
}

/** The newest turn, running or not. */
export function latestTurn(state: TurnState): Turn | null {
  return state[0] ?? null
}

function upsert(
  state: TurnState,
  id: string,
  status: Status,
  stopRequested: boolean,
  now: number,
): TurnState {
  const index = state.findIndex((turn) => turn.id === id)
  if (index === -1) return [merge(undefined, id, status, stopRequested, now), ...state]
  const next = [...state]
  next[index] = merge(state[index], id, status, stopRequested, now)
  return next
}

function merge(
  known: Turn | undefined,
  id: string,
  status: Status,
  stopRequested: boolean,
  now: number,
): Turn {
  const current = known?.status
  const winner = current !== undefined && rank(current) >= rank(status) ? current : status
  const ended = isTerminal(winner)
  return {
    id,
    status: winner,
    stopRequested: (known?.stopRequested ?? false) || stopRequested,
    endedAt: ended ? (known?.endedAt ?? now) : null,
  }
}

function rank(status: Status): number {
  return isTerminal(status) ? 2 : status === 'Running' ? 1 : 0
}

/** A status the daemon names that this client does not know is treated as
 * still pending, so an unknown future state never hides a Stop button. */
function asStatus(kind: string): Status {
  return kind === 'Running' || TERMINAL.has(kind as Status) ? (kind as Status) : 'Pending'
}
