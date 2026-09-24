// One job: which model, mode and effort the next turn runs with, given what
// the harness session offers (spec §12.4). Pure: the composer holds the
// value, and every rule about how it follows the session is here.
//
// The session's choices carry the efforts of the model it holds now, and
// nothing about another's. So after the person picks another model, its
// efforts are unknown until the session reports that model: the composer sets
// it on the session at once (spec §12.7) and its answer carries them; until
// then the effort menu is disabled, the effort stays and Send waits.

import type { Choice, SessionChoices, TurnSettings } from '@/api/client'

/** The session's current values, except a mode that is disabled moves to the
 * first enabled one. */
export function initialSettings(c: SessionChoices): TurnSettings {
  return { ...c.current, mode: usableMode(c, c.current.mode) }
}

/** The settings after the person picks `model`. */
export function withModel(c: SessionChoices, s: TurnSettings, model: string): TurnSettings {
  if (!effortsKnown(c, model)) return { ...s, model }
  return { ...s, model, effort: offeredEffort(c, s.effort) }
}

/** Whether the session's efforts are those of `model`. */
export function effortsKnown(c: SessionChoices, model: string): boolean {
  return c.current.model === model
}

/** The line that says a model change moved the mode (spec §12.4), or `null`
 * when `next` reports the mode the person had. */
export function modeNote(prev: TurnSettings, next: SessionChoices): string | null {
  if (next.current.mode === prev.mode) return null
  const model = labelOf(next.models, next.current.model)
  const from = labelOf(next.modes, prev.mode)
  const to = labelOf(next.modes, next.current.mode)
  return `${model} does not offer ${from}; switched to ${to}`
}

/** A turn can start with `s`: its mode is offered and enabled. False when no
 * mode is enabled at all. */
export function sendable(c: SessionChoices, s: TurnSettings): boolean {
  return c.modes.some((m) => m.id === s.mode && m.enabled)
}

/** The settings once the session's choices changed from `prev` to `next`,
 * and the note to show, if any.
 *
 * When the session now holds the model the person chose, the effort moves to
 * one that model offers. When that model is new to the session and it reports
 * another mode than the person's, the model moved the mode: the settings take
 * the session's and the note says so. `busy` (a send or a turn in flight)
 * holds the mode back, because a turn start sets the model before the mode,
 * and the report between the two is not a refusal. */
export function afterOptions(
  prev: SessionChoices,
  next: SessionChoices,
  s: TurnSettings,
  busy: boolean,
): { settings: TurnSettings; note: string | null } {
  let settings = s
  let note: string | null = null
  if (next.current.model === s.model) {
    settings = { ...settings, effort: offeredEffort(next, s.effort) }
    const modelMoved = prev.current.model !== next.current.model
    if (modelMoved && !busy) {
      note = modeNote(s, next)
      if (note !== null) settings = { ...settings, mode: next.current.mode }
    }
  }
  return { settings: { ...settings, mode: usableMode(next, settings.mode) }, note }
}

/** `effort` if the session's model offers it, else the first it offers;
 * `null` when it offers none. */
function offeredEffort(c: SessionChoices, effort: string | null): string | null {
  const enabled = c.efforts.filter((e) => e.enabled)
  if (enabled.some((e) => e.id === effort)) return effort
  return enabled[0]?.id ?? null
}

/** `mode` if enabled, else the first enabled mode; `mode` itself when none is,
 * so the menu still names it and Send stays off. */
function usableMode(c: SessionChoices, mode: string): string {
  if (c.modes.some((m) => m.id === mode && m.enabled)) return mode
  return c.modes.find((m) => m.enabled)?.id ?? mode
}

export function labelOf(choices: readonly Choice[], id: string | null): string {
  if (id === null) return 'None'
  return choices.find((c) => c.id === id)?.label ?? id
}
