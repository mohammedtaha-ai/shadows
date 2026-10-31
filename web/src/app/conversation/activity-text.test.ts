import { describe, expect, it } from 'vitest'
import type { CallKind } from '@/api/threads'
import type { ToolStep } from './activity-rows'
import { groupSummary, stepLine } from './activity-text'

const step = (kind: CallKind | null, extra: Partial<ToolStep> = {}): ToolStep => ({
  key: 'k',
  ordinal: 1,
  tool: null,
  kind,
  title: 'title',
  sentence: null,
  target: null,
  lines: null,
  status: 'completed',
  added: null,
  removed: null,
  diff: null,
  error: null,
  startedAt: null,
  finishedAt: null,
  ...extra,
})

describe('activity text', () => {
  it.each<[string, ToolStep[], string]>([
    ['read', [step('read', { target: 'a' })], 'Read a file'],
    [
      'read twice',
      [step('read', { target: 'a' }), step('read', { target: 'b' }), step('read', { target: 'a' })],
      'Read 2 files',
    ],
    ['edit', [step('edit', { target: 'a' })], 'Edited a file'],
    ['delete', [step('delete'), step('delete')], 'Deleted 2 files'],
    ['move', [step('move')], 'Moved a file'],
    ['search', [step('search'), step('search'), step('search')], 'Searched 3 times'],
    ['search once', [step('search')], 'Searched once'],
    ['execute', [step('execute'), step('execute')], 'Ran 2 commands'],
    ['fetch', [step('fetch')], 'Fetched a page'],
    ['think', [step('think'), step('think')], 'Thought'],
    ['switch_mode', [step('switch_mode')], 'Switched mode'],
    ['other', [step('other'), step(null)], 'Used 2 tools'],
    ['order', [step('execute'), step('read'), step('execute')], 'Ran 2 commands, read a file'],
  ])('words %s', (_name, steps, text) => {
    expect(groupSummary(steps)).toBe(text)
  })

  it("an edit's counts are summed", () => {
    expect(
      groupSummary([
        step('read', { target: 'a' }),
        step('edit', { target: 'x', added: 30, removed: 1 }),
        step('edit', { target: 'y', added: 6, removed: 2 }),
      ]),
    ).toBe('Read a file, edited 2 files +36 −3')
  })

  it('a sentence wins over the verb', () => {
    expect(stepLine(step('read', { sentence: 'Read the plan', target: 'a.ts' })).text).toBe(
      'Read the plan',
    )
    expect(stepLine(step('edit', { target: 'a.ts', added: 4 }))).toEqual({
      text: 'Edit a.ts',
      counts: '+4',
    })
    expect(stepLine(step('execute', { title: 'npm test' })).text).toBe('npm test')
  })

  it('lines read as (47–102)', () => {
    expect(stepLine(step('read', { target: 'a.ts', lines: [47, 102] })).text).toBe(
      'Read a.ts (47–102)',
    )
  })
})
