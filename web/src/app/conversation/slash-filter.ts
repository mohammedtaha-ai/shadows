// One job: which `/` entries a composer text lists (spec §21.4). `null` when
// the menu does not apply: the whole text must be `/` and no whitespace.

import type { SlashCommand } from '@/stream/frames'

export function slashMatches(
  commands: readonly SlashCommand[],
  text: string,
): readonly SlashCommand[] | null {
  if (!/^\/\S*$/.test(text)) return null
  const typed = text.slice(1).toLowerCase()
  const starts: SlashCommand[] = []
  const afterColon: SlashCommand[] = []
  const contains: SlashCommand[] = []
  for (const command of commands) {
    const name = command.name.toLowerCase()
    if (name.startsWith(typed)) starts.push(command)
    else if (name.slice(name.lastIndexOf(':') + 1).startsWith(typed)) afterColon.push(command)
    else if (name.includes(typed)) contains.push(command)
  }
  return [...starts, ...afterColon, ...contains]
}
