import { describe, expect, it } from 'vitest'
import { fakeChoices } from '@/test/contract-fixtures'
import {
  FrameError,
  parseCaughtUp,
  parseCommands,
  parseDurable,
  parseOptions,
  parsePlanShow,
  parseUsage,
} from './frames'

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

  it('parses a usage frame with limits', () => {
    const u = parseUsage(
      '{"thread_id":"t1","context_used":1234,"context_window":200000,"limits":{"five_hour":{"utilization":0.25,"resets_at":1790212200},"seven_day":null,"observed_at":"2026-09-24T02:49:00Z"}}',
    )
    expect(u.contextUsed).toBe(1234)
    expect(u.limits?.fiveHour?.utilization).toBe(0.25)
    expect(u.limits?.sevenDay).toBeNull()
  })

  it('reads a usage frame that reported nothing as nulls', () => {
    const u = parseUsage('{"thread_id":"t1","context_used":null,"context_window":null,"limits":null}')
    expect(u).toEqual({ threadId: 't1', contextUsed: null, contextWindow: null, limits: null })
  })

  it('refuses a usage frame without thread_id', () => {
    expect(() => parseUsage('{"context_used":1,"context_window":2,"limits":null}')).toThrow(
      FrameError,
    )
  })

  it('parses an options frame', () => {
    const o = parseOptions(JSON.stringify({ thread_id: 't1', choices: fakeChoices }))
    expect(o.choices.models.map((m) => m.id)).toEqual(['fake-small', 'fake-large'])
  })

  it('refuses an options frame whose choices have no current settings', () => {
    const { models, efforts, modes } = fakeChoices
    const choices = { models, efforts, modes }
    expect(() => parseOptions(JSON.stringify({ thread_id: 't1', choices }))).toThrow(FrameError)
  })

  it('reads a commands frame with and without a hint', () => {
    const commands = [
      { name: 'compact', description: 'Clear history', hint: null },
      { name: 'superpowers:brainstorming', description: 'Explore', hint: '[topic]' },
    ]
    expect(parseCommands(JSON.stringify({ thread_id: 't1', commands }))).toEqual({
      threadId: 't1',
      commands,
    })
  })

  it('refuses a commands entry without a name', () => {
    const data = JSON.stringify({ thread_id: 't1', commands: [{ description: 'x', hint: null }] })
    expect(() => parseCommands(data)).toThrow(FrameError)
  })

  it('reads a plan-show frame, and refuses a place it does not know', () => {
    const frame = {
      thread_id: 't1',
      target_tab: 'tab-1',
      workflow_id: 'w1',
      version: 2,
      task_number: 4,
      place: 'side',
    }
    expect(parsePlanShow(JSON.stringify(frame))).toEqual({
      threadId: 't1',
      targetTab: 'tab-1',
      workflowId: 'w1',
      version: 2,
      taskNumber: 4,
      place: 'side',
    })
    const untargeted = parsePlanShow(
      JSON.stringify({ ...frame, target_tab: null, task_number: null }),
    )
    expect(untargeted.targetTab).toBeNull()
    expect(untargeted.taskNumber).toBeNull()
    expect(() => parsePlanShow(JSON.stringify({ ...frame, place: 'window' }))).toThrow(FrameError)
  })
})
