// One job: refetching a project's plan views from its journal notifications (§16.8).

import { useQueryClient } from '@tanstack/react-query'
import { useEffect } from 'react'
import { projectEventsUrl } from '@/api/client'
import { planQuery, planVersionsQuery, plansQuery } from '@/api/queries'
import { ThreadStream } from './thread-stream'

const PLAN_EVENTS = new Set([
  'WorkflowDraftStarted', 'WorkflowEdited', 'WorkflowFrozen', 'PlanArchived', 'PlanUnarchived',
])

/** One connection in the layout; changing projects or leaving closes it.
 * Reuses the conversation stream's cursor, reconnect and de-duplication. */
export function useProjectEvents(projectId: string | undefined): void {
  const queryClient = useQueryClient()
  useEffect(() => {
    if (projectId === undefined) return
    const pending = new Map<string, string>()
    const refetch = (planId: string, workflowId: string) => {
      // Include Active-only and archived-inclusive list variants.
      void queryClient.invalidateQueries({ queryKey: plansQuery(projectId).queryKey.slice(0, 3) })
      void queryClient.invalidateQueries({ queryKey: planVersionsQuery(planId).queryKey })
      void queryClient.invalidateQueries({
        queryKey: planQuery(workflowId).queryKey.slice(0, 2),
        predicate: (query) => {
          const data = query.state.data
          return query.queryKey[2] === workflowId ||
            (typeof data === 'object' && data !== null && 'plan_id' in data && data.plan_id === planId)
        },
      })
    }
    const stream = new ThreadStream({
      url: (after) => projectEventsUrl(projectId, after),
      onDurable: (event) => {
        if (!PLAN_EVENTS.has(event.kind)) return
        const payload = event.payload
        if (typeof payload !== 'object' || payload === null ||
          !('plan_id' in payload) || typeof payload.plan_id !== 'string' ||
          !('workflow_id' in payload) || typeof payload.workflow_id !== 'string') return
        if (stream.getState().connection === 'live') refetch(payload.plan_id, payload.workflow_id)
        else pending.set(payload.workflow_id, payload.plan_id)
      },
      onCaughtUp: () => {
        for (const [workflow, plan] of pending) refetch(plan, workflow)
        pending.clear()
      },
    })
    stream.start()
    return () => stream.close()
  }, [projectId, queryClient])
}
