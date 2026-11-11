// Distinguish entries by kind rather than text so tool output cannot masquerade as a card.

import type { AttemptNotice } from '@/api/runs'
import type { ThreadEntry } from '@/api/threads'
import { type SubagentCard, readAttemptNotice, readSubagentCard } from '@/stream'

/** Suppress plan_show's tool line because its plan card already represents the result. */
const SENTENCES: Readonly<Record<string, string | null>> = {
  workflow_list: 'Listed the plans',
  workflow_get: 'Read the plan',
  task_get: 'Read a task',
  draft_prepare: 'Prepared a new plan',
  draft_start: 'Plan started',
  plan_edit: 'Plan edited',
  plan_show: null,
  agreement_edit: "Edited the agreement",
  agreement_get: "Read the agreement",
  agreement_list: "Listed the agreements",
  agreement_review: "Reviewed the agreement",
  agreement_start: "Agreement started",
  map_approve: "Approved the map",
  map_propose: "Proposed a map",
  outline: "Read the outline",
  plan_approve: "Approved the plan",
  roadmap_approve: "Approved the roadmap",
  roadmap_propose: "Proposed a roadmap",
  vision_approve: "Approved the vision",
  vision_propose: "Proposed a vision",
  where_is: "Found where it is",
  who_uses: "Found who uses it",
  workspace_get: "Read the workspace",
}

/** How a tool titled `title` reads: a sentence for Shadows' tools, its title
 * for any other, `null` when it adds no line of its own. */
export function toolText(title: string): string | null {
  const name = /^mcp__shadows__(\w+)/.exec(title)?.[1]
  if (name === undefined) return title
  if (!Object.hasOwn(SENTENCES, name)) return `Used Shadows: ${name}`
  return SENTENCES[name] ?? null
}

export function subagentOf(entry: ThreadEntry): SubagentCard | null {
  if (entry.kind !== 'Subagent' || entry.card === null) return null
  try {
    return readSubagentCard(entry.card)
  } catch {
    return null
  }
}

/** An executor's notice: a message whose author is an Executor, never decided from its text.
 * `card` is null when the entry has none or it cannot be read. */
export function noticeOf(entry: ThreadEntry): { card: AttemptNotice | null } | null {
  if (entry.kind !== 'UserMessage' || entry.author.kind !== 'Executor') return null
  return { card: readAttemptNotice(entry.card) }
}

/** An entry that adds no line: a tool call whose result is another entry. */
export function silent(entry: ThreadEntry): boolean {
  return entry.kind === 'ToolCall' && toolText(entry.body) === null
}

/** What an entry's Copy puts on the clipboard: the text as it reads, not its wrapping. A tool
 * call is never an entry row; its group copies its steps' lines. */
export function copyText(entry: ThreadEntry): string {
  const notice = noticeOf(entry)
  if (notice !== null) return notice.card?.report ?? notice.card?.reason ?? entry.body
  const card = subagentOf(entry)
  return card === null ? entry.body : (card.report ?? card.title)
}
