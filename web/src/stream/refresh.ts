import type { QueryClient } from '@tanstack/react-query'
import { invalidateWorkflows, invalidateProjectWorkflows } from '@/api/workflows'
import { invalidateDesign } from '@/api/design'
import { invalidateAgreements, invalidateAgreementVersions } from '@/api/agreements'
import { invalidateProjects } from '@/api/projects'
import { invalidateEntries, invalidateOperations, invalidateThreadLists } from '@/api/threads'
import type { DurableEvent } from './frames'
import { subscribeProjectStream, subscribeThreadStream, type StreamSubscriber } from './thread-registry'

export type RefreshTarget =
  | { kind: 'workflows' | 'projects' | 'thread-lists' }
  | { kind: 'design' | 'agreements' | 'agreement-versions' | 'project-workflows'; projectId: string }
  | { kind: 'entries' | 'operations'; threadId: string }

type RefreshEvent = DurableEvent & { projectId?: string }

const PLAN_EVENTS = new Set(['WorkflowDraftStarted', 'WorkflowEdited', 'WorkflowFrozen', 'PlanArchived', 'PlanUnarchived'])
const DESIGN_EVENTS = new Set(['ProjectDesignChanged', 'ProjectStandardsSaved', 'VisionProposed', 'MapProposed', 'RoadmapProposed', 'RoadmapApproved', 'ProjectStructureWritten'])
const REACH_EVENTS = new Set(['PlanDependenciesChanged', 'ProjectLinked', 'ProjectUnlinked', 'ProjectRemoved'])

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
    // Which operation events end a turn is Core's lifecycle; every one refreshes rather than
    // the client keeping a copy of the terminal kinds.
    const operation = event.kind.startsWith('Operation')
    if (event.kind === 'ThreadEntryAppended' || operation) targets.push({ kind: 'entries', threadId: event.threadId })
    if (operation) targets.push({ kind: 'operations', threadId: event.threadId })
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

interface RefreshScope { projectId?: string; threadId?: string; plansOnly?: boolean }

function baseline(scope: RefreshScope): RefreshTarget[] {
  return scope.threadId === undefined
    ? [{ kind: 'agreements', projectId: scope.projectId! }, { kind: 'design', projectId: scope.projectId! }, { kind: 'project-workflows', projectId: scope.projectId! }]
    : scope.plansOnly ? [{ kind: 'workflows' }]
    : [{ kind: 'workflows' }, { kind: 'entries', threadId: scope.threadId }, { kind: 'operations', threadId: scope.threadId }]
}

/** Replayed notifications reconcile once at caught-up; transient UI listeners still see every event. */
function refreshSubscriber(client: QueryClient, current: () => RefreshScope): StreamSubscriber {
  const pending = new Map<string, RefreshTarget>()
  return {
    onDurable: (event, live) => {
      const scope = current()
      let targets = classifyRefresh({ ...event, projectId: scope.projectId, threadId: scope.threadId ?? null })
      const plan = targets.some(target => target.kind === 'workflows')
      if (scope.plansOnly && !plan) return
      if (scope.threadId !== undefined && !plan) {
        targets = targets.filter(target => target.kind === 'entries' || target.kind === 'operations' || target.kind === 'thread-lists')
      }
      if (scope.threadId === undefined) targets = targets.filter(target => target.kind !== 'thread-lists')
      if (live) dispatchRefresh(client, targets)
      // A replay of N events refetching N times would cancel and restart each read it touches.
      else for (const target of targets) pending.set(JSON.stringify(target), target)
    },
    onCaughtUp: () => {
      const targets = new Map(baseline(current()).map(target => [JSON.stringify(target), target]))
      for (const [key, target] of pending) targets.set(key, target)
      dispatchRefresh(client, [...targets.values()])
      pending.clear()
    },
  }
}

interface SharedRefresh { members: RefreshScope[]; live: boolean; release: () => void }

// Many views watch one stream (every plan card of a conversation, every view of
// a project); refreshing per view would refetch the same queries once per view.
const refreshes = new WeakMap<QueryClient, Map<string, SharedRefresh>>()

function hold(
  client: QueryClient, key: string, member: RefreshScope,
  effective: (members: readonly RefreshScope[]) => RefreshScope,
  subscribe: (subscriber: StreamSubscriber) => () => void,
): () => void {
  let scope = refreshes.get(client)
  if (!scope) { scope = new Map(); refreshes.set(client, scope) }
  let shared = scope.get(key)
  if (shared) {
    const before = new Set(baseline(effective(shared.members)).map(target => JSON.stringify(target)))
    shared.members.push(member)
    // A live stream sends no caught-up to a joiner; only what it adds is unwatched so far.
    if (shared.live) {
      dispatchRefresh(client, baseline(effective(shared.members)).filter(target => !before.has(JSON.stringify(target))))
    }
  } else {
    const created: SharedRefresh = { members: [member], live: false, release: () => {} }
    const subscriber = refreshSubscriber(client, () => effective(created.members))
    scope.set(key, created)
    created.release = subscribe({ ...subscriber, onState: state => { created.live = state.connection === 'live' } })
    shared = created
  }
  const held = shared
  const map = scope
  let released = false
  return () => {
    if (released) return
    released = true
    held.members.splice(held.members.indexOf(member), 1)
    if (held.members.length === 0) {
      if (map.get(key) === held) map.delete(key)
      held.release()
    }
  }
}

/** Keeps a thread's views fresh while held; holders of one thread share one refresh. */
export function holdThreadRefresh(
  client: QueryClient, threadId: string, member: { projectId?: string; plansOnly?: boolean },
): () => void {
  return hold(client, `thread:${threadId}`, { ...member }, members => ({
    threadId,
    projectId: members.find(held => held.projectId !== undefined)?.projectId,
    plansOnly: members.every(held => held.plansOnly === true),
  }), subscriber => subscribeThreadStream(client, threadId, subscriber))
}

/** Keeps a project's views fresh while held; holders of one project share one refresh. */
export function holdProjectRefresh(client: QueryClient, projectId: string): () => void {
  return hold(client, `project:${projectId}`, {}, () => ({ projectId }),
    subscriber => subscribeProjectStream(client, projectId, subscriber))
}
