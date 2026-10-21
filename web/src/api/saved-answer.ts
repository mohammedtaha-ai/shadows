// One job: write a mutation's answer into the query cache so that a read sent
// before the mutation cannot overwrite it when it answers late.
//
// A read in flight when the mutation started carries the old state. Cancelling
// it first (only that exact key, so other queries under the same prefix keep
// fetching) means its answer is dropped, and the written answer stays.

import type { QueryClient, QueryKey, Updater } from '@tanstack/react-query'

export async function writeSaved<TData = unknown>(
  client: QueryClient,
  queryKey: QueryKey,
  updater: Updater<TData | undefined, TData | undefined>,
): Promise<void> {
  await client.cancelQueries({ queryKey, exact: true })
  client.setQueryData<TData>(queryKey, updater)
}
