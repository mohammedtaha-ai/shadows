import { queryOptions, type QueryClient } from '@tanstack/react-query'
import { getPlan, getPlanMap, getPlanVersions, listPlans, type Plan } from './client'
import { writeSaved } from './saved-answer'
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
