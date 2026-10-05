// One job: the TanStack Query identity of each piece of daemon state.

import { queryOptions } from '@tanstack/react-query'
import { getOutcome, getOutcomes, getPart, getParts, getVision } from './design'
import {
  getCodeSettings,
  getCodeStatus,
  getInstructions,
  getPlan,
  getPlanMap,
  getPlanVersions,
  getThread,
  listCodeLinks,
  listGrants,
  listDirs,
  listEntries,
  listHarnesses,
  listOperations,
  listPlans,
  listProjects,
  listQueued,
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

/** A project's planning threads, oldest first. A thread's own stream
 * refreshes the list on its `ThreadRetitled`, but the harness titles a thread
 * after its turn has ended, often once the person has left it, so the list is
 * also polled every 10 s while shown (spec §12.10). */
export function threadsQuery(projectId: string) {
  return queryOptions({
    queryKey: ['projects', projectId, 'threads'],
    queryFn: () => listThreads(projectId),
    refetchInterval: 10_000,
  })
}

/** Historical conversation links still read a thread omitted from the live list. */
export function threadQuery(threadId: string) {
  return queryOptions({
    queryKey: ['threads', threadId, 'detail'],
    queryFn: () => getThread(threadId),
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

/** A thread's waiting messages (spec §20); the stream's queue events invalidate it. */
export function queuedQuery(threadId: string) {
  return queryOptions({
    queryKey: ['threads', threadId, 'queue'] as const,
    queryFn: () => listQueued(threadId),
  })
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

export function planMapQuery(projectId: string) {
  return queryOptions({ queryKey: [...workflowsKey, 'map', projectId], queryFn: () => getPlanMap(projectId) })
}

/** A project's plans, each plan's latest version (§16.2), or archived ones too. */
export function plansQuery(projectId: string, archived = false) {
  return queryOptions({
    queryKey: [...workflowsKey, 'list', projectId, { archived }],
    queryFn: () => listPlans(projectId, archived),
  })
}

/** One plan with every version, oldest first (§16.10). */
export function planVersionsQuery(planId: string) {
  return queryOptions({
    queryKey: [...workflowsKey, 'versions', planId],
    queryFn: () => getPlanVersions(planId),
  })
}

/** One plan version, as the Workflows page and the conversation's cards show it. */
export function planQuery(workflowId: string) {
  return queryOptions({
    queryKey: [...workflowsKey, 'plan', workflowId],
    queryFn: () => getPlan(workflowId),
  })
}

/** A project's current Planner instructions (spec §13.8). */
export function instructionsQuery(projectId: string) {
  return queryOptions({
    queryKey: ['projects', projectId, 'instructions'],
    queryFn: () => getInstructions(projectId),
  })
}

export function visionQuery(projectId: string) {
  return queryOptions({
    queryKey: ['projects', projectId, 'design', 'vision'],
    queryFn: () => getVision(projectId),
  })
}

export function partQuery(projectId: string, partId: string) {
  return queryOptions({ queryKey: ['projects', projectId, 'design', 'part', partId], queryFn: () => getPart(projectId, partId) })
}
export function outcomeQuery(projectId: string, outcomeId: string) {
  return queryOptions({ queryKey: ['projects', projectId, 'design', 'outcome', outcomeId], queryFn: () => getOutcome(projectId, outcomeId) })
}
export function outcomesQuery(projectId: string, parent?: string, after?: string) {
  return queryOptions({ queryKey: ['projects', projectId, 'design', 'outcomes', parent ?? null, after ?? null], queryFn: () => getOutcomes(projectId, parent, after) })
}
export function partsQuery(projectId: string, parent?: string, after?: string) {
  return queryOptions({ queryKey: ['projects', projectId, 'design', 'parts', parent ?? null, after ?? null], queryFn: () => getParts(projectId, parent, after) })
}

/** A project's grants for external agents (spec §13.7). No thread's stream
 * carries them, so the list is polled every 10 s while shown. */
export function grantsQuery(projectId: string) {
  return queryOptions({
    queryKey: ['projects', projectId, 'grants'],
    queryFn: () => listGrants(projectId),
    refetchInterval: 10_000,
  })
}

/** How a project's code index stands (§15.5). The index changes on its own
 * as files do, so it is polled while shown: every 2 s while it indexes, so
 * the count moves, and every 10 s otherwise. */
export function codeStatusQuery(projectId: string) {
  return queryOptions({
    queryKey: ['projects', projectId, 'code', 'status'],
    queryFn: () => getCodeStatus(projectId),
    refetchInterval: (query) => (query.state.data?.state.state === 'indexing' ? 2_000 : 10_000),
  })
}

/** The projects whose index this one reads, by slug (§15.6). */
export function codeLinksQuery(projectId: string) {
  return queryOptions({
    queryKey: ['projects', projectId, 'code', 'links'],
    queryFn: () => listCodeLinks(projectId),
  })
}

/** The code index's settings, global to the daemon (§15.6). */
export const codeSettingsQuery = queryOptions({
  queryKey: ['code', 'settings'],
  queryFn: getCodeSettings,
})

/** One directory's subdirectories; `null` lists the roots. Not retried: a
 * path that does not exist is an answer, shown as it is. */
export function dirsQuery(path: string | null) {
  return queryOptions({
    queryKey: ['fs', 'dirs', path],
    queryFn: () => listDirs(path),
    retry: false,
  })
}
