// One job: what the client knows about a thread's turns — which is running,
// whether it was asked to stop, and how the latest one ended.
//
// Two sources feed it, in either order: the operations route (a snapshot, read
// on opening the thread and again at each `caught-up`) and the stream's durable
// operation events, which name their operation. Core stamps each with the
// journal `seq` of the operation's latest event and the status that event left,
// so per turn the source with the higher `seq` wins and the other cannot pull
// the turn back. The status itself is Core's answer, never derived here.

import type { Operation } from '@/api/threads'
import type { DurableEvent } from '@/stream'

export type Status = 'Pending' | 'Running' | 'Completed' | 'Failed' | 'Cancelled' | 'Interrupted'

export interface Turn {
  readonly id: string
  readonly status: Status
  /** A stop was asked for and the turn has not ended yet, or ended since. */
  readonly stopRequested: boolean
  /** When this client learned the turn had ended (ms), else `null`. */
  readonly endedAt: number | null
  /** Journal seq of the source that set this turn; -1 when only the start route has answered. */
  readonly seq: number
}

/** Newest first. */
export type TurnState = readonly Turn[]

export const initialTurnState: TurnState = []

export type TurnAction =
  | { type: 'snapshot'; operations: readonly Operation[]; now: number }
  | { type: 'event'; event: DurableEvent; now: number }
  /** The start route answered with this operation before its events arrived. */
  | { type: 'started'; id: string }

// The only status knowledge kept: whether a status is the end, which the screen
// needs to offer Stop and to stamp `endedAt`. Core owns every other rule.
const TERMINAL: ReadonlySet<Status> = new Set(['Completed', 'Failed', 'Cancelled', 'Interrupted'])

/** The start route's answer carries no seq, so any row or event replaces it. */
const UNORDERED = -1

export function isTerminal(status: Status): boolean {
  return TERMINAL.has(status)
}

export function turnReducer(state: TurnState, action: TurnAction): TurnState {
  switch (action.type) {
    case 'started':
      return upsert(state, action.id, UNORDERED, 'Pending', false, 0)
    case 'event': {
      const { event, now } = action
      if (event.operationId === null || !event.kind.startsWith('Operation')) return state
      const { status_kind, cancel_requested } = readPayload(event.payload)
      if (status_kind === undefined) return state
      return upsert(
        state,
        event.operationId,
        event.seq,
        asStatus(status_kind),
        cancel_requested,
        now,
      )
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
          op.seq,
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
  seq: number,
  status: Status,
  stopRequested: boolean,
  now: number,
): TurnState {
  const index = state.findIndex((turn) => turn.id === id)
  if (index === -1) return [merge(undefined, id, seq, status, stopRequested, now), ...state]
  const next = [...state]
  next[index] = merge(state[index], id, seq, status, stopRequested, now)
  return next
}

/** The higher seq wins; an equal one keeps what is known, so a replay changes nothing. */
function merge(
  known: Turn | undefined,
  id: string,
  seq: number,
  status: Status,
  stopRequested: boolean,
  now: number,
): Turn {
  if (known !== undefined && seq <= known.seq) return known
  return {
    id,
    status,
    stopRequested,
    endedAt: isTerminal(status) ? (known?.endedAt ?? now) : null,
    seq,
  }
}

/** A payload is `unknown` on the wire: read only what is there. */
function readPayload(payload: unknown): { status_kind: string | undefined; cancel_requested: boolean } {
  const fields = typeof payload === 'object' && payload !== null ? (payload as Record<string, unknown>) : {}
  return {
    status_kind: typeof fields.status_kind === 'string' ? fields.status_kind : undefined,
    cancel_requested: fields.cancel_requested === true,
  }
}

/** A status the daemon names that this client does not know is treated as
 * still pending, so an unknown future state never hides a Stop button. */
function asStatus(kind: string): Status {
  return kind === 'Running' || TERMINAL.has(kind as Status) ? (kind as Status) : 'Pending'
}
