// One job: how a tool call reads in the conversation — its title, or for
// Shadows' own tools a sentence (§13.11), a subagent's card (§22), or no line
// at all. Entries are told apart by their kind (§23.8), never by their text.

import type { ThreadEntry } from '@/api/client'
import { type SubagentCard, readSubagentCard } from '@/stream/frames'

/** What each of Shadows' tools (`mcp__shadows__<name>`, §13.6) did, said as a
 * person would. `plan_show` says nothing: the card it wrote is the entry. */
const SENTENCES: Readonly<Record<string, string | null>> = {
  workflow_list: 'Listed the plans',
  workflow_get: 'Read the plan',
  task_get: 'Read a task',
  draft_prepare: 'Prepared a new plan',
  draft_start: 'Plan started',
  plan_edit: 'Plan edited',
  plan_show: null,
}

/** A tool call is a `ToolCall` entry whose body is its title (§23.8); its
 * title, or `null` for any other entry. */
export function toolTitle(entry: ThreadEntry): string | null {
  return entry.kind === 'ToolCall' ? entry.body : null
}

/** How a tool titled `title` reads: a sentence for Shadows' tools, its title
 * for any other, `null` when it adds no line of its own. */
export function toolText(title: string): string | null {
  const name = /^mcp__shadows__(\w+)/.exec(title)?.[1]
  if (name === undefined || !(name in SENTENCES)) return title
  return SENTENCES[name] ?? null
}

/** A subagent is a `Subagent` entry carrying its card (§22.2, §23.8); its
 * card, or `null` for any other entry or a card this client cannot read. */
export function subagentOf(entry: ThreadEntry): SubagentCard | null {
  if (entry.kind !== 'Subagent' || entry.card === null) return null
  try {
    return readSubagentCard(entry.card)
  } catch {
    return null
  }
}

export const isTool = (entry: ThreadEntry) => entry.kind === 'ToolCall'

/** An entry that adds no line: a tool call whose result is another entry. */
export function silent(entry: ThreadEntry): boolean {
  const tool = toolTitle(entry)
  return tool !== null && toolText(tool) === null
}

/** What Copy puts on the clipboard: the text as it reads, not its wrapping. */
export function copyText(entry: ThreadEntry): string {
  const card = subagentOf(entry)
  if (card !== null) return card.report ?? card.title
  const tool = toolTitle(entry)
  return tool === null ? entry.body : (toolText(tool) ?? tool)
}
