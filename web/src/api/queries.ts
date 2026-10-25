// One job: the TanStack Query identity of each piece of daemon state.

import { queryOptions } from '@tanstack/react-query'
import {
  getCodeSettings,
  getCodeStatus,
  getInstructions,
  listCodeLinks,
  listGrants,
  listDirs,
  listHarnesses,
  listProjects,
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

/** A project's current Planner instructions (spec §13.8). */
export function instructionsQuery(projectId: string) {
  return queryOptions({
    queryKey: ['projects', projectId, 'instructions'],
    queryFn: () => getInstructions(projectId),
  })
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

export { workflowsKey, planMapQuery, plansQuery, planVersionsQuery, planQuery } from './workflows'

export { standardsQuery, stageQuery, visionQuery, visionProposalsQuery, mapProposalsQuery, roadmapProposalsQuery, partQuery, outcomeQuery, outcomesQuery, partsQuery } from './design'

export { threadsQuery, threadQuery, threadEntriesKey, entriesQuery, queuedQuery, threadOperationsKey, operationsQuery } from './threads'
