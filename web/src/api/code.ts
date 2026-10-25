import { queryOptions, type QueryClient } from '@tanstack/react-query'
import { getCodeSettings, getCodeStatus, listCodeLinks, type CodeSettings, type ProjectLink } from './client'
import { writeSaved } from './saved-answer'
export const codeKeys = {
  settings: () => ['code', 'settings'],
  status: (projectId: string) => ['projects', projectId, 'code', 'status'],
  links: (projectId: string) => ['projects', projectId, 'code', 'links'],
}


/** How a project's code index stands (§15.5). The index changes on its own
 * as files do, so it is polled while shown: every 2 s while it indexes, so
 * the count moves, and every 10 s otherwise. */
export function codeStatusQuery(projectId: string) {
  return queryOptions({
    queryKey: codeKeys.status(projectId),
    queryFn: () => getCodeStatus(projectId),
    refetchInterval: (query) => (query.state.data?.state.state === 'indexing' ? 2_000 : 10_000),
  })
}


/** The projects whose index this one reads, by slug (§15.6). */
export function codeLinksQuery(projectId: string) {
  return queryOptions({
    queryKey: codeKeys.links(projectId),
    queryFn: () => listCodeLinks(projectId),
  })
}


/** The code index's settings, global to the daemon (§15.6). */
export const codeSettingsQuery = queryOptions({
  queryKey: codeKeys.settings(),
  queryFn: getCodeSettings,
})
export function saveCodeSettingsAnswer(client: QueryClient, settings: CodeSettings) {
  return writeSaved(client, codeKeys.settings(), settings)
}
export function saveUnlinkedCodeAnswer(client: QueryClient, projectId: string, linked: string) {
  return writeSaved<ProjectLink[]>(client, codeKeys.links(projectId), links => links?.filter(value => value.linked !== linked))
}
