import { describe, expect, it } from 'vitest'
import type { Operation } from '@/api/threads'
import type { DurableEvent } from '@/stream'
import {
  type TurnAction,
  type TurnState,
  initialTurnState,
  latestTurn,
  runningTurn,
  turnReducer,
} from './turn-state'

const STATUS: Record<string, string> = {
  OperationCreated: 'Pending',
  OperationStarted: 'Running',
  OperationCompleted: 'Completed',
  OperationFailed: 'Failed',
  OperationCancelled: 'Cancelled',
  OperationInterrupted: 'Interrupted',
}

// An event as Core sends it: its own seq, the status it leaves, whether a stop was asked.
const at = (
  seq: number,
  kind: string,
  operationId: string,
  payload: unknown,
  now = 0,
): TurnAction => {
  const e: DurableEvent = { seq, kind, operationId, threadId: 't', payload }
  return { type: 'event', event: e, now }
}

const event = (
  seq: number,
  kind: string,
  operationId: string,
  opts: { now?: number; stop?: boolean; status?: string } = {},
): TurnAction =>
  at(
    seq,
    kind,
    operationId,
    {
      status_kind: opts.status ?? STATUS[kind] ?? 'Running',
      cancel_requested: opts.stop ?? false,
    },
    opts.now,
  )

const op = (
  id: string,
  status_kind: string,
  seq: number,
  cancel_requested_at?: string,
): Operation => ({
  id,
  status_kind,
  seq,
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
      snapshot([op('b', 'Running', 9), op('a', 'Completed', 4)]),
      // The replay from the start of the thread arrives after the snapshot.
      event(1, 'OperationCreated', 'a'),
      event(2, 'OperationStarted', 'a'),
      event(4, 'OperationCompleted', 'a'),
      event(5, 'OperationCreated', 'b'),
      event(9, 'OperationStarted', 'b'),
    )
    expect(runningTurn(state)?.id).toBe('b')
    expect(latestTurn(state)?.id).toBe('b')

    state = turnReducer(state, event(10, 'OperationCancellationRequested', 'b', { stop: true }))
    expect(runningTurn(state)).toMatchObject({ id: 'b', stopRequested: true })

    state = turnReducer(state, event(11, 'OperationCancelled', 'b', { now: 42, stop: true }))
    expect(runningTurn(state)).toBeNull()
    expect(latestTurn(state)).toMatchObject({ id: 'b', status: 'Cancelled', endedAt: 42 })
  })

  it('a snapshot read before a turn ended does not bring it back', () => {
    const state = run(
      event(1, 'OperationCreated', 'a'),
      event(2, 'OperationStarted', 'a'),
      event(3, 'OperationCompleted', 'a', { now: 7 }),
      snapshot([op('a', 'Running', 2)], 9),
    )
    expect(runningTurn(state)).toBeNull()
    expect(latestTurn(state)).toMatchObject({ status: 'Completed', endedAt: 7 })
  })

  it('a replayed start does not reopen a turn the snapshot saw end', () => {
    const state = run(
      snapshot([op('a', 'Completed', 3)], 5),
      event(1, 'OperationCreated', 'a'),
      event(2, 'OperationStarted', 'a'),
    )
    expect(runningTurn(state)).toBeNull()
    expect(latestTurn(state)).toMatchObject({ status: 'Completed', endedAt: 5 })
  })

  it('a newer snapshot row replaces an older event', () => {
    const state = run(event(2, 'OperationStarted', 'a'), snapshot([op('a', 'Failed', 6)], 8))
    expect(runningTurn(state)).toBeNull()
    expect(latestTurn(state)).toMatchObject({ status: 'Failed', endedAt: 8 })
  })

  it('an equal seq keeps what is known', () => {
    const state = run(
      event(3, 'OperationCompleted', 'a', { now: 7 }),
      snapshot([op('a', 'Running', 3)], 9),
      event(3, 'OperationStarted', 'a'),
    )
    expect(latestTurn(state)).toMatchObject({ status: 'Completed', endedAt: 7 })
  })

  it('a turn begun after the snapshot is newest, and stays so when the snapshot is read again', () => {
    let state = run(snapshot([op('a', 'Completed', 3)]), event(4, 'OperationCreated', 'b'))
    expect(runningTurn(state)?.id).toBe('b')
    state = turnReducer(state, snapshot([op('b', 'Running', 5), op('a', 'Completed', 3)]))
    expect(state.map((turn) => [turn.id, turn.status])).toEqual([
      ['b', 'Running'],
      ['a', 'Completed'],
    ])
  })

  it('knows a turn is running from the start route before any of its events', () => {
    const state = run(snapshot([op('a', 'Completed', 3)]), { type: 'started', id: 'b' })
    expect(runningTurn(state)?.id).toBe('b')
  })

  it('the start route yields to any row or event', () => {
    const fromEvent = run({ type: 'started', id: 'b' }, event(1, 'OperationCompleted', 'b', { now: 3 }))
    expect(latestTurn(fromEvent)).toMatchObject({ status: 'Completed', endedAt: 3 })
    const fromRow = run({ type: 'started', id: 'b' }, snapshot([op('b', 'Running', 1)]))
    expect(latestTurn(fromRow)).toMatchObject({ status: 'Running', seq: 1 })
    const afterEvent = run(event(1, 'OperationStarted', 'b'), { type: 'started', id: 'b' })
    expect(latestTurn(afterEvent)).toMatchObject({ status: 'Running' })
  })

  it('a cancellation request sets stopRequested without changing the status', () => {
    const state = run(
      event(1, 'OperationStarted', 'a'),
      event(2, 'OperationCancellationRequested', 'a', { stop: true, status: 'Running' }),
    )
    expect(runningTurn(state)).toMatchObject({ status: 'Running', stopRequested: true })
  })

  it('an unknown status keeps the turn unended, so Stop stays offered', () => {
    const state = run(event(1, 'OperationCreated', 'a', { status: 'Suspended' }))
    expect(runningTurn(state)).toMatchObject({ id: 'a', status: 'Pending', endedAt: null })
  })

  it('reads a stop already requested from the snapshot, and ignores events of no operation', () => {
    const state = run(snapshot([op('a', 'Running', 2, '2026-09-23T00:00:01Z')]))
    expect(runningTurn(state)?.stopRequested).toBe(true)
    const noOp: DurableEvent = { seq: 99, kind: 'ThreadEntryAppended', operationId: null, threadId: 't', payload: {} }
    expect(turnReducer(state, { type: 'event', event: noOp, now: 0 })).toBe(state)
  })

  it('ignores an operation event without a status_kind', () => {
    const state = run(snapshot([op('a', 'Running', 2)]))
    expect(turnReducer(state, at(5, 'OperationCompleted', 'a', {}))).toBe(state)
    expect(turnReducer(state, at(6, 'OperationCompleted', 'a', null))).toBe(state)
  })
})
