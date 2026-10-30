import { queryOptions } from '@tanstack/react-query'
import type { components } from './schema'
import { client, unwrap } from './transport'

export const harnessKeys = { list: () => ['harnesses'] }
// Harness capabilities can change, so reads retain normal focus refetching.
export const harnessesQuery = queryOptions({ queryKey: harnessKeys.list(), queryFn: listHarnesses })

export type HarnessInfo = components['schemas']['HarnessInfo']

/** The CLIs a conversation can run on, with what each remembers and its
 * latest reported limits. */
export function listHarnesses(): Promise<HarnessInfo[]> {
  return unwrap(client.GET('/api/harnesses'))
}

/** What Core allows on one harness: its modes in listed order, the mode a new
 * conversation starts in and the allowed mode that asks nothing. */
export type HarnessPolicy = Pick<HarnessInfo, 'modes' | 'initial_mode' | 'unattended_mode'>

/** No decided modes: what a harness absent from the list, or a list not yet
 * read, offers. */
const NO_POLICY: HarnessPolicy = { modes: [], initial_mode: null, unattended_mode: null }

/** The policy of the harness of `kind`; none while the list is unread or lacks it. */
export function policyOf(harnesses: readonly HarnessInfo[] | undefined, kind: string): HarnessPolicy {
  return harnesses?.find((h) => h.kind === kind) ?? NO_POLICY
}

/** How the policy names `mode`; the id itself for a mode it does not list. */
export function modeLabel(policy: HarnessPolicy, mode: string): string {
  return policy.modes.find((m) => m.id === mode)?.label ?? mode
}

/** What the policy says `mode` allows; `null` for a mode it does not list. */
export function modeDescription(policy: HarnessPolicy, mode: string): string | null {
  return policy.modes.find((m) => m.id === mode)?.description ?? null
}
