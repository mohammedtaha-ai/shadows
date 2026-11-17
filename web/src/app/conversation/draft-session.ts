// Offer the draft's choices before a session exists.

import type { Choice, SessionChoices } from '@/api/sessions'
import type { Provider } from '@/api/aylul-profiles'
import type { CatalogModel, HarnessInfo, HarnessPolicy } from '@/api/harnesses'
import type { SessionView } from './use-session'

/** What a draft's model menu reads: no model is chosen before a session
 * exists, and Send takes the one the session opens at. */
export const DRAFT_SESSION_MODEL = 'unopened'
const UNOPENED: Choice = {
  id: DRAFT_SESSION_MODEL,
  label: 'Set on send',
  description: null,
  enabled: true,
  reason: null,
}

/** A draft's choices combine the project's modes with its CLI catalog or provider. */
export function draftSession(
  policy: HarnessPolicy | undefined, allowed: string[] | undefined, provider?: Provider, defaultModel?: string,
  catalog: readonly CatalogModel[] = [], remembered?: HarnessInfo['remembered'],
): SessionView {
  if (allowed === undefined || policy === undefined) return { state: 'connecting' }
  const model = catalog.find(item => item.id === remembered?.model)
  const catalogOptions: Choice[] = catalog.map(({ id, label, description }) =>
    ({ id, label, description, enabled: true, reason: null }))
  const choices: SessionChoices = {
    // Drafts stage files locally; the opened session and Core judge the first send.
    capabilities: { image: true, image_reason: null },
    current: {
      mode: policy.initial_mode ?? policy.modes[0]?.id ?? '',
      model: provider === undefined ? model?.id ?? UNOPENED.id : defaultModel ?? provider.models[0]!.id,
      effort: draftEffort(model, remembered?.effort ?? null),
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
    models: provider === undefined ? (model === undefined ? [UNOPENED, ...catalogOptions] : catalogOptions)
      : provider.models.map(({ id }) => ({ id, label: id, description: null, enabled: true, reason: null })),
    efforts: model?.efforts ?? [],
  }
  return { state: 'ready', choices }
}

export function draftEffort(model: CatalogModel | undefined, remembered: string | null): string | null {
  const efforts = model?.efforts.filter(item => item.enabled) ?? []
  return efforts.some(item => item.id === remembered) ? remembered : efforts[0]?.id ?? null
}
