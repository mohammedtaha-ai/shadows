import { describe, expect, it } from 'vitest'
import type { Operation } from '@/api/client'
import type { DurableEvent } from '@/stream/frames'
import {
  type TurnAction,
  type TurnState,
  initialTurnState,
  latestTurn,
  runningTurn,
  turnReducer,
} from './turn-state'

let seq = 0
const event = (kind: string, operationId: string, now = 0): TurnAction => {
  const e: DurableEvent = { seq: ++seq, kind, operationId, threadId: 't', payload: {} }
  return { type: 'event', event: e, now }
}

const op = (id: string, status_kind: string, cancel_requested_at?: string): Operation => ({
  id,
  status_kind,
  cancel_requested_at: cancel_requested_at ?? null,
  kind: 'PlannerTurn',
  created_at: '2026-09-23T00:00:00Z',
  runtime_instance_id: 'r',
  invocation: null,
})

const snapshot = (operations: Operation[], now = 0): TurnAction => ({
  type: 'snapshot',
  operations,
  now,
})

const run = (...actions: TurnAction[]): TurnState =>
  actions.reduce(turnReducer, initialTurnState)

describe('turnReducer', () => {
  it('a reload during a running turn shows it running, then follows it to its end', () => {
    let state = run(
      snapshot([op('b', 'Running'), op('a', 'Completed')]),
      // The replay from the start of the thread arrives after the snapshot.
      event('OperationCreated', 'a'),
      event('OperationStarted', 'a'),
      event('OperationCompleted', 'a'),
      event('OperationCreated', 'b'),
      event('OperationStarted', 'b'),
    )
    expect(runningTurn(state)?.id).toBe('b')
    expect(latestTurn(state)?.id).toBe('b')

    state = turnReducer(state, event('OperationCancellationRequested', 'b'))
    expect(runningTurn(state)).toMatchObject({ id: 'b', stopRequested: true })

    state = turnReducer(state, event('OperationCancelled', 'b', 42))
    expect(runningTurn(state)).toBeNull()
    expect(latestTurn(state)).toMatchObject({ id: 'b', status: 'Cancelled', endedAt: 42 })
  })

  it('a snapshot read before a turn ended does not bring it back', () => {
    const state = run(
      event('OperationCreated', 'a'),
      event('OperationStarted', 'a'),
      event('OperationCompleted', 'a', 7),
      snapshot([op('a', 'Running')], 9),
    )
    expect(runningTurn(state)).toBeNull()
    expect(latestTurn(state)).toMatchObject({ status: 'Completed', endedAt: 7 })
  })

  it('a turn begun after the snapshot is newest, and stays so when the snapshot is read again', () => {
    let state = run(snapshot([op('a', 'Completed')]), event('OperationCreated', 'b'))
    expect(runningTurn(state)?.id).toBe('b')
    state = turnReducer(state, snapshot([op('b', 'Running'), op('a', 'Completed')]))
    expect(state.map((turn) => [turn.id, turn.status])).toEqual([
      ['b', 'Running'],
      ['a', 'Completed'],
    ])
  })

  it('knows a turn is running from the start route before any of its events', () => {
    const state = run(snapshot([op('a', 'Completed')]), { type: 'started', id: 'b' })
    expect(runningTurn(state)?.id).toBe('b')
  })

  it('reads a stop already requested from the snapshot, and ignores events of no operation', () => {
    const state = run(snapshot([op('a', 'Running', '2026-09-23T00:00:01Z')]))
    expect(runningTurn(state)?.stopRequested).toBe(true)
    const noOp: DurableEvent = { seq: 99, kind: 'ThreadEntryAppended', operationId: null, threadId: 't', payload: {} }
    expect(turnReducer(state, { type: 'event', event: noOp, now: 0 })).toBe(state)
  })
})
