import { queryOptions, type QueryClient } from '@tanstack/react-query'
import { writeSaved } from './saved-answer'
import type { components } from './schema'
import { client, DAEMON_URL, unwrap, ApiError } from './transport'
import { retryLost } from './error'

export type TaskRun = components['schemas']['TaskRun']
export type ExecutorChoice = components['schemas']['ExecutorChoice']
export type ExecutorInput = components['schemas']['ExecutorInput']
export type Attempt = components['schemas']['Attempt']
export type AttemptState = components['schemas']['AttemptState']
type FileDiff = components['schemas']['FileDiff']
export type AttemptFile = components['schemas']['AttemptFile']
export type AttemptNotice = components['schemas']['AttemptNotice']
export type AgentReplyNotice = components['schemas']['AgentReplyNotice']
export type RecheckNotice = components['schemas']['RecheckNotice']
export type RecheckFailure = components['schemas']['RecheckFailure']
export type RecheckRow = components['schemas']['RecheckRow']
export type CheckOutcome = components['schemas']['CheckOutcome']
export type LastRecheck = components['schemas']['LastRecheck']
export type AgentNotice = AttemptNotice | AgentReplyNotice
export type CheckResult = components['schemas']['CheckResult']
export type DiffHunk = components['schemas']['DiffHunk']
export type Need = components['schemas']['Need']
export type AttemptBrief = components['schemas']['AttemptBrief']
export type TaskConflict = components['schemas']['TaskConflict']
export type ConflictEvidence = components['schemas']['ConflictEvidence']
export type ConsumerRef = components['schemas']['ConsumerRef']
type RunRefusal = components['schemas']['RunRefusal']

const runKeys = {
  all: () => ['runs'] as const,
  tasks: () => ['runs', 'task'] as const,
  task: (workflowId: string, version: number, task: number) => ['runs', 'task', workflowId, version, task],
  attempts: (workflowId: string, version: number, task: number) => ['runs', 'attempts', workflowId, version, task],
  diff: (attemptId: string) => ['runs', 'diff', attemptId],
  file: (attemptId: string, path: string) => ['runs', 'file', attemptId, path],
  brief: (attemptId: string) => ['runs', 'brief', attemptId],
  conflict: (attemptId: string) => ['runs', 'conflict', attemptId],
  checks: (attemptId: string) => ['runs', 'checks', attemptId],
  recheck: (workflowId: string) => ['runs', 'recheck', workflowId],
  recheckRows: (recheckId: string) => ['runs', 'recheck-rows', recheckId],
}

/** A task's run state: its marks, latest attempt, needs and whether Start may run. */
export function taskRunQuery(workflowId: string, version: number, task: number) {
  return queryOptions({
    queryKey: runKeys.task(workflowId, version, task),
    queryFn: () => getTaskRun(workflowId, version, task),
    retry: retryLost,
  })
}

/** Whether the task waits for the Planner's judgment: its latest attempt is committed, the task is not marked done and Core
 * still refuses Start as busy. A task stopped on the Planner's cap is committed and not done but may be started, so it does not wait. */
export function awaitingJudgment(run: TaskRun): boolean {
  return run.latest !== null && run.latest.state === 'committed' && !run.done && run.start.code === 'task_busy'
}

/** Core frees Start when the latest committed addition stops on the Planner's cap. */
export function stoppedOnAdditionCap(run: TaskRun): boolean {
  return run.latest !== null && run.latest.state === 'committed' && run.latest.origin === 'planner' && run.latest.addition !== null && !run.done && run.start.allowed
}

/** The task's attempts, newest first; TaskProgressChanged on the project stream refreshes it. */
export function attemptsQuery(workflowId: string, version: number, task: number) {
  return queryOptions({
    queryKey: runKeys.attempts(workflowId, version, task),
    queryFn: () => listAttempts(workflowId, version, task),
    retry: retryLost,
  })
}

export function attemptDiffQuery(attemptId: string) {
  return queryOptions({ queryKey: runKeys.diff(attemptId), queryFn: () => getAttemptDiff(attemptId) })
}

