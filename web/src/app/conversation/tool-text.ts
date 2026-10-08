// One job: how a tool call reads in the conversation — its title, or for
// Shadows' own tools a sentence (§13.11), a subagent's card (§22), or no line
// at all.

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

/** A tool call arrives as an agent message whose body is `[tool: <title>]`
 * (spec §12.3); its title, or `null` for any other body. */
export function toolTitle(body: string): string | null {
  return /^\[tool: ([\s\S]*)\]$/.exec(body)?.[1] ?? null
}

/** How a tool titled `title` reads: a sentence for Shadows' tools, its title
 * for any other, `null` when it adds no line of its own. */
export function toolText(title: string): string | null {
  const name = /^mcp__shadows__(\w+)/.exec(title)?.[1]
  if (name === undefined || !(name in SENTENCES)) return title
  return SENTENCES[name] ?? null
}

/** A subagent arrives as an agent message whose body is
 * `[subagent: <card JSON>]` (spec §22.2); its card, or `null` for any other. */
export function subagentOf(body: string): SubagentCard | null {
  const json = /^\[subagent: ([\s\S]*)\]$/.exec(body)?.[1]
  if (json === undefined) return null
  try {
    return readSubagentCard(JSON.parse(json))
  } catch {
    return null
  }
}

export const isTool = (entry: ThreadEntry) =>
  entry.kind === 'AgentMessage' && toolTitle(entry.body) !== null

/** An entry that adds no line: a tool call whose result is another entry. */
export function silent(entry: ThreadEntry): boolean {
  return isTool(entry) && toolText(toolTitle(entry.body) ?? '') === null
}

/** What Copy puts on the clipboard: the text as it reads, not its wrapping. */
export function copyText(entry: ThreadEntry): string {
  const card = subagentOf(entry.body)
  if (card !== null) return card.report ?? card.title
  const tool = toolTitle(entry.body)
  return tool === null ? entry.body : (toolText(tool) ?? tool)
}
