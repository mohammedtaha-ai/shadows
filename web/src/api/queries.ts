// One job: the TanStack Query identity of each piece of daemon state.

import { queryOptions } from '@tanstack/react-query'
import {
  listDirs,
  listHarnesses,
} from './client'

/** The CLIs a conversation can run on (spec §12.10). Their limits move, so a
 * screen that shows them refetches on focus like any other query. */
export const harnessesQuery = queryOptions({ queryKey: ['harnesses'], queryFn: listHarnesses })

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

export { projectsQuery, instructionsQuery, grantsQuery } from './projects'

export { codeStatusQuery, codeLinksQuery, codeSettingsQuery } from './code'
