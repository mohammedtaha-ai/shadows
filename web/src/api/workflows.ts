import { queryOptions, type QueryClient } from '@tanstack/react-query'
import { writeSaved } from './saved-answer'
import type { components } from './schema'
import { client, unwrap } from './transport'

export const workflowKeys = {
  all: () => ['workflows'] as const,
  plans: () => ['workflows', 'plan'],
  map: (projectId: string) => ['workflows', 'map', projectId],
  list: (projectId: string, archived: boolean) => ['workflows', 'list', projectId, { archived }],
  versions: (planId: string) => ['workflows', 'versions', planId],
  plan: (workflowId: string) => ['workflows', 'plan', workflowId],
}

/** Every plan query, lists and versions alike: a thread's `Workflow*` event
 * or an approval invalidates this prefix, since a new draft also changes its
 * predecessor's `next`. */
export const workflowsKey = workflowKeys.all()

export function planMapQuery(projectId: string) {
  return queryOptions({ queryKey: workflowKeys.map(projectId), queryFn: () => getPlanMap(projectId) })
}

/** A project's plans, each plan's latest version (§16.2), or archived ones too. */
export function plansQuery(projectId: string, archived = false) {
  return queryOptions({
    queryKey: workflowKeys.list(projectId, archived),
    queryFn: () => listPlans(projectId, archived),
  })
}

/** One plan with every version, oldest first (§16.10). */
export function planVersionsQuery(planId: string) {
  return queryOptions({
    queryKey: workflowKeys.versions(planId),
    queryFn: () => getPlanVersions(planId),
  })
}

/** One plan version, as the Workflows page and the conversation's cards show it. */
export function planQuery(workflowId: string) {
  return queryOptions({
    queryKey: workflowKeys.plan(workflowId),
    queryFn: () => getPlan(workflowId),
  })
}
export function savePlanAnswer(client: QueryClient, plan: Plan) {
  return writeSaved(client, workflowKeys.plan(plan.id), plan)
}

export function invalidateWorkflows(client: QueryClient): void {
  void client.invalidateQueries({ queryKey: workflowKeys.all() })
}

export function invalidateProjectWorkflows(client: QueryClient, projectId: string): void {
  void client.invalidateQueries({ queryKey: workflowKeys.map(projectId) })
  void client.invalidateQueries({ queryKey: workflowKeys.list(projectId, false).slice(0, 3) })
  void client.invalidateQueries({ queryKey: workflowKeys.plans(), predicate: query => {
    const data = query.state.data
    return typeof data === 'object' && data !== null && 'project_id' in data && data.project_id === projectId
  } })
}

export type PlanOp = components['schemas']['PlanOp']

export type Plan = components['schemas']['Plan']

export type PlanTask = components['schemas']['PlanTask']

export type PlanLink = components['schemas']['Link']

export type LinkedTask = components['schemas']['LinkedTask']

export type PlanMap = components['schemas']['PlanMap']

export type MapPlan = components['schemas']['MapPlan']

export type PlanListing = components['schemas']['PlanListing']

export type PlanVersions = components['schemas']['PlanVersions']

export type VersionLine = components['schemas']['VersionLine']

export type WrittenBy = components['schemas']['WrittenBy']

export type Approved = components['schemas']['Approved']

/** Each project's plan by its latest version, or archived ones too. */
export function listPlans(projectId: string, archived = false): Promise<PlanListing[]> {
  return unwrap(
    client.GET('/api/projects/{id}/workflows', {
      params: { path: { id: projectId }, query: { archived } },
    }),
  )
}

/** One plan with every version, oldest first. */
export function getPlanVersions(planId: string): Promise<PlanVersions> {
  return unwrap(client.GET('/api/plans/{id}', { params: { path: { id: planId } } }))
}

/** Archives a plan. */
export function archivePlan(planId: string, commandId: string): Promise<PlanVersions> {
  return unwrap(
    client.POST('/api/plans/{id}/archive', {
      params: { path: { id: planId } },
      body: { command_id: commandId },
    }),
  )
}

/** Unarchives a plan. */
export function unarchivePlan(planId: string, commandId: string): Promise<PlanVersions> {
  return unwrap(
    client.POST('/api/plans/{id}/unarchive', {
      params: { path: { id: planId } },
      body: { command_id: commandId },
    }),
  )
}

/** One plan version: tasks, links, revision, its neighbours, what blocks its
 * approval and what the last edit changed. */
export function getPlan(workflowId: string): Promise<Plan> {
  return unwrap(client.GET('/api/workflows/{id}', { params: { path: { id: workflowId } } }))
}

export function getPlanMap(projectId: string): Promise<PlanMap> {
  return unwrap(client.GET('/api/projects/{id}/plan-map', { params: { path: { id: projectId } } }))
}

/** Approves a Draft at the revision the person saw. Answers what
 * the approval did, not the plan: the caller reads the plan again. */
export function approvePlan(
  workflowId: string,
  commandId: string,
  expectedRevision: number,
): Promise<Approved> {
  return unwrap(
    client.POST('/api/workflows/{id}/approve', {
      params: { path: { id: workflowId } },
      body: { command_id: commandId, expected_revision: expectedRevision },
    }),
  )
}

export function editBindings(id: string, command_id: string, expected_revision: number,
  ops: PlanOp[]): Promise<components['schemas']['EditOutcome']> {
  return unwrap(client.POST('/api/workflows/{id}/bindings', {
    params: { path: { id } }, body: { command_id, expected_revision, ops },
  }))
}
