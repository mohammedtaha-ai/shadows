// One job: refetching a project's plan views from its journal notifications (§16.8).

import { type QueryClient, useQueryClient } from '@tanstack/react-query'
import { useEffect } from 'react'
import { projectEventsUrl } from '@/api/client'
import { planMapQuery, planQuery, planVersionsQuery, plansQuery, projectsQuery } from '@/api/queries'
import { ThreadStream } from './thread-stream'

const PLAN_EVENTS = new Set([
  'WorkflowDraftStarted', 'WorkflowEdited', 'WorkflowFrozen', 'PlanArchived', 'PlanUnarchived',
])
const REACH_EVENTS = new Set(['PlanDependenciesChanged', 'ProjectLinked', 'ProjectUnlinked', 'ProjectRemoved'])

interface ProjectWatch { references: number; close: () => void }
const watches = new WeakMap<QueryClient, Map<string, ProjectWatch>>()

/** Visible views of the same project share one connection per query client.
 * The last view leaving closes it. ThreadStream owns reconnect and cursors. */
export function useProjectEvents(projectId: string | undefined): void {
  const queryClient = useQueryClient()
  useEffect(() => {
    if (projectId === undefined) return
    let projects = watches.get(queryClient)
    if (!projects) { projects = new Map(); watches.set(queryClient, projects) }
    let watch = projects.get(projectId)
    if (!watch) {
      watch = { references: 0, close: watchProject(queryClient, projectId) }
      projects.set(projectId, watch)
    }
    watch.references++
    return () => {
      watch.references--
      if (watch.references === 0) { watch.close(); projects.delete(projectId) }
    }
  }, [projectId, queryClient])
}

function watchProject(queryClient: QueryClient, projectId: string): () => void {
    const pending = new Map<string, string>()
    const invalidateProject = () => {
      void queryClient.invalidateQueries({ queryKey: planMapQuery(projectId).queryKey })
      void queryClient.invalidateQueries({ queryKey: plansQuery(projectId).queryKey.slice(0, 3) })
      void queryClient.invalidateQueries({ queryKey: ['workflows', 'plan'], predicate: query => {
        const data = query.state.data
        return typeof data === 'object' && data !== null && 'project_id' in data && data.project_id === projectId
      } })
    }
    const refetch = (planId: string, workflowId: string) => {
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
    const stream = new ThreadStream({
      url: (after) => projectEventsUrl(projectId, after),
      onDurable: (event) => {
        if (event.kind === 'ProjectDesignChanged') {
          void queryClient.invalidateQueries({ queryKey: ['projects', projectId, 'design'] })
          return
        }
        if (REACH_EVENTS.has(event.kind)) {
          if (stream.getState().connection === 'live') invalidateProject()
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
        if (stream.getState().connection === 'live') refetch(payload.plan_id, payload.workflow_id)
        else pending.set(payload.workflow_id, payload.plan_id)
      },
      onCaughtUp: () => {
        invalidateProject()
        // Refetch even without a replayed design event: reconnect can follow a stale cache read.
        void queryClient.invalidateQueries({ queryKey: ['projects', projectId, 'design'] })
        for (const [workflow, plan] of pending) refetch(plan, workflow)
        pending.clear()
      },
    })
    stream.start()
    return () => stream.close()
}
