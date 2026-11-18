import { queryOptions, type QueryClient } from '@tanstack/react-query'
import { writeSaved } from './saved-answer'
import type { components } from './schema'
import { client, DAEMON_URL, unwrap } from './transport'

export const projectKeys = {
  all: () => ['projects'],
  project: (projectId: string) => ['projects', projectId],
  instructions: (projectId: string) => ['projects', projectId, 'instructions'],
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

export function instructionsQuery(projectId: string) {
  return queryOptions({
    queryKey: projectKeys.instructions(projectId),
    queryFn: () => getInstructions(projectId),
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

export function invalidateProjects(client: QueryClient): void {
  void client.invalidateQueries({ queryKey: projectsQuery.queryKey, exact: true })
}

export type Project = components['schemas']['Project']

type CreateProject = components['schemas']['CreateProject']

export type InstructionsVersion = components['schemas']['InstructionsVersion']

/** Every project, oldest first. Also the reachability probe: it is the
 * cheapest call that proves the daemon answers this origin. */
function listProjects(): Promise<Project[]> {
  return unwrap(client.GET('/api/projects'))
}

/** Creates a project owning an existing directory. Replaying the same
 * `command_id` with the same body answers the first result. */
export function createProject(body: CreateProject): Promise<Project> {
  return unwrap(client.POST('/api/projects', { body }))
}

/** Sets the modes a project allows, per harness kind. */
export function setProjectModes(
  projectId: string,
  commandId: string,
  allowedModes: Record<string, string[]>,
): Promise<Project> {
  return unwrap(
    client.PATCH('/api/projects/{id}', {
      params: { path: { id: projectId } },
      body: { command_id: commandId, allowed_modes: allowedModes },
    }),
  )
}

/** Removes a project that holds no conversation: it is listed
 * nowhere again, its code index, links and grants go, its folder is not
 * touched and its slug stays taken. Refused `PROJECT_HAS_THREADS` otherwise. */
export function removeProject(projectId: string, commandId: string): Promise<Project> {
  return unwrap(
    client.DELETE('/api/projects/{id}', {
      params: { path: { id: projectId }, query: { command_id: commandId } },
    }),
  )
}

/** The project's current instructions, or `null` before the first save. */
function getInstructions(projectId: string): Promise<InstructionsVersion | null> {
  return unwrap(
    client.GET('/api/projects/{id}/planner-instructions', { params: { path: { id: projectId } } }),
  )
}

/** Saves the project's instructions as its next version. */
export function saveInstructions(
  projectId: string,
  commandId: string,
  body: string,
): Promise<InstructionsVersion> {
  return unwrap(
    client.PUT('/api/projects/{id}/planner-instructions', {
      params: { path: { id: projectId } },
      body: { command_id: commandId, body },
    }),
  )
}

/** Plan notifications of one project, resumed by the existing journal cursor. */
export function projectEventsUrl(projectId: string, after: number): string {
  const url = new URL(`/api/projects/${encodeURIComponent(projectId)}/events`, DAEMON_URL)
  url.searchParams.set('after', String(after))
  return url.toString()
}

export { DAEMON_URL } from './transport'
