import { queryOptions, type QueryClient } from '@tanstack/react-query'
import { writeSaved } from './saved-answer'
import type { components } from './schema'
import { client, unwrap, ApiError } from './transport'

export type TaskRun = components['schemas']['TaskRun']
export type Attempt = components['schemas']['Attempt']
export type AttemptState = components['schemas']['AttemptState']
type FileDiff = components['schemas']['FileDiff']
export type AttemptFile = components['schemas']['AttemptFile']
export type DiffHunk = components['schemas']['DiffHunk']
export type Need = components['schemas']['Need']
type RunRefusal = components['schemas']['RunRefusal']

const runKeys = {
  all: () => ['runs'] as const,
  task: (workflowId: string, version: number, task: number) => ['runs', 'task', workflowId, version, task],
  attempts: (workflowId: string, version: number, task: number) => ['runs', 'attempts', workflowId, version, task],
  diff: (attemptId: string) => ['runs', 'diff', attemptId],
  file: (attemptId: string, path: string) => ['runs', 'file', attemptId, path],
}

/** Core's answer, even a refusal such as a task it cannot find, is final; only a lost call is tried again. */
function retryLost(failures: number, error: Error): boolean {
  return failures < 3 && error instanceof ApiError && error.problem.kind === 'unreachable'
}

/** A task's run state: its marks, latest attempt, needs and whether Start may run. */
export function taskRunQuery(workflowId: string, version: number, task: number) {
  return queryOptions({
    queryKey: runKeys.task(workflowId, version, task),
    queryFn: () => getTaskRun(workflowId, version, task),
    retry: retryLost,
    // A running attempt, or a committed one the Planner has not judged, changes without the page asking.
    refetchInterval: (query) => {
      const run = query.state.data
      return run !== undefined && (run.latest?.state === 'running' || awaitingJudgment(run)) ? 2000 : false
    },
  })
}

/** Whether the task waits for the Planner's judgment: its latest attempt is committed, the task is not marked done and Core
 * still refuses Start as busy. A task stopped on the Planner's cap is committed and not done but may be started, so it does not wait.
 * Read as the planner review request states it (docs/shared/requests/2026-11-11-planner-review.md). */
export function awaitingJudgment(run: TaskRun): boolean {
  return run.latest !== null && run.latest.state === 'committed' && !run.done && run.start.code === 'task_busy'
}

/** Core frees Start when the latest committed addition stops on the Planner's cap. */
export function stoppedOnAdditionCap(run: TaskRun): boolean {
  return run.latest !== null && run.latest.state === 'committed' && run.latest.addition !== null && !run.done && run.start.allowed
}

/** The task's attempts, newest first, read again while the newest one runs. */
export function attemptsQuery(workflowId: string, version: number, task: number) {
  return queryOptions({
    queryKey: runKeys.attempts(workflowId, version, task),
    queryFn: () => listAttempts(workflowId, version, task),
    retry: retryLost,
    refetchInterval: (query) => (query.state.data?.[0]?.state === 'running' ? 2000 : false),
  })
}

export function attemptDiffQuery(attemptId: string) {
  return queryOptions({ queryKey: runKeys.diff(attemptId), queryFn: () => getAttemptDiff(attemptId) })
}

export function attemptFileQuery(attemptId: string, path: string) {
  return queryOptions({ queryKey: runKeys.file(attemptId, path), queryFn: () => getAttemptFile(attemptId, path) })
}

export function saveTaskRunAnswer(queryClient: QueryClient, workflowId: string, version: number, task: number, run: TaskRun) {
  return writeSaved(queryClient, runKeys.task(workflowId, version, task), run)
}

export function invalidateRuns(queryClient: QueryClient): void {
  void queryClient.invalidateQueries({ queryKey: runKeys.all() })
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

/** Starts an attempt on `model`. A refusal is an `ApiError` whose message is Core's reason. */
export async function startAttempt(
  workflowId: string, version: number, task: number, commandId: string, model: string,
): Promise<Attempt> {
  try {
    return await unwrap(client.POST('/api/workflows/{workflow}/versions/{version}/tasks/{task}/attempts', {
      params: { path: { workflow: workflowId, version, task } },
      body: { command_id: commandId, model },
    }))
  } catch (error) {
    throw withRefusalReason(error)
  }
}

export function stopAttempt(attemptId: string): Promise<Attempt> {
  return unwrap(client.POST('/api/attempts/{id}/stop', { params: { path: { id: attemptId } } }))
}

/** Marks the task done or not done. */
export function markTaskDone(
  workflowId: string, version: number, task: number, commandId: string, done: boolean,
): Promise<TaskRun> {
  return unwrap(client.PUT('/api/workflows/{workflow}/versions/{version}/tasks/{task}/done', {
    params: { path: { workflow: workflowId, version, task } },
    body: { command_id: commandId, done },
  }))
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
