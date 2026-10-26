// One job: refetching a project's plan views from its journal notifications (§16.8).

import { type QueryClient, useQueryClient } from '@tanstack/react-query'
import { useEffect } from 'react'
import { planMapQuery, planQuery, planVersionsQuery, plansQuery, projectsQuery } from '@/api/queries'
import { designKeys } from '@/api/design'
import { workflowKeys } from '@/api/workflows'
import { agreementKeys } from '@/api/agreements'
import { subscribeProjectStream } from './thread-registry'

const PLAN_EVENTS = new Set([
  'WorkflowDraftStarted', 'WorkflowEdited', 'WorkflowFrozen', 'PlanArchived', 'PlanUnarchived',
])
const REACH_EVENTS = new Set(['PlanDependenciesChanged', 'ProjectLinked', 'ProjectUnlinked', 'ProjectRemoved'])

export function useProjectEvents(projectId: string | undefined): void {
  const queryClient = useQueryClient()
  useEffect(() => {
    if (projectId === undefined) return
    return watchProject(queryClient, projectId)
  }, [projectId, queryClient])
}

function watchProject(queryClient: QueryClient, projectId: string): () => void {
    const pending = new Map<string, string>()
    const invalidateProject = () => {
      void queryClient.invalidateQueries({ queryKey: planMapQuery(projectId).queryKey })
      void queryClient.invalidateQueries({ queryKey: plansQuery(projectId).queryKey.slice(0, 3) })
      void queryClient.invalidateQueries({ queryKey: workflowKeys.plans(), predicate: query => {
        const data = query.state.data
        return typeof data === 'object' && data !== null && 'project_id' in data && data.project_id === projectId
      } })
    }
    const refetch = (planId: string, workflowId: string) => {
      void queryClient.invalidateQueries({ queryKey: designKeys.all(projectId) })
      void queryClient.invalidateQueries({ queryKey: agreementKeys.versions(projectId) })
      invalidateProject()
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
    return subscribeProjectStream(queryClient, projectId, {
      onDurable: (event, live) => {
        if (event.kind === 'AgreementChanged') {
          void queryClient.invalidateQueries({ queryKey: agreementKeys.list(projectId) })
          void queryClient.invalidateQueries({ queryKey: agreementKeys.versions(projectId) })
          return
        }
        if (event.kind === 'ProjectDesignChanged' || event.kind === 'ProjectStandardsSaved' || event.kind === 'VisionProposed' || event.kind === 'MapProposed' || event.kind === 'RoadmapProposed' || event.kind === 'RoadmapApproved' || event.kind === 'ProjectStructureWritten') {
          void queryClient.invalidateQueries({ queryKey: designKeys.all(projectId) })
          return
        }
        if (REACH_EVENTS.has(event.kind)) {
          if (live) invalidateProject()
          if (event.kind === 'ProjectRemoved') {
            void queryClient.invalidateQueries({ queryKey: projectsQuery.queryKey, exact: true })
          }
          return
        }
        if (!PLAN_EVENTS.has(event.kind)) return
        const payload = event.payload
        if (typeof payload !== 'object' || payload === null ||
          !('plan_id' in payload) || typeof payload.plan_id !== 'string' ||
          !('workflow_id' in payload) || typeof payload.workflow_id !== 'string') return
        if (live) refetch(payload.plan_id, payload.workflow_id)
        else pending.set(payload.workflow_id, payload.plan_id)
      },
      onCaughtUp: () => {
        void queryClient.invalidateQueries({ queryKey: agreementKeys.list(projectId) })
        void queryClient.invalidateQueries({ queryKey: agreementKeys.versions(projectId) })
        invalidateProject()
        // Refetch even without a replayed design event: reconnect can follow a stale cache read.
        void queryClient.invalidateQueries({ queryKey: designKeys.all(projectId) })
        for (const [workflow, plan] of pending) refetch(plan, workflow)
        pending.clear()
      },
    })
}
