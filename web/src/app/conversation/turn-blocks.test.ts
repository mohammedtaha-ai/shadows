import { describe, expect, it } from 'vitest'
import type { ThreadEntry } from '@/api/threads'
import { agentEntry, entryOfKind, toolEntry, userEntry } from '@/test/contract-fixtures'
import type { Row, ToolStep } from './activity-rows'
import { type Block, turnBlocks } from './turn-blocks'

const entryRow = (entry: ThreadEntry): Row => ({ kind: 'entry', entry })

function toolRow(entry: ThreadEntry): Row {
  const step: ToolStep = {
    key: entry.id, ordinal: entry.ordinal, tool: null, kind: null, title: entry.body, sentence: null,
    target: null, lines: null, status: 'completed', added: null, removed: null, diff: null,
    error: null, startedAt: null, finishedAt: null,
  }
  return { kind: 'tools', key: entry.id, steps: [step] }
}

const notice = (id: string): ThreadEntry => ({
  ...entryOfKind(id, 'UserMessage', 'instructions'),
  author: { kind: 'Executor', id: 'x1' },
})

const shape = (blocks: Block[]) =>
  blocks.map((b) => (b.kind === 'row' ? (b.row.kind === 'entry' ? b.row.entry.id : b.row.kind) : `turn(${b.inside.length})`))

describe('turnBlocks', () => {
  it('a_finished_turn_folds_its_notice_tools_and_text_and_keeps_its_final_answer', () => {
    const n = notice('n1')
    const note = agentEntry('a1', 'Let me look.')
    const t1 = toolEntry('t1', 'Read a.ts')
    const t2 = toolEntry('t2', 'Read b.ts')
    const final = agentEntry('a2', 'All done.')
    const entries = [n, note, t1, t2, final]
    const rows = [entryRow(n), entryRow(note), toolRow(t1), toolRow(t2), entryRow(final)]
    const blocks = turnBlocks(rows, null, entries)
    expect(shape(blocks)).toEqual(['turn(4)', 'a2'])
    const box = blocks[0]
    expect(box?.kind === 'turn' && box.summary).toEqual({ notices: [null], tools: 2, notes: 1 })
  })

  it('a_turn_with_no_activity_has_no_box', () => {
    const u = userEntry('u1', 'hello')
    const a = agentEntry('a1', 'hi')
    expect(shape(turnBlocks([entryRow(u), entryRow(a)], null, [u, a]))).toEqual(['u1', 'a1'])
  })

  it('a_proposal_card_stays_outside_the_box', () => {
    const t = toolEntry('t1', 'Read a.ts')
    const vision = entryOfKind('v1', 'VisionProposal', 'a vision')
    const answer = agentEntry('a1', 'Here it is.')
    const blocks = turnBlocks([toolRow(t), entryRow(vision), entryRow(answer)], null, [t, vision, answer])
    expect(shape(blocks)).toEqual(['turn(1)', 'v1', 'a1'])
  })

  it('the_person_prompt_stays_above_the_box', () => {
    const u = userEntry('u1', 'do it')
    const t = toolEntry('t1', 'Read a.ts')
    const blocks = turnBlocks([entryRow(u), toolRow(t)], null, [u, t])
    expect(shape(blocks)).toEqual(['u1', 'turn(1)'])
  })
})
