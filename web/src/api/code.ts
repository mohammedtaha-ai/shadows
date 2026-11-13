import { queryOptions, type QueryClient } from '@tanstack/react-query'
import { writeSaved } from './saved-answer'
import type { components } from './schema'
import { client, unwrap, unwrapEmpty } from './transport'

export const codeKeys = {
  servers: (projectId: string) => ['projects', projectId, 'code', 'servers'],
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

export function codeServersQuery(projectId: string) {
  return queryOptions({
    queryKey: codeKeys.servers(projectId),
    queryFn: () => listServers(projectId),
    // A server starts or fails on its own, so a shown list follows it.
    refetchInterval: 5_000,
  })
}
export function saveUnlinkedCodeAnswer(client: QueryClient, projectId: string, linked: string) {
  return writeSaved<ProjectLink[]>(client, codeKeys.links(projectId), links => links?.filter(value => value.linked !== linked))
}

export type ProjectStatus = components['schemas']['ProjectStatus']


export type ProjectLink = components['schemas']['ProjectLink']

export type ServerStatus = components['schemas']['ServerStatus']

export type AddedServer = components['schemas']['AddedServer']

/** How a project's code index stands. Asking does not make it active. */
function getCodeStatus(projectId: string): Promise<ProjectStatus> {
  return unwrap(
    client.GET('/api/projects/{id}/code/status', { params: { path: { id: projectId } } }),
  )
}

/** The projects whose index this one reads, by slug. */
function listCodeLinks(projectId: string): Promise<ProjectLink[]> {
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

/** Every catalog and added server as the project sees it. Asking does not make it active. */
function listServers(projectId: string): Promise<ServerStatus[]> {
  return unwrap(
    client.GET('/api/projects/{id}/code/servers', { params: { path: { id: projectId } } }),
  )
}

/** Switches a server off for every project, or back on. */
export function setServerOff(serverId: string, off: boolean, commandId: string): Promise<void> {
  return unwrapEmpty(
    client.PUT('/api/code/servers/{server}/off', {
      params: { path: { server: serverId } },
      body: { command_id: commandId, off },
    }),
  )
}

/** Adds a server beside the catalog, on the person's own responsibility. */
export function addServer(server: AddedServer, commandId: string): Promise<ServerStatus> {
  return unwrap(
    client.POST('/api/code/servers', { body: { ...server, command_id: commandId } }),
  )
}

/** Removes a server the person added. */
export function removeServer(serverId: string, commandId: string): Promise<void> {
  return unwrapEmpty(
    client.DELETE('/api/code/servers/{server}', {
      params: { path: { server: serverId }, query: { command_id: commandId } },
    }),
  )
}
