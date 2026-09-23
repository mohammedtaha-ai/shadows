// One job: the TanStack Query identity of each piece of daemon state.

import { queryOptions } from '@tanstack/react-query'
import { listProjects } from './client'

/** The project list, which the app shell also reads as "is the daemon there".
 *
 * Polled only once the daemon has answered, so a daemon that goes away turns
 * the indicator red. Before the first answer it is not polled: a query with no
 * data goes back to `pending` on every fetch, which would flash "Connecting"
 * over the error every few seconds. Then the next attempt is the Try again
 * button, or coming back to the window after starting the daemon in a
 * terminal (TanStack Query refetches on focus). */
export const projectsQuery = queryOptions({
  queryKey: ['projects'],
  queryFn: listProjects,
  retry: 2,
  retryDelay: (attempt) => Math.min(500 * 2 ** attempt, 4000),
  refetchInterval: (query) => (query.state.data === undefined ? false : 10_000),
})

/** A thread's entries (`GET /api/threads/{id}/entries`). A durable
 * `ThreadEntryAppended` carries only `{ordinal, kind}`, so the stream hook
 * invalidates this key and the entries are fetched again (ruling 51). */
export function threadEntriesKey(threadId: string) {
  return ['threads', threadId, 'entries'] as const
}
