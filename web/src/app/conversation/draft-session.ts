// Offer the draft's choices before a session exists.

import type { Choice, SessionChoices } from '@/api/sessions'
import type { Provider } from '@/api/aylul-profiles'
import { policyOf } from '../mode-policy'
import type { SessionView } from './use-session'

/** What a draft's model menu reads: no model is chosen before a session
 * exists, and Send takes the one the session opens at. */
const UNOPENED: Choice = {
  id: 'unopened',
  label: 'Set on send',
  description: null,
  enabled: true,
  reason: null,
}

/** What a draft's composer offers before a session exists: the harness's
 * modes as the project allows them, no model chosen and no effort. */
export function draftSession(harness: string, allowed: string[] | undefined, provider?: Provider, defaultModel?: string): SessionView {
  if (allowed === undefined) return { state: 'connecting' }
  const policy = policyOf(harness)
  const choices: SessionChoices = {
    // Drafts stage files locally; the opened session and Core judge the first send.
    capabilities: { image: true, image_reason: null },
    current: {
      mode: policy.initial ?? policy.modes[0]?.id ?? '',
      model: provider === undefined ? UNOPENED.id : defaultModel ?? provider.models[0]!.id,
      effort: null,
    },
    modes: policy.modes.map((m) => {
      const enabled = allowed.includes(m.id)
      return {
        id: m.id,
        label: m.label,
        description: null,
        enabled,
        reason: enabled ? null : 'Not allowed in this project',
      }
    }),
    models: provider === undefined ? [UNOPENED] : provider.models.map(({ id }) => ({ id, label: id, description: null, enabled: true, reason: null })),
    efforts: [],
  }
  return { state: 'ready', choices }
}
