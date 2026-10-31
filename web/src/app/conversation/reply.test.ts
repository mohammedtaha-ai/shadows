import { describe, expect, it } from 'vitest'
import { type ReplyAction, type ReplyState, type ShownReply, initialReply, replyReducer, shownReply } from './reply'
import { agentOrdinal } from './use-conversation'

describe('agentOrdinal', () => {
  it('moves the reply on for each kind a turn writes (§23.8)', () => {
    for (const kind of ['AgentMessage', 'ToolCall', 'Subagent']) {
      expect(agentOrdinal({ ordinal: 3, kind })).toBe(3)
    }
  })

  it('ignores the person, plans and refusals', () => {
    for (const kind of ['UserMessage', 'PlanView', 'PermissionRefused']) {
      expect(agentOrdinal({ ordinal: 3, kind })).toBeNull()
    }
  })
})

const run = (...actions: ReplyAction[]): ReplyState => actions.reduce(replyReducer, initialReply)
const stream = (text: string | undefined, op: string | null = 'op'): ReplyAction => ({
  type: 'stream',
  op,
  text,
})
const agentEntry = (ordinal: number, streaming: Record<string, string> = {}, message = true): ReplyAction => ({
  type: 'agent-entry',
  ordinal,
  message,
  streaming,
})
/** All the shown text, its parts in order. */
const textOf = (shown: ShownReply | null) =>
  shown === null
    ? undefined
    : [...shown.cuts.map((cut) => cut.text), shown.tail].filter((part) => part !== '').join('\n\n')
const durable = (ordinals: number[], fetchedAt = 0, endedAt: number | null = null) => ({
  ordinals: new Set(ordinals),
  fetchedAt,
  endedAt,
})

describe('the streamed reply', () => {
  it('stays after turn-end until the entries list holds the agent entry', () => {
    // The entry is announced, then turn-end drops the streamed text.
    const state = run(stream('Hel'), stream('Hello'), agentEntry(2), stream(undefined))

    // The refetch has not landed: the reply is still on screen, no cursor,
    // and entry 2 would not be shown twice.
    expect(shownReply(state, durable([1]))).toEqual({
      cuts: [{ ordinal: 2, message: true, text: 'Hello' }],
      tail: '',
      live: false,
      hidden: new Set([2]),
    })
    // It lands: the entry takes over.
    expect(shownReply(state, durable([1, 2]))).toBeNull()
  })

  it('does not give way to the entry while the turn is still streaming', () => {
    const state = run(stream('First.'), agentEntry(2), stream('First.Second'))
    expect(shownReply(state, durable([1, 2]))).toEqual({
      cuts: [{ ordinal: 2, message: true, text: 'First.' }],
      tail: 'Second',
      live: true,
      hidden: new Set([2]),
    })
  })

  it("splits messages where the stream stood when the entry was announced, not at the last render", () => {
    // The last delta of the first message arrived after the text was last
    // seen, but before the entry was announced: it belongs to the first.
    const state = run(stream('First'), agentEntry(2, { op: 'First.' }), stream('First.Second'))
    expect(textOf(shownReply(state, durable([1])))).toBe('First.\n\nSecond')
  })

  it('a turn stopped mid-sentence stays until entries are fetched after it ended', () => {
    const state = run(stream('Partial'), stream(undefined, null))
    expect(textOf(shownReply(state, durable([1], 9, 10)))).toBe('Partial')
    expect(shownReply(state, durable([1], 11, 10))).toBeNull()
  })

  it('a new turn replaces what the last one left', () => {
    const state = run(stream('Old'), stream(undefined), stream('New', 'op2'))
    expect(shownReply(state, durable([1]))).toMatchObject({ cuts: [], tail: 'New', live: true })
  })

  it('cuts the text at a tool call and hides only the messages it stands in for', () => {
    const state = run(
      stream('First.'),
      agentEntry(2, { op: 'First.' }),
      agentEntry(3, { op: 'First.' }, false),
      stream('First.Second'),
    )
    const shown = shownReply(state, durable([1, 2, 3]))
    expect(shown?.cuts).toEqual([
      { ordinal: 2, message: true, text: 'First.' },
      { ordinal: 3, message: false, text: '' },
    ])
    expect(shown?.tail).toBe('Second')
    expect(shown?.hidden).toEqual(new Set([2]))
  })

  it('shows nothing before any text streams', () => {
    expect(shownReply(run(stream(undefined, null)), durable([]))).toBeNull()
  })
})
