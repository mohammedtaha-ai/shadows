// One job: sample values of the daemon's contract, typed against
// `schema.d.ts`, so a contract change breaks the tests that use them at
// compile time rather than at run time. Test-only.
//
// The models, efforts and modes are made up, as the fake agent's are: no test
// of product behaviour names a real Claude model (plan, Global Constraints).

import type {
  Choice,
  HarnessInfo,
  InvocationView,
  Operation,
  PlanningThread,
  Project,
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
  kind: string,
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
    created_at: '2026-09-24T00:00:00Z',
    operation_id: operationId,
  }
}

export const userEntry = (id: string, body: string) => entryOfKind(id, 'UserMessage', body)
export const agentEntry = (id: string, body: string) => entryOfKind(id, 'AgentMessage', body)
