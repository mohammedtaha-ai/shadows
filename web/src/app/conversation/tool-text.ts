// Distinguish entries by kind rather than text so tool output cannot masquerade as a card.

import type { ThreadEntry } from '@/api/threads'
import { type SubagentCard, readSubagentCard } from '@/stream'

/** Suppress plan_show's tool line because its plan card already represents the result. */
const SENTENCES: Readonly<Record<string, string | null>> = {
  workflow_list: 'Listed the plans',
  workflow_get: 'Read the plan',
  task_get: 'Read a task',
  draft_prepare: 'Prepared a new plan',
  draft_start: 'Plan started',
  plan_edit: 'Plan edited',
  plan_show: null,
}

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
