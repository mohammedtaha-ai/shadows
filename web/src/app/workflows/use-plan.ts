// One job: one plan version as a component reads it, kept current.

import { type UseQueryResult, useQuery } from '@tanstack/react-query'
import type { Plan, WrittenBy } from '@/api/workflows'
import { planQuery } from '@/api/workflows'
import { usePlanFrames } from './plan-frames'

/** Plan version `workflowId`, refetched whenever its thread's stream carries
 * a plan change, including archive or unarchive. Used by the
 * Workflows page and by the conversation's plan cards and side panel. */
export function usePlan(workflowId: string): UseQueryResult<Plan> {
  const plan = useQuery(planQuery(workflowId))
  usePlanFrames(writerThread(plan.data?.written_by), plan.data?.project_id)
  return plan
}

/** The conversation that wrote `plan`, when a Planner did (§16.3). */
export function writerThread(target: WrittenBy | Plan | undefined): string | undefined {
  if (target === undefined) return undefined
  const writer = 'written_by' in target ? target.written_by : target
  return writer.kind === 'planner' ? writer.thread_id : undefined
}
