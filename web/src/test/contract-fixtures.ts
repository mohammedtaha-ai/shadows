// One job: sample values of the daemon's contract, typed against
// `schema.d.ts`, so a contract change breaks the tests that use them at
// compile time rather than at run time. Test-only.
//
// The models, efforts and modes are made up, as the fake agent's are: no test
// of product behaviour names a real Claude model (plan, Global Constraints).

import type {
  Choice,
  Grant,
  HarnessInfo,
  InvocationView,
  Operation,
  Plan,
  PlanListing,
  PlanTask,
  PlanVersions,
  PlanningThread,
  Project,
  ProjectStatus,
  SessionChoices,
  ThreadEntry,
} from '@/api/client'

/** One offered value; the label defaults to the id. */
export function choice(id: string, label = id): Choice {
  return { id, label, description: null, enabled: true, reason: null }
}

export const claudeHarness: HarnessInfo = {
  kind: 'claude-code',
  label: 'Claude Code',
  available: true,
  reason: null,
  remembered: null,
  limits: null,
}

export const codexHarness: HarnessInfo = {
  kind: 'codex',
  label: 'Codex',
  available: false,
  reason: 'Codex is not runnable yet',
  remembered: null,
  limits: null,
}

export const fakeChoices: SessionChoices = {
  models: [choice('fake-small'), choice('fake-large')],
  efforts: [choice('low'), choice('high'), choice('max')],
  modes: [choice('acceptEdits', 'Accept edits'), choice('auto', 'Auto')],
  current: { model: 'fake-large', mode: 'acceptEdits', effort: 'high' },
}

export function projectWithModes(modes: Record<string, string[]>): Project {
  return {
    id: 'p1',
    slug: 'demo',
    name: 'Demo',
    directory: 'C:\\work\\demo',
    created_at: '2026-09-24T00:00:00Z',
    allowed_modes: modes,
  }
}

export const projectFixture: Project = projectWithModes({ 'claude-code': ['acceptEdits', 'auto'] })

export const threadFixture: PlanningThread = {
  id: 't1',
  project_id: 'p1',
  title: 'Conversation 1',
  status: 'Open',
  created_at: '2026-09-24T00:00:00Z',
  harness: 'claude-code',
  forked_from_thread: null,
  removed_at: null,
}

/** A project's code index, ready, by its slug. */
export function codeStatusFixture(project: string, extra: Partial<ProjectStatus> = {}): ProjectStatus {
  return {
    project,
    state: { state: 'ready' },
    files: 277,
    skipped: [],
    updated_at: '2026-09-24T00:00:00Z',
    ...extra,
  }
}

/** A live grant bound to project `p1`, for an external agent. */
export function grantFixture(id: string, extra: Partial<Grant> = {}): Grant {
  return {
    id,
    kind: 'project',
    project_id: 'p1',
    thread_id: null,
    created_at: '2026-09-24T00:00:00Z',
    revoked_at: null,
    ...extra,
  }
}

export const invocationFixture: InvocationView = {
  harness_kind: 'claude-code',
  harness_version: '0.0.0-fake',
  agent_version: '0.0.0-fake',
  requested_model: 'fake-large',
  requested_mode: 'acceptEdits',
  requested_effort: 'high',
  observed_model: 'fake-large',
  context_used: null,
  context_window: null,
}

function operation(status_kind: string, invocation: InvocationView | null): Operation {
  return {
    id: 'op1',
    kind: 'PlannerTurn',
    status_kind,
    thread_id: 't1',
    runtime_instance_id: 'r1',
    created_at: '2026-09-24T00:00:00Z',
    invocation,
  }
}

/** Turn `op1`, completed, with the invocation it recorded. */
export function completedOperation(invocation: InvocationView | null): Operation {
  return operation('Completed', invocation)
}

/** Turn `op1`, still running. */
export function runningOperation(): Operation {
  return operation('Running', invocationFixture)
}

let ordinal = 0