/** The project's newest recheck batch, or null when it has had none; RecheckStarted and RecheckFinished on the
 * project stream refresh it. */
export function lastRecheckQuery(workflowId: string) {
  return queryOptions({
    queryKey: runKeys.recheck(workflowId),
    queryFn: () => unwrap(client.GET('/api/workflows/{workflow}/recheck', { params: { path: { workflow: workflowId } } })),
    retry: retryLost,
  })
}

/** One recheck batch's rows in Core's order, with the stored output of each finished row. */
export function recheckRowsQuery(recheckId: string) {
  return queryOptions({
    queryKey: runKeys.recheckRows(recheckId),
    queryFn: () => unwrap(client.GET('/api/rechecks/{recheck}', { params: { path: { recheck: recheckId } } })),
    retry: retryLost,
  })
}

/** The address of a batch's live output; `EventSource` in `stream/` owns the connection. */
export function recheckOutputUrl(recheckId: string): string {
  return new URL(`/api/rechecks/${encodeURIComponent(recheckId)}/output`, DAEMON_URL).toString()
}

/** Asks Core to rerun the checks of the project's done tasks; it answers at once and runs in the background. */
export function recheckProject(workflowId: string, commandId: string) {
  return unwrap(client.POST('/api/workflows/{workflow}/recheck', {
    params: { path: { workflow: workflowId } },
    body: { command_id: commandId },
  }))
}

/** An attempt's check results; TaskProgressChanged on the project stream refreshes them. */
export function attemptChecksQuery(id: string) {
  return queryOptions({
    queryKey: runKeys.checks(id),
    queryFn: () => unwrap(client.GET('/api/attempts/{id}/checks', { params: { path: { id } } })),
    retry: retryLost,
  })
}

export function attemptFileQuery(attemptId: string, path: string) {
  return queryOptions({ queryKey: runKeys.file(attemptId, path), queryFn: () => getAttemptFile(attemptId, path) })
}

/** The context the attempt received, as Core saved it. An attempt from before contexts were saved answers
 * context_not_recorded, which is a final answer and not read again. */
export function attemptBriefQuery(id: string) {
  return queryOptions({
    queryKey: runKeys.brief(id),
    queryFn: () => unwrap(client.GET('/api/attempts/{id}/brief', { params: { path: { id } } })),
    retry: retryLost,
  })
}

/** The contract conflict the executor reported, or null; TaskProgressChanged on the project stream refreshes it. */
export function attemptConflictQuery(id: string) {
  return queryOptions({
    queryKey: runKeys.conflict(id),
    queryFn: () => unwrap(client.GET('/api/attempts/{id}/conflict', { params: { path: { id } } })),
    retry: retryLost,
  })
}

/** Whether Core refused a brief read because the attempt began before contexts were saved. */
export function contextNotRecorded(error: unknown): boolean {
  if (!(error instanceof ApiError) || error.problem.kind !== 'http') return false
  try {
    return (JSON.parse(error.problem.body) as Partial<RunRefusal>).code === 'context_not_recorded'
  } catch { return false }
}

export function saveTaskRunAnswer(queryClient: QueryClient, workflowId: string, version: number, task: number, run: TaskRun) {
  return writeSaved(queryClient, runKeys.task(workflowId, version, task), run)
}

export function invalidateRuns(queryClient: QueryClient): void {
  void queryClient.invalidateQueries({ queryKey: runKeys.all() })
}

/** One task's run and attempt list, as TaskProgressChanged names them. */
export function invalidateTaskRun(queryClient: QueryClient, workflowId: string, version: number, task: number): void {
  void queryClient.invalidateQueries({ queryKey: runKeys.task(workflowId, version, task) })
  void queryClient.invalidateQueries({ queryKey: runKeys.attempts(workflowId, version, task) })
}

