// The HTTP calls for the project design workspace.
import createClient from 'openapi-fetch'
import { DAEMON_URL } from './client'
import { unwrap } from './error'
import type { components, paths } from './schema'

export type VisionContent = components['schemas']['VisionContent']
export type VisionView = components['schemas']['VisionView']
export type DesignChange = components['schemas']['DesignChange']
export type DesignEdit = components['schemas']['DesignEdit']
export type Part = components['schemas']['Part']
export type PartContent = components['schemas']['PartContent']
export type PartView = components['schemas']['PartView']
export type PartPage = components['schemas']['PartPage']
export type Outcome = components['schemas']['Outcome']
export type OutcomeContent = components['schemas']['OutcomeContent']
export type OutcomeView = components['schemas']['OutcomeView']
export type OutcomePage = components['schemas']['OutcomePage']
export function getOutcome(projectId: string, outcomeId: string): Promise<OutcomeView> {
  return unwrap(client.GET('/api/projects/{id}/design/outcomes/{outcome}', { params: { path: { id: projectId, outcome: outcomeId } } }))
}
export function getOutcomes(projectId: string, parent?: string, after?: string): Promise<OutcomePage> {
  return unwrap(client.GET('/api/projects/{id}/design/outcomes', { params: { path: { id: projectId }, query: { parent, after } } }))
}

export function getPart(projectId: string, partId: string): Promise<PartView> {
  return unwrap(client.GET('/api/projects/{id}/design/parts/{part}', { params: { path: { id: projectId, part: partId } } }))
}
export function getParts(projectId: string, parent?: string, after?: string): Promise<PartPage> {
  return unwrap(client.GET('/api/projects/{id}/design/parts', { params: { path: { id: projectId }, query: { parent, after } } }))
}
const client = createClient<paths>({ baseUrl: DAEMON_URL, fetch: (request) => fetch(request) })

export function getVision(projectId: string): Promise<VisionView> {
  return unwrap(client.GET('/api/projects/{id}/design/vision', { params: { path: { id: projectId } } }))
}

export function editDesign(projectId: string, body: DesignEdit): Promise<DesignChange> {
  return unwrap(client.POST('/api/projects/{id}/design/edits', { params: { path: { id: projectId } }, body }))
}
