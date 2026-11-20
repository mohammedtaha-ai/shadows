// The project's state and its tables and screens, as ProjectState and Design answer them.
import { queryOptions, type QueryClient } from '@tanstack/react-query'
import { retryLost } from './error'
import type { components } from './schema'
import { client, unwrap } from './transport'

export type ProjectStateView = components['schemas']['ProjectStateView']
export type PartState = components['schemas']['PartState']
/** The generated name of a plan's progress line; `PlanState` is Active/Archived. */
export type PlanProgress = components['schemas']['PlanProgress']
export type TaskCounts = components['schemas']['TaskCounts']
export type StateCounts = components['schemas']['StateCounts']
export type ElementGap = components['schemas']['ElementGap']
export type PartDetail = components['schemas']['PartDetail']
export type PlanTasks = components['schemas']['PlanTasks']
export type TaskLine = components['schemas']['TaskLine']
export type ElementView = components['schemas']['ElementView']
export type DesignElement = components['schemas']['DesignElement']
export type ElementState = components['schemas']['ElementState']
export type ElementTask = components['schemas']['ElementTask']
export type ElementPin = components['schemas']['ElementPin']
export type BuildState = components['schemas']['BuildState']

export const projectStateKeys = {
  all: (projectId: string) => ['projects', projectId, 'project-state'],
  reads: (projectId: string) => ['projects', projectId, 'project-state', 'reads'],
  state: (projectId: string, partId?: string) => ['projects', projectId, 'project-state', 'reads', 'state', partId ?? null],
  elements: (projectId: string) => ['projects', projectId, 'project-state', 'reads', 'elements'],
  revisions: (projectId: string, elementId: string) => ['projects', projectId, 'project-state', 'revisions', elementId],
}

export function projectStateQuery(projectId: string, partId?: string) {
  return queryOptions({
    queryKey: projectStateKeys.state(projectId, partId),
    queryFn: () => unwrap(client.GET('/api/projects/{id}/state', {
      params: { path: { id: projectId }, query: partId === undefined ? undefined : { part_id: partId } },
    })),
    retry: retryLost,
  })
}

export function elementsQuery(projectId: string) {
  return queryOptions({
    queryKey: projectStateKeys.elements(projectId),
    queryFn: () => unwrap(client.GET('/api/projects/{id}/elements', { params: { path: { id: projectId } } })),
    retry: retryLost,
  })
}

export function elementRevisionsQuery(projectId: string, elementId: string) {
  return queryOptions({
    queryKey: projectStateKeys.revisions(projectId, elementId),
    queryFn: () => unwrap(client.GET('/api/projects/{id}/elements/{element}/revisions', {
      params: { path: { id: projectId, element: elementId } },
    })),
    retry: retryLost,
  })
}

export function invalidateProjectState(client: QueryClient, projectId: string): void {
  void client.invalidateQueries({ queryKey: projectStateKeys.reads(projectId) })
}

export function invalidateElementRevisions(client: QueryClient, projectId: string, elementId: string): void {
  void client.invalidateQueries({ queryKey: projectStateKeys.revisions(projectId, elementId) })
}
