// One job: reading `/model` and `/effort` as the model and effort pickers'
// choices (spec §21.4). Sent to the harness as text they would change its
// session without telling Shadows, and the pickers would show a model the
// session no longer runs, so the composer sets them through the pickers.

import type { Choice, SessionChoices } from '@/api/client'

export type SessionCommand =
  /** Open that picker: the command named no choice. */
  | { kind: 'open'; setting: 'model' | 'effort' }
  /** Set that picker to `id`. */
  | { kind: 'set'; setting: 'model' | 'effort'; id: string }
  /** The text named a choice the session does not offer. */
  | { kind: 'unknown'; message: string }

/** What `text` asks of the pickers, or `null` when it is not `/model` or
 * `/effort`. `choices` is `null` while the session is not ready. */
export function sessionCommand(text: string, choices: SessionChoices | null): SessionCommand | null {
  const found = /^\/(model|effort)(?:\s+(.*))?$/s.exec(text.trim())
  if (found === null) return null
  const setting = found[1] as 'model' | 'effort'
  const asked = (found[2] ?? '').trim()
  if (asked === '') return { kind: 'open', setting }
  if (choices === null) return { kind: 'unknown', message: 'The session is not ready yet.' }
  const id = match(setting === 'model' ? choices.models : choices.efforts, asked)
  if (id === null) {
    return { kind: 'unknown', message: `This session offers no ${setting} named “${asked}”.` }
  }
  return { kind: 'set', setting, id }
}

/** The enabled choice whose id or label is `asked`, else the first whose id
 * or label contains it, ignoring case. */
function match(options: readonly Choice[], asked: string): string | null {
  const want = asked.toLowerCase()
  const enabled = options.filter((o) => o.enabled)
  const named = (o: Choice) => [o.id.toLowerCase(), o.label.toLowerCase()]
  const exact = enabled.find((o) => named(o).includes(want))
  if (exact !== undefined) return exact.id
  return enabled.find((o) => named(o).some((n) => n.includes(want)))?.id ?? null
}
