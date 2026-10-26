import type { QueryClient } from '@tanstack/react-query'
import { invalidateWorkflows, invalidateProjectWorkflows } from '@/api/workflows'
import { invalidateDesign } from '@/api/design'
import { invalidateAgreements, invalidateAgreementVersions } from '@/api/agreements'
import { invalidateProjects } from '@/api/projects'
import { invalidateEntries, invalidateOperations, invalidateThreadLists } from '@/api/threads'
import type { DurableEvent } from './frames'
import type { StreamSubscriber } from './thread-registry'

export type RefreshTarget =
  | { kind: 'workflows' | 'projects' | 'thread-lists' }
  | { kind: 'design' | 'agreements' | 'agreement-versions' | 'project-workflows'; projectId: string }
  | { kind: 'entries' | 'operations'; threadId: string }

type RefreshEvent = DurableEvent & { projectId?: string }

const PLAN_EVENTS = new Set(['WorkflowDraftStarted', 'WorkflowEdited', 'WorkflowFrozen', 'PlanArchived', 'PlanUnarchived'])
const DESIGN_EVENTS = new Set(['ProjectDesignChanged', 'ProjectStandardsSaved', 'VisionProposed', 'MapProposed', 'RoadmapProposed', 'RoadmapApproved', 'ProjectStructureWritten'])
const REACH_EVENTS = new Set(['PlanDependenciesChanged', 'ProjectLinked', 'ProjectUnlinked', 'ProjectRemoved'])
const TERMINAL_EVENTS = new Set(['OperationCompleted', 'OperationFailed', 'OperationCancelled', 'OperationInterrupted'])

/** Identifiers come from the delivery scope or event payload, never cache state. */
export function classifyRefresh(event: RefreshEvent): RefreshTarget[] {
  const payload = event.payload
  const projectId = event.projectId ?? (typeof payload === 'object' && payload !== null &&
    'project_id' in payload && typeof payload.project_id === 'string' ? payload.project_id : undefined)
  const targets: RefreshTarget[] = []
  if (PLAN_EVENTS.has(event.kind)) {
    targets.push({ kind: 'workflows' })
    if (projectId !== undefined) targets.push({ kind: 'design', projectId }, { kind: 'agreement-versions', projectId })
  }
  if (projectId !== undefined) {
    if (DESIGN_EVENTS.has(event.kind)) targets.push({ kind: 'design', projectId })
    if (event.kind === 'AgreementChanged') targets.push({ kind: 'agreements', projectId })
    if (REACH_EVENTS.has(event.kind)) targets.push({ kind: 'project-workflows', projectId })
  }
  if (event.kind === 'ProjectRemoved') targets.push({ kind: 'projects' })
  if (event.threadId !== null) {
    if (event.kind === 'ThreadEntryAppended' || TERMINAL_EVENTS.has(event.kind)) targets.push({ kind: 'entries', threadId: event.threadId })
    if (TERMINAL_EVENTS.has(event.kind)) targets.push({ kind: 'operations', threadId: event.threadId })
  }
  if (event.kind === 'ThreadRetitled') targets.push({ kind: 'thread-lists' })
  return targets
}

export function dispatchRefresh(client: QueryClient, targets: readonly RefreshTarget[]): void {
  for (const target of targets) {
    switch (target.kind) {
      case 'workflows': invalidateWorkflows(client); break
      case 'project-workflows': invalidateProjectWorkflows(client, target.projectId); break
      case 'design': invalidateDesign(client, target.projectId); break
      case 'agreements': invalidateAgreements(client, target.projectId); break
      case 'agreement-versions': invalidateAgreementVersions(client, target.projectId); break
      case 'projects': invalidateProjects(client); break
      case 'entries': invalidateEntries(client, target.threadId); break
      case 'operations': invalidateOperations(client, target.threadId); break
      case 'thread-lists': invalidateThreadLists(client); break
    }
  }
}

/** Replayed notifications reconcile once at caught-up; transient UI listeners still see every event. */
export function refreshSubscriber(
  client: QueryClient,
  scope: { projectId?: string; threadId?: string; plansOnly?: boolean },
): StreamSubscriber {
  const pending = new Map<string, RefreshTarget>()
  const baseline: RefreshTarget[] = scope.threadId === undefined
    ? [{ kind: 'agreements', projectId: scope.projectId! }, { kind: 'design', projectId: scope.projectId! }, { kind: 'project-workflows', projectId: scope.projectId! }]
    : scope.plansOnly ? [{ kind: 'workflows' }]
    : [{ kind: 'workflows' }, { kind: 'entries', threadId: scope.threadId }, { kind: 'operations', threadId: scope.threadId }]
  return {
    onDurable: (event, live) => {
      let targets = classifyRefresh({ ...event, projectId: scope.projectId, threadId: scope.threadId ?? null })
      const plan = targets.some(target => target.kind === 'workflows')
      if (scope.plansOnly && !plan) return
      if (scope.threadId !== undefined && !plan) {
        targets = targets.filter(target => target.kind === 'entries' || target.kind === 'operations' || target.kind === 'thread-lists')
      }
      if (scope.threadId === undefined) targets = targets.filter(target => target.kind !== 'thread-lists')
      if (live) dispatchRefresh(client, targets)
      else for (const target of targets) {
        // Project design and agreement notices must reach dirty editors during replay.
        if (scope.threadId === undefined && target.kind !== 'workflows' && target.kind !== 'project-workflows' && !PLAN_EVENTS.has(event.kind)) {
          dispatchRefresh(client, [target])
        } else pending.set(JSON.stringify(target), target)
      }
    },
    onCaughtUp: () => {
      const targets = new Map(baseline.map(target => [JSON.stringify(target), target]))
      for (const [key, target] of pending) targets.set(key, target)
      dispatchRefresh(client, [...targets.values()])
      pending.clear()
    },
  }
}
