// The HTTP calls for the project design workspace.
import createClient from 'openapi-fetch'
import { DAEMON_URL } from './client'
import { unwrap } from './error'
import type { components, paths } from './schema'

export type VisionContent = components['schemas']['VisionContent']
export type VisionView = components['schemas']['VisionView']
export type DesignChange = components['schemas']['DesignChange']
export type DesignEdit = components['schemas']['DesignEdit']
const client = createClient<paths>({ baseUrl: DAEMON_URL, fetch: (request) => fetch(request) })

export function getVision(projectId: string): Promise<VisionView> {
  return unwrap(client.GET('/api/projects/{id}/design/vision', { params: { path: { id: projectId } } }))
}

export function editDesign(projectId: string, body: DesignEdit): Promise<DesignChange> {
  return unwrap(client.POST('/api/projects/{id}/design/edits', { params: { path: { id: projectId } }, body }))
}
