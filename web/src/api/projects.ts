import { queryOptions, type QueryClient } from '@tanstack/react-query'
import { getInstructions, listGrants, listProjects, type Project, type InstructionsVersion, type Grant } from './client'
import { writeSaved } from './saved-answer'
export const projectKeys = {
  all: () => ['projects'],
  project: (projectId: string) => ['projects', projectId],
  instructions: (projectId: string) => ['projects', projectId, 'instructions'],
  grants: (projectId: string) => ['projects', projectId, 'grants'],
}


/** The project list, which the app shell also reads as "is the daemon there".
 *
 * Polled only once the daemon has answered, so a daemon that goes away turns
 * the indicator red. Before the first answer it is not polled: a query with no
 * data goes back to `pending` on every fetch, which would flash "Connecting"
 * over the error every few seconds. Then the next attempt is the Try again
 * button, or coming back to the window after starting the daemon in a
 * terminal (TanStack Query refetches on focus). */
export const projectsQuery = queryOptions({
  queryKey: projectKeys.all(),
  queryFn: listProjects,
  retry: 2,
  retryDelay: (attempt) => Math.min(500 * 2 ** attempt, 4000),
  refetchInterval: (query) => (query.state.data === undefined ? false : 10_000),
})


/** A project's current Planner instructions (spec §13.8). */
export function instructionsQuery(projectId: string) {
  return queryOptions({
    queryKey: projectKeys.instructions(projectId),
    queryFn: () => getInstructions(projectId),
  })
}


/** A project's grants for external agents (spec §13.7). No thread's stream
 * carries them, so the list is polled every 10 s while shown. */
export function grantsQuery(projectId: string) {
  return queryOptions({
    queryKey: projectKeys.grants(projectId),
    queryFn: () => listGrants(projectId),
    refetchInterval: 10_000,
  })
}
export function saveProjectAnswer(client: QueryClient, project: Project) {
  return writeSaved<Project[]>(client, projectKeys.all(), projects =>
    projects?.map(value => value.id === project.id ? project : value))
}
export function saveRemovedProjectAnswer(client: QueryClient, projectId: string) {
  return writeSaved<Project[]>(client, projectKeys.all(), projects =>
    projects?.filter(value => value.id !== projectId))
}
export function saveInstructionsAnswer(client: QueryClient, projectId: string, version: InstructionsVersion) {
  return writeSaved(client, projectKeys.instructions(projectId), version)
}
export function saveRevokedGrantAnswer(client: QueryClient, projectId: string, revoked: Grant) {
  return writeSaved<Grant[]>(client, projectKeys.grants(projectId), grants =>
    grants?.map(value => value.id === revoked.id ? revoked : value))
}

export function invalidateProjects(client: QueryClient): void {
  void client.invalidateQueries({ queryKey: projectsQuery.queryKey, exact: true })
}
