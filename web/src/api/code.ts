import { queryOptions, type QueryClient } from '@tanstack/react-query'
import { writeSaved } from './saved-answer'
import type { components } from './schema'
import { client, unwrap, unwrapEmpty } from './transport'

export const codeKeys = {
  settings: () => ['code', 'settings'],
  status: (projectId: string) => ['projects', projectId, 'code', 'status'],
  links: (projectId: string) => ['projects', projectId, 'code', 'links'],
}

/** Poll while shown because files change independently: every 2 s while indexing, 10 s otherwise. */
export function codeStatusQuery(projectId: string) {
  return queryOptions({
    queryKey: codeKeys.status(projectId),
    queryFn: () => getCodeStatus(projectId),
    refetchInterval: (query) => (query.state.data?.state.state === 'indexing' ? 2_000 : 10_000),
  })
}

export function codeLinksQuery(projectId: string) {
  return queryOptions({
    queryKey: codeKeys.links(projectId),
    queryFn: () => listCodeLinks(projectId),
  })
}

/** Code index settings apply to the whole daemon. */
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

export type ProjectStatus = components['schemas']['ProjectStatus']

export type IndexState = components['schemas']['IndexState']

export type ProjectLink = components['schemas']['ProjectLink']

export type CodeSettings = components['schemas']['CodeSettings']

/** How a project's code index stands. Asking does not make it active. */
export function getCodeStatus(projectId: string): Promise<ProjectStatus> {
  return unwrap(
    client.GET('/api/projects/{id}/code/status', { params: { path: { id: projectId } } }),
  )
}

/** The projects whose index this one reads, by slug. */
export function listCodeLinks(projectId: string): Promise<ProjectLink[]> {
  return unwrap(
    client.GET('/api/projects/{id}/code/links', { params: { path: { id: projectId } } }),
  )
}

/** Lets the project read `linkedId`'s index, one way. */
export function putCodeLink(
  projectId: string,
  linkedId: string,
  commandId: string,
): Promise<ProjectLink> {
  return unwrap(
    client.PUT('/api/projects/{id}/code/links/{linked}', {
      params: { path: { id: projectId, linked: linkedId } },
      body: { command_id: commandId },
    }),
  )
}

/** Removes the link: the project no longer reads `linkedId`'s index. */
export function removeCodeLink(
  projectId: string,
  linkedId: string,
  commandId: string,
): Promise<void> {
  return unwrapEmpty(
    client.DELETE('/api/projects/{id}/code/links/{linked}', {
      params: { path: { id: projectId, linked: linkedId }, query: { command_id: commandId } },
    }),
  )
}

/** The code index's settings: how many projects are active at once. */
export function getCodeSettings(): Promise<CodeSettings> {
  return unwrap(client.GET('/api/code/settings'))
}

/** Sets how many projects are active at once, 1 to 20. */
export function setActiveLimit(commandId: string, activeLimit: number): Promise<CodeSettings> {
  return unwrap(
    client.PUT('/api/code/settings', {
      body: { command_id: commandId, active_limit: activeLimit },
    }),
  )
}
