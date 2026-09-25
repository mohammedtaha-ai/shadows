// One job: one plan version as a component reads it, kept current.

import { type UseQueryResult, useQuery } from '@tanstack/react-query'
import type { Plan } from '@/api/client'
import { planQuery } from '@/api/queries'
import { usePlanFrames } from './plan-frames'

/** Plan version `workflowId`, refetched whenever its thread's stream carries
 * a `WorkflowEdited`, `WorkflowFrozen` or `WorkflowDraftStarted`. Used by the
 * Workflows page and by the conversation's plan cards and side panel. */
export function usePlan(workflowId: string): UseQueryResult<Plan> {
  const plan = useQuery(planQuery(workflowId))
  usePlanFrames(plan.data?.thread_id)
  return plan
}