/** An entry of any kind, written by turn `op1` unless `operationId` says otherwise. */
export function entryOfKind(
  id: string,
  kind: ThreadEntry['kind'],
  body: string,
  operationId: string | null = 'op1',
): ThreadEntry {
  ordinal += 1
  return {
    id,
    thread_id: 't1',
    ordinal,
    kind,
    author: { kind: kind === 'UserMessage' ? 'User' : 'Agent', id: 'local' },
    body,
    refs: [],
    card: null,
    created_at: '2026-09-24T00:00:00Z',
    operation_id: operationId,
  }
}

export const userEntry = (id: string, body: string) => entryOfKind(id, 'UserMessage', body)
export const agentEntry = (id: string, body: string) => entryOfKind(id, 'AgentMessage', body)
/** A tool line (§23.8): its body is the tool's title. */
export const toolEntry = (id: string, title: string) => entryOfKind(id, 'ToolCall', title)
/** A subagent (§22.2, §23.8): its body is the card's title, its card in `card`. */
export const subagentEntry = (id: string, card: { title: string } & Record<string, unknown>) => ({
  ...entryOfKind(id, 'Subagent', card.title),
  card,
})

/** Task `T{number}` of a plan, with one acceptance item. */
export function planTask(number: number, title: string, extra: Partial<PlanTask> = {}): PlanTask {
  return {
    id: `task-${number}`,
    number,
    title,
    goal: `The goal of ${title}`,
    reads: [],
    writes: [],
    acceptance: [{ number: 1, text: `${title} works` }],
    ...extra,
  }
}

/** Plan version `w1` of thread `t1`: a Draft of two tasks, T2 needing T1. */
export function planFixture(extra: Partial<Plan> = {}): Plan {
  return {
    id: 'w1',
    plan_id: 'plan1',
    plan_state: 'Active',
    project_id: 'p1',
    written_by: {
      kind: 'planner',
      thread_id: 't1',
      thread_title: 'Login',
      thread_removed: false,
      model: null,
      harness: null,
    },
    change_reason: null,
    title: 'Login flow',
    goal: 'People can sign in',
    state: 'Draft',
    version: 1,
    revision: 3,
    created_at: '2026-09-25T00:00:00Z',
    frozen_at: null,
    previous: null,
    next: null,
    blockers: [],
    last_edit: null,
    tasks: [planTask(1, 'Schema'), planTask(2, 'Login screen')],
    links: [{ task: 2, after: 1, kind: 'needs', label: 'the users table', waiting_items: [] }],
    ...extra,
  }
}

export function planListing(plan: Plan): PlanListing {
  return {
    id: plan.id,
    plan_id: plan.plan_id,
    plan_state: plan.plan_state,
    title: plan.title,
    state: plan.state,
    version: plan.version,
    updated_at: plan.created_at,
  }
}

export function planVersionsFixture(plan: Plan, versions?: PlanVersions['versions']): PlanVersions {
  return {
    plan_id: plan.plan_id,
    project_id: plan.project_id,
    state: plan.plan_state,
    archived_at: plan.plan_state === 'Archived' ? '2026-10-01T00:00:00Z' : null,
    versions:
      versions && versions.length > 0
        ? versions
        : [
            {
              workflow_id: plan.id,
              version: plan.version,
              state: plan.state,
              title: plan.title,
              written_by: plan.written_by,
              change_reason: plan.change_reason,
              created_at: plan.created_at,
            },
          ],
  }
}

/** A `PlanView` card (§13.9) of plan version `workflowId`, about task `taskId`
 * when given; its body is what the daemon writes. */
export function planViewEntry(
  id: string,
  body: string,
  workflowId: string,
  taskId?: string,
): ThreadEntry {
  const entry = entryOfKind(id, 'PlanView', body)
  const refs: ThreadEntry['refs'] = [{ Workflow: workflowId }]
  if (taskId !== undefined) refs.push({ Task: taskId })
  return { ...entry, refs }
}
