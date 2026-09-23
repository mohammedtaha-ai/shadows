// One job: the reply text shown while a turn streams, and how long it stays.
//
// The streamed text is transient; the reply's durable form is the turn's
// AgentMessage entries, which reach the entries list only after a refetch. The
// stream announces an entry (a durable `ThreadEntryAppended`) before the
// `turn-end` that drops the streamed text, but the refetch it triggers lands
// later — so showing the stream only while it streams made the reply blink out
// between the two. The streamed text therefore stays until the entries list
// holds every agent entry it stands in for, or, when it stands in for none
// (a turn stopped mid-sentence), until the entries were fetched after the turn
// ended. While it is shown, the entries it stands in for are hidden, so a
// message is never on screen twice.

export interface ReplyState {
  /** The turn whose text this is, or `null` when there is none. */
  readonly op: string | null
  /** Messages of this turn already announced as agent entries. */
  readonly segments: readonly string[]
  /** How much of the turn's streamed text `segments` hold. */
  readonly consumed: number
  /** Text of the message still streaming. */
  readonly tail: string
  /** Ordinals of the agent entries `segments` stand in for. */
  readonly covers: readonly number[]
  /** The stream is still delivering this text. */
  readonly live: boolean
}

export type ReplyAction =
  /** The running turn's streamed text as it stands (`op` null: no turn runs;
   * `text` undefined: its text is gone — the turn ended or the stream broke). */
  | { type: 'stream'; op: string | null; text: string | undefined }
  /** A live `ThreadEntryAppended` for an AgentMessage, with the stream's text
   * by turn at that instant: the message it announces ends there. */
  | { type: 'agent-entry'; ordinal: number; streaming: Readonly<Record<string, string>> }

export const initialReply: ReplyState = {
  op: null,
  segments: [],
  consumed: 0,
  tail: '',
  covers: [],
  live: false,
}

export function replyReducer(state: ReplyState, action: ReplyAction): ReplyState {
  switch (action.type) {
    case 'stream': {
      const { op, text } = action
      if (op === null || text === undefined || text === '') {
        return state.live ? { ...state, live: false } : state
      }
      // A different turn, or the same one streaming again after a break (the
      // text before the break is never resent): start over.
      if (op !== state.op || !state.live) {
        return { ...initialReply, op, tail: text, live: true }
      }
      return { ...state, tail: text.slice(state.consumed) }
    }
    case 'agent-entry': {
      if (state.op === null || !state.live) return state
      const streamed = action.streaming[state.op]
      const tail = streamed === undefined ? state.tail : streamed.slice(state.consumed)
      return {
        ...state,
        segments: tail === '' ? state.segments : [...state.segments, tail],
        consumed: state.consumed + tail.length,
        tail: '',
        covers: [...state.covers, action.ordinal],
      }
    }
  }
}

/** What the entries list currently holds, and what is known about the turn. */
export interface Durable {
  /** Ordinals in the fetched entries list. */
  readonly ordinals: ReadonlySet<number>
  /** When that list was fetched (ms). */
  readonly fetchedAt: number
  /** When the reply's turn was learned to have ended (ms), if it has. */
  readonly endedAt: number | null
}

export interface ShownReply {
  readonly text: string
  /** Still streaming: draw the cursor. */
  readonly live: boolean
  /** Entries this text stands in for; the list does not show them. */
  readonly hidden: ReadonlySet<number>
}

export function shownReply(state: ReplyState, durable: Durable): ShownReply | null {
  if (state.op === null) return null
  if (!state.live) {
    const covered =
      state.covers.length > 0 && state.covers.every((ordinal) => durable.ordinals.has(ordinal))
    const refetchedAfterEnd = durable.endedAt !== null && durable.fetchedAt >= durable.endedAt
    if (covered || refetchedAfterEnd) return null
  }
  const text = [...state.segments, state.tail].filter((part) => part !== '').join('\n\n')
  if (text === '') return null
  return { text, live: state.live, hidden: new Set(state.covers) }
}