/** One attempt's check results and conflict, as TaskProgressChanged names the attempt. */
export function invalidateAttemptReads(queryClient: QueryClient, attemptId: string): void {
  void queryClient.invalidateQueries({ queryKey: runKeys.checks(attemptId) })
  void queryClient.invalidateQueries({ queryKey: runKeys.conflict(attemptId) })
}

/** A finished row, a batch's start or its end refreshes that workflow's newest-batch read and that batch's rows, nothing else. */
export function invalidateRecheck(queryClient: QueryClient, workflowId: string, recheckId: string): void {
  void queryClient.invalidateQueries({ queryKey: runKeys.recheck(workflowId) })
  void queryClient.invalidateQueries({ queryKey: runKeys.recheckRows(recheckId) })
}

export function invalidateExecutorChoices(queryClient: QueryClient): Promise<void> {
  return queryClient.invalidateQueries({ queryKey: runKeys.tasks() })
}

function getTaskRun(workflowId: string, version: number, task: number): Promise<TaskRun> {
  return unwrap(client.GET('/api/workflows/{workflow}/versions/{version}/tasks/{task}', {
    params: { path: { workflow: workflowId, version, task } },
  }))
}

/** The task's attempts, newest first. */
function listAttempts(workflowId: string, version: number, task: number): Promise<Attempt[]> {
  return unwrap(client.GET('/api/workflows/{workflow}/versions/{version}/tasks/{task}/attempts', {
    params: { path: { workflow: workflowId, version, task } },
  }))
}

/** Starts an attempt on the saved choice. */
export async function startAttempt(
  workflowId: string, version: number, task: number, commandId: string,
): Promise<Attempt> {
  try {
    return await unwrap(client.POST('/api/workflows/{workflow}/versions/{version}/tasks/{task}/attempts', {
      params: { path: { workflow: workflowId, version, task } },
      body: { command_id: commandId },
    }))
  } catch (error) {
    throw withRefusalReason(error)
  }
}

export async function setTaskExecutor(
  workflowId: string, version: number, task: number, commandId: string, input: ExecutorInput,
): Promise<ExecutorChoice> {
  try {
    return await unwrap(client.PUT('/api/workflows/{workflow}/versions/{version}/tasks/{task}/executor', {
      params: { path: { workflow: workflowId, version, task } },
      body: { command_id: commandId, ...input },
    }))
  } catch (error) { throw withRefusalReason(error) }
}

export function stopAttempt(attemptId: string): Promise<Attempt> {
  return unwrap(client.POST('/api/attempts/{id}/stop', { params: { path: { id: attemptId } } }))
}

/** Marks the task done or not done. */
export async function markTaskDone(
  workflowId: string, version: number, task: number, commandId: string, done: boolean, reason?: string,
): Promise<TaskRun> {
  try {
    return await unwrap(client.PUT('/api/workflows/{workflow}/versions/{version}/tasks/{task}/done', {
      params: { path: { workflow: workflowId, version, task } },
      body: { command_id: commandId, done, reason },
    }))
  } catch (error) { throw withRefusalReason(error) }
}

/** Each file of the attempt's commit with its counts and patch. */
function getAttemptDiff(attemptId: string): Promise<FileDiff[]> {
  return unwrap(client.GET('/api/attempts/{id}/diff', { params: { path: { id: attemptId } } }))
}

/** One file as the attempt's commit holds it. */
function getAttemptFile(attemptId: string, path: string): Promise<AttemptFile> {
  return unwrap(client.GET('/api/attempts/{id}/files', { params: { path: { id: attemptId }, query: { path } } }))
}

/** A refused run answers `{ code, reason }`, which is not the daemon's usual
 * error body; the same error then says Core's reason. */
function withRefusalReason(error: unknown): unknown {
  if (!(error instanceof ApiError) || error.problem.kind !== 'http') return error
  try {
    const body = JSON.parse(error.problem.body) as Partial<RunRefusal>
    if (typeof body.reason === 'string') return new ApiError(error.problem, body.reason, { cause: error })
  } catch { /* not a refusal */ }
  return error
}
