import { describe, expect, it } from 'vitest'
import type { ToolFrame } from '@/stream'
import { agentEntry, toolCardEntry, toolCardFixture, toolEntry } from '@/test/contract-fixtures'
import { activityRows, currentTodos, liveSteps, stepsOf, withLiveSteps } from './activity-rows'

const call = (id: string, op = 'op1') =>
  toolCardEntry(`e-${id}`, `Read ${id}`, toolCardFixture({ id, target: id }), op)

const frame = (id: string, extra: Partial<ToolFrame> = {}): ToolFrame => ({
  op: 'op1',
  id,
  tool: 'Read',
  kind: 'read',
  title: `Read ${id}`,
  target: id,
  lines: null,
  status: 'in_progress',
  startedAt: '2026-10-31T10:00:00Z',
  ...extra,
})

describe('activity rows', () => {
  it('puts streamed text before the entry it was cut at, splitting the group there', () => {
    const a = call('a')
    const b = call('b')
    const rows = activityRows([a, b], [
      { ordinal: a.ordinal - 1, text: 'First.' },
      { ordinal: b.ordinal, text: 'Then.' },
      { ordinal: b.ordinal + 1, text: 'Last.' },
    ])
    expect(rows.map((row) => (row.kind === 'text' ? row.text : row.kind === 'tools' ? row.steps.map((s) => s.key) : row.kind)))
      .toEqual(['First.', ['e-a'], 'Then.', ['e-b'], 'Last.'])
  })

  it('groups consecutive tool calls of one turn, and splits at another kind or another turn', () => {
    const rows = activityRows([
      call('a'),
      call('b'),
      agentEntry('m1', 'hello'),
      call('c'),
      call('d', 'op2'),
    ])
    expect(rows.map((r) => (r.kind === 'tools' ? r.steps.map((s) => s.target) : 'entry'))).toEqual([
      ['a', 'b'],
      'entry',
      ['c'],
      ['d'],
    ])
  })

  it('an entry with no card is a completed step with its title', () => {
    const [row] = activityRows([toolEntry('t1', 'npm test')])
    expect(row?.kind === 'tools' && row.steps[0]).toMatchObject({
      kind: null,
      status: 'completed',
      title: 'npm test',
      sentence: null,
    })
  })

  it('a live frame already written as an entry is not a live step, whatever the frame says', () => {
    const steps = liveSteps([frame('a', { status: 'in_progress' }), frame('b')], [call('a')], 'op1')
    expect(steps.map((s) => s.key)).toEqual(['live-b'])
  })

  it('a completed call is drawn from its last frame until its entry lands', () => {
    const steps = liveSteps([frame('a', { status: 'completed' })], [], 'op1')
    expect(steps).toHaveLength(1)
    expect(steps[0]).toMatchObject({ status: 'completed', ordinal: null, error: null })
    expect(liveSteps([frame('a', { status: 'pending' })], [], 'op1', true)[0]?.status).toBe(
      'stopped',
    )
  })
})

describe('the rows a view shares', () => {
  it('joins the unwritten calls of the turn to its last group, or starts one', () => {
    const a = call('a')
    const written = activityRows([a])
    const live = liveSteps([frame('b')], [a], 'op1')
    const joined = withLiveSteps(written, live, [a], 'op1')
    expect(stepsOf(joined).map((s) => s.key)).toEqual(['e-a', 'live-b'])
    expect(joined).toHaveLength(1)
    const after = [a, agentEntry('m', 'done')]
    const split = withLiveSteps(activityRows(after), live, after, 'op1')
    expect(split.map((r) => r.kind)).toEqual(['tools', 'entry', 'tools'])
  })

  it('takes the live task list while it has items, else the last written one', () => {
    const written = { ...agentEntry('t', 'x'), kind: 'TodoList' as const, card: { entries: [{ content: 'Old', status: 'pending' }] } }
    const live = { op1: [{ content: 'New', status: 'in_progress' as const }] }
    expect(currentTodos(live, [written]).map((i) => i.content)).toEqual(['New'])
    expect(currentTodos({ op1: [] }, [written]).map((i) => i.content)).toEqual(['Old'])
    expect(currentTodos({}, [])).toEqual([])
  })
})
