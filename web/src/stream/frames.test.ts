import { describe, expect, it } from 'vitest'
import { FrameError, parseCaughtUp, parseDurable } from './frames'

describe('frames', () => {
  it('reads a durable frame with its operation, its thread, and its payload object', () => {
    const event = parseDurable(
      JSON.stringify({
        seq: 7,
        kind: 'OperationStarted',
        operation_id: 'op-1',
        thread_id: 'th-1',
        payload: { kind: 'PlannerTurn' },
      }),
    )
    expect(event).toEqual({
      seq: 7,
      kind: 'OperationStarted',
      operationId: 'op-1',
      threadId: 'th-1',
      payload: { kind: 'PlannerTurn' },
    })
  })

  it('reads an event that names no operation as null, and refuses one missing the field', () => {
    const frame = { seq: 1, kind: 'PlanningThreadCreated', thread_id: 'th-1', payload: {} }
    expect(parseDurable(JSON.stringify({ ...frame, operation_id: null })).operationId).toBeNull()
    expect(() => parseDurable(JSON.stringify(frame))).toThrow(FrameError)
  })

  it('reads caught-up as a JSON object', () => {
    expect(parseCaughtUp('{"seq":12}')).toBe(12)
    expect(() => parseCaughtUp('12')).toThrow(FrameError)
  })
})
