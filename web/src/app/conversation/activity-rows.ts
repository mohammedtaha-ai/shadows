// One job: turn the shown entries and the running turn's `tool` frames into
// rows — plain entries, and runs of tool calls as steps — so a view draws what
// Core said and works out no status, target or diff of its own.

import type { CallKind, CallStatus, DiffHunk, ThreadEntry } from '@/api/threads'
import { type ToolFrame, readToolCard } from '@/stream'
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
 * entry with no card stands alone: it has nothing to count or time. */
export function activityRows(shown: readonly ThreadEntry[]): Row[] {
  const rows: Row[] = []
  let open: { op: string | null; steps: ToolStep[] } | null = null
  for (const entry of shown) {
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
