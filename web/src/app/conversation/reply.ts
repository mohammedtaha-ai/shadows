// One job: the reply text shown while a turn streams, and how long it stays.
//
// The streamed text is transient; the reply's durable form is the turn's
// AgentMessage entries, which reach the entries list only after a refetch. The
// stream announces an entry (a durable `ThreadEntryAppended`) before the
// `turn-end` that drops the streamed text, but the refetch it triggers lands
// later, so dropping the reply at turn-end would hide it until the refetch
// lands. The streamed text therefore stays until the entries list
// holds every agent entry it was cut at, or, when it was cut at none
// (a turn stopped mid-sentence), until the entries were fetched after the turn
// ended. While it is shown, the messages it stands in for are hidden, so a
// message is never on screen twice. A tool call or a subagent is never hidden:
// the text is cut where each was announced, so the list draws it between the
// paragraphs it ran between, as it will once the turn has ended.

/** The turn's text up to one announced agent entry. */
export interface Cut {
  /** The announced entry's ordinal: this text came before it. */
  readonly ordinal: number
  /** The entry is an AgentMessage: this text stands in for it. */
  readonly message: boolean
  /** What streamed since the last cut; '' when the entry followed another at once. */
  readonly text: string
}

export interface ReplyState {
  /** The turn whose text this is, or `null` when there is none. */
  readonly op: string | null
  /** The turn's text cut at each agent entry announced so far, in order. */
  readonly cuts: readonly Cut[]
  /** How much of the turn's streamed text `cuts` hold. */
  readonly consumed: number
  /** Text of the message still streaming. */
  readonly tail: string
  /** The stream is still delivering this text. */
  readonly live: boolean
}

export type ReplyAction =
  /** The running turn's streamed text as it stands (`op` null: no turn runs;
   * `text` undefined: its text is gone — the turn ended or the stream broke). */
  | { type: 'stream'; op: string | null; text: string | undefined }
  /** A live `ThreadEntryAppended` for an agent entry (`message`: an
   * AgentMessage), with the stream's text by turn at that instant: the text
   * before the entry ends there. */
  | { type: 'agent-entry'; ordinal: number; message: boolean; streaming: Readonly<Record<string, string>> }

const NO_CUTS: readonly Cut[] = []

export const initialReply: ReplyState = {
  op: null,
  cuts: NO_CUTS,
  consumed: 0,
  tail: '',
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
        cuts: [...state.cuts, { ordinal: action.ordinal, message: action.message, text: tail }],
        consumed: state.consumed + tail.length,
        tail: '',
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
  /** The text cut at each announced agent entry, drawn before that entry.
   * The same array while no entry is announced, so a delta leaves it alone. */
  readonly cuts: readonly Cut[]
  /** The text after the last cut, drawn after the turn's rows. */
  readonly tail: string
  /** Still streaming: draw the cursor. */
  readonly live: boolean
  /** The messages this text stands in for; the list does not show them. */
  readonly hidden: ReadonlySet<number>
}

export function shownReply(state: ReplyState, durable: Durable): ShownReply | null {
  if (state.op === null) return null
  if (!state.live) {
    const covered =
      state.cuts.length > 0 && state.cuts.every((cut) => durable.ordinals.has(cut.ordinal))
    const refetchedAfterEnd = durable.endedAt !== null && durable.fetchedAt >= durable.endedAt
    if (covered || refetchedAfterEnd) return null
  }
  if (state.tail === '' && state.cuts.every((cut) => cut.text === '')) return null
  return {
    cuts: state.cuts,
    tail: state.tail,
    live: state.live,
    hidden: new Set(state.cuts.filter((cut) => cut.message).map((cut) => cut.ordinal)),
  }
}
