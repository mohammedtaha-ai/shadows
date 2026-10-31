import { describe, expect, it } from 'vitest'
import type { ToolFrame } from '@/stream'
import { agentEntry, toolCardEntry, toolCardFixture, toolEntry } from '@/test/contract-fixtures'
import { activityRows, liveSteps } from './activity-rows'

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
