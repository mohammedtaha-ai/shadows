import { describe, expect, it } from 'vitest'
import { type ReplyAction, type ReplyState, initialReply, replyReducer, shownReply } from './reply'
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
const agentEntry = (ordinal: number, streaming: Record<string, string> = {}): ReplyAction => ({
  type: 'agent-entry',
  ordinal,
  streaming,
})
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
      text: 'Hello',
      live: false,
      hidden: new Set([2]),
    })
    // It lands: the entry takes over.
    expect(shownReply(state, durable([1, 2]))).toBeNull()
  })

  it('does not give way to the entry while the turn is still streaming', () => {
    const state = run(stream('First.'), agentEntry(2), stream('First.Second'))
    expect(shownReply(state, durable([1, 2]))).toEqual({
      text: 'First.\n\nSecond',
      live: true,
      hidden: new Set([2]),
    })
  })

  it("splits messages where the stream stood when the entry was announced, not at the last render", () => {
    // The last delta of the first message arrived after the text was last
    // seen, but before the entry was announced: it belongs to the first.
    const state = run(stream('First'), agentEntry(2, { op: 'First.' }), stream('First.Second'))
    expect(shownReply(state, durable([1]))?.text).toBe('First.\n\nSecond')
  })

  it('a turn stopped mid-sentence stays until entries are fetched after it ended', () => {
    const state = run(stream('Partial'), stream(undefined, null))
    expect(shownReply(state, durable([1], 9, 10))?.text).toBe('Partial')
    expect(shownReply(state, durable([1], 11, 10))).toBeNull()
  })

  it('a new turn replaces what the last one left', () => {
    const state = run(stream('Old'), stream(undefined), stream('New', 'op2'))
    expect(shownReply(state, durable([1]))).toMatchObject({ text: 'New', live: true })
  })

  it('shows nothing before any text streams', () => {
    expect(shownReply(run(stream(undefined, null)), durable([]))).toBeNull()
  })
})
