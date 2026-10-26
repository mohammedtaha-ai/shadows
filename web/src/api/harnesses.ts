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
