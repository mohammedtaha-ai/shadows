// One job: turn the shown entries and the running turn's `tool` frames into
// rows — plain entries, and runs of tool calls as steps — so a view draws what
// Core said and works out no status, target or diff of its own.

import type { CallKind, CallStatus, DiffHunk, ThreadEntry } from '@/api/threads'
import { type TodoItem, type ToolFrame, readTodoList, readToolCard } from '@/stream'
import { toolText } from './tool-text'

export type ToolStatus = CallStatus

/** One tool call as the conversation draws it. */
export interface ToolStep {
  /** The entry's id, or `live-<call id>` for a call that has no entry yet. */
  key: string
  /** The entry's ordinal; `null` for a live step. */
  ordinal: number | null
  tool: string | null
  /** `null` for an entry with no card (an old one): it has only its title. */
  kind: CallKind | null
  title: string
  /** A Shadows tool read as a sentence; `null` for any other tool. */
  sentence: string | null
  target: string | null
  lines: readonly number[] | null
  status: ToolStatus
  added: number | null
  removed: number | null
  diff: readonly DiffHunk[] | null
  error: string | null
  startedAt: string | null
  finishedAt: string | null
}

export type Row =
  | { kind: 'entry'; entry: ThreadEntry }
  | { kind: 'tools'; key: string; steps: ToolStep[] }
  /** Streamed text of the running turn not yet in the list as its message. */
  | { kind: 'text'; key: string; text: string }

/** Streamed text that came before the entry at `ordinal`. */
export interface TextPart {
  ordinal: number
  text: string
}

function sentenceOf(title: string): string | null {
  const text = toolText(title)
  return text === title ? null : text
}

function durableStep(entry: ThreadEntry): ToolStep {
  const card = readToolCard(entry.card)
  if (card === null) {
    return {
      key: entry.id,
      ordinal: entry.ordinal,
      tool: null,
      kind: null,
      title: entry.body,
      sentence: sentenceOf(entry.body),
      target: null,
      lines: null,
      status: 'completed',
      added: null,
      removed: null,
      diff: null,
      error: null,
      startedAt: null,
      finishedAt: null,
    }
  }
  return {
    key: entry.id,
    ordinal: entry.ordinal,
    tool: card.tool ?? null,
    kind: card.kind,
    title: entry.body,
    sentence: sentenceOf(entry.body),
    target: card.target ?? null,
    lines: card.lines ?? null,
    status: card.status,
    added: card.added ?? null,
    removed: card.removed ?? null,
    diff: card.diff ?? null,
    error: card.error ?? null,
    startedAt: card.started_at,
    finishedAt: card.finished_at,
  }
}

/** Consecutive tool calls of one turn that carry a card form one group. An
 * entry with no card stands alone: it has nothing to count or time. A text
 * part (in ordinal order) goes before the first entry at or after its
 * `ordinal`, and ends a group as a message entry would. */
export function activityRows(shown: readonly ThreadEntry[], texts: readonly TextPart[] = []): Row[] {
  const rows: Row[] = []
  let open: { op: string | null; steps: ToolStep[] } | null = null
  let next = 0
  const textsUpTo = (ordinal: number) => {
    for (; next < texts.length && (texts[next]?.ordinal ?? Infinity) <= ordinal; next += 1) {
      const part = texts[next]
      if (part === undefined || part.text === '') continue
      open = null
      rows.push({ kind: 'text', key: `reply-${part.ordinal}`, text: part.text })
    }
  }
  for (const entry of shown) {
    textsUpTo(entry.ordinal)
    if (entry.kind !== 'ToolCall') {
      open = null
      rows.push({ kind: 'entry', entry })
      continue
    }
    const step = durableStep(entry)
    if (step.kind === null) {
      open = null
      rows.push({ kind: 'tools', key: step.key, steps: [step] })
      continue
    }
    if (open !== null && open.op === entry.operation_id) {
      open.steps.push(step)
      continue
    }
    const steps = [step]
    open = { op: entry.operation_id, steps }
    rows.push({ kind: 'tools', key: step.key, steps })
  }
  textsUpTo(Infinity)
  return rows
}

/** The turn `op`'s calls that no entry has taken over. A durable entry is
 * final over every frame of its call; a frame can repeat or lag. Once the turn
 * `ended`, a call still unended is shown as stopped, as Core will write it. */
export function liveSteps(
  frames: readonly ToolFrame[],
  entries: readonly ThreadEntry[],
  op: string,
  ended = false,
): ToolStep[] {
  const written = new Set<string>()
  for (const entry of entries) {
    if (entry.kind !== 'ToolCall') continue
    const card = readToolCard(entry.card)
    if (card !== null) written.add(card.id)
  }
  return frames
    .filter((frame) => frame.op === op && !written.has(frame.id))
    .map((frame) => ({
      key: `live-${frame.id}`,
      ordinal: null,
      tool: frame.tool,
      kind: frame.kind,
      title: frame.title,
      sentence: sentenceOf(frame.title),
      target: frame.target,
      lines: frame.lines,
      status:
        ended && (frame.status === 'pending' || frame.status === 'in_progress')
          ? 'stopped'
          : frame.status,
      added: null,
      removed: null,
      diff: null,
      error: null,
      startedAt: frame.startedAt,
      finishedAt: null,
    }))
}

/** The written rows with the turn's unwritten calls on the end. They are the
 * turn's newest: they join its last group when nothing came after it, else
 * form one there, before the text still streaming (Core cuts the message at
 * each call). */
export function withLiveSteps(
  written: readonly Row[],
  live: readonly ToolStep[],
  shown: readonly ThreadEntry[],
  turnOp: string | undefined,
): Row[] {
  if (live.length === 0) return [...written]
  const last = written.at(-1)
  const lastKey = last?.kind === 'tools' ? last.steps.at(-1)?.key : undefined
  const lastOp = shown.find((entry) => entry.id === lastKey)?.operation_id
  if (last?.kind === 'tools' && lastOp === turnOp && !last.steps.some((step) => step.kind === null)) {
    return [...written.slice(0, -1), { ...last, steps: [...last.steps, ...live] }]
  }
  return [...written, { kind: 'tools', key: 'live-tools', steps: [...live] }]
}

/** Every tool step the rows hold, in order. */
export function stepsOf(rows: readonly Row[]): ToolStep[] {
  return rows.flatMap((row) => (row.kind === 'tools' ? row.steps : []))
}

/** The running turn's newest task list while it has items, else the last one
 * an entry holds. */
export function currentTodos(
  live: Readonly<Record<string, readonly TodoItem[]>>,
  entries: readonly ThreadEntry[],
): readonly TodoItem[] {
  const streamed = Object.values(live).at(-1)
  if (streamed !== undefined && streamed.length > 0) return streamed
  for (const entry of entries.toReversed()) {
    if (entry.kind !== 'TodoList') continue
    const card = readTodoList(entry.card)
    if (card !== null && card.entries.length > 0) return card.entries
  }
  return []
}
