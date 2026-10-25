import { queryOptions, type QueryClient } from '@tanstack/react-query'
import { getThread, listEntries, listOperations, listQueued, listThreads, type PlanningThread } from './client'
import { writeSaved } from './saved-answer'
export const threadKeys = {
  list: (projectId: string) => ['projects', projectId, 'threads'],
  detail: (threadId: string) => ['threads', threadId, 'detail'],
  entries: (threadId: string) => ['threads', threadId, 'entries'] as const,
  queue: (threadId: string) => ['threads', threadId, 'queue'] as const,
  operations: (threadId: string) => ['threads', threadId, 'operations'] as const,
}


/** A project's planning threads, oldest first. A thread's own stream
 * refreshes the list on its `ThreadRetitled`, but the harness titles a thread
 * after its turn has ended, often once the person has left it, so the list is
 * also polled every 10 s while shown (spec §12.10). */
export function threadsQuery(projectId: string) {
  return queryOptions({
    queryKey: threadKeys.list(projectId),
    queryFn: () => listThreads(projectId),
    refetchInterval: 10_000,
  })
}


/** Historical conversation links still read a thread omitted from the live list. */
export function threadQuery(threadId: string) {
  return queryOptions({
    queryKey: threadKeys.detail(threadId),
    queryFn: () => getThread(threadId),
  })
}


/** A thread's entries (`GET /api/threads/{id}/entries`). A durable
 * `ThreadEntryAppended` carries only `{ordinal, kind}`, so the stream hook
 * invalidates this key and the entries are fetched again (ruling 51). */
export function threadEntriesKey(threadId: string) {
  return threadKeys.entries(threadId)
}


export function entriesQuery(threadId: string) {
  return queryOptions({ queryKey: threadEntriesKey(threadId), queryFn: () => listEntries(threadId) })
}


/** A thread's waiting messages (spec §20); the stream's queue events invalidate it. */
export function queuedQuery(threadId: string) {
  return queryOptions({
    queryKey: threadKeys.queue(threadId),
    queryFn: () => listQueued(threadId),
  })
}


/** A thread's operations, newest first: on opening a thread, whether a turn is
 * running and which. The stream's durable operation events carry it on from
 * there; the stream hook invalidates this key at each `caught-up`. */
export function threadOperationsKey(threadId: string) {
  return threadKeys.operations(threadId)
}


export function operationsQuery(threadId: string) {
  return queryOptions({
    queryKey: threadOperationsKey(threadId),
    queryFn: () => listOperations(threadId),
  })
}
export function saveThreadHarnessAnswer(client: QueryClient, projectId: string, thread: PlanningThread) {
  return writeSaved<PlanningThread[]>(client, threadKeys.list(projectId), threads =>
    threads?.map(value => value.id === thread.id ? thread : value))
}
export async function saveRemovedThreadAnswer(client: QueryClient, thread: PlanningThread) {
  await writeSaved(client, threadKeys.detail(thread.id), thread)
  await writeSaved<PlanningThread[]>(client, threadKeys.list(thread.project_id), threads =>
    threads?.filter(value => value.id !== thread.id))
}
