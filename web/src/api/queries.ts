// One job: the TanStack Query identity of each piece of daemon state.

import { queryOptions } from '@tanstack/react-query'
import {
  getPlan,
  listDirs,
  listEntries,
  listHarnesses,
  listOperations,
  listPlans,
  listProjects,
  listThreads,
} from './client'

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

/** The CLIs a conversation can run on (spec §12.10). Their limits move, so a
 * screen that shows them refetches on focus like any other query. */
export const harnessesQuery = queryOptions({ queryKey: ['harnesses'], queryFn: listHarnesses })

/** A project's planning threads, oldest first. */
export function threadsQuery(projectId: string) {
  return queryOptions({
    queryKey: ['projects', projectId, 'threads'],
    queryFn: () => listThreads(projectId),
  })
}

/** A thread's entries (`GET /api/threads/{id}/entries`). A durable
 * `ThreadEntryAppended` carries only `{ordinal, kind}`, so the stream hook
 * invalidates this key and the entries are fetched again (ruling 51). */
export function threadEntriesKey(threadId: string) {
  return ['threads', threadId, 'entries'] as const
}

export function entriesQuery(threadId: string) {
  return queryOptions({ queryKey: threadEntriesKey(threadId), queryFn: () => listEntries(threadId) })
}

/** A thread's operations, newest first: on opening a thread, whether a turn is
 * running and which. The stream's durable operation events carry it on from
 * there; the stream hook invalidates this key at each `caught-up`. */
export function threadOperationsKey(threadId: string) {
  return ['threads', threadId, 'operations'] as const
}

export function operationsQuery(threadId: string) {
  return queryOptions({
    queryKey: threadOperationsKey(threadId),
    queryFn: () => listOperations(threadId),
  })
}

/** Every plan query, lists and versions alike: a thread's `Workflow*` event
 * or an approval invalidates this prefix, since a new draft also changes its
 * predecessor's `next`. */
export const workflowsKey = ['workflows'] as const

/** A project's plans, each conversation's latest version (spec §13.10). No
 * thread's stream carries the list, so it is polled every 10 s while shown;
 * TanStack Query pauses the poll while the window is hidden. */
export function plansQuery(projectId: string) {
  return queryOptions({
    queryKey: [...workflowsKey, 'list', projectId],
    queryFn: () => listPlans(projectId),
    refetchInterval: 10_000,
  })
}

/** One plan version, as the Workflows page and the conversation's cards show it. */
export function planQuery(workflowId: string) {
  return queryOptions({
    queryKey: [...workflowsKey, 'plan', workflowId],
    queryFn: () => getPlan(workflowId),
  })
}

/** One directory's subdirectories; `null` lists the roots. Not retried: a
 * path that does not exist is an answer, shown as it is. */
export function dirsQuery(path: string | null) {
  return queryOptions({
    queryKey: ['fs', 'dirs', path],
    queryFn: () => listDirs(path),
    retry: false,
  })
}
