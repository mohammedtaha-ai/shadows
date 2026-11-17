// One task's run state and mutations, with replay keys for commands that carry them.

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useEffect, useRef } from 'react'
import { type Attempt, type AttemptState, type ExecutorInput, type TaskRun, attemptsQuery, invalidateExecutorChoices,
  invalidateRuns, markTaskDone, saveTaskRunAnswer, setTaskExecutor, startAttempt, stopAttempt, taskRunQuery } from '@/api/runs'
import { ApiError } from '@/api/error'
import { attemptFor, type Attempt as Command } from '@/api/command-id'

const LABELS: Record<AttemptState, string> = {
  running: 'Running', committed: 'Committed', stopped: 'Stopped', interrupted: 'Interrupted', failed: 'Failed',
}

/** The words for where the latest attempt stands. */
export function attemptLabel(attempt: Attempt | null): string {
  return attempt === null ? 'Not started' : LABELS[attempt.state]
}

const STARTING_READ_MS = 1000

/** A failed call that got an answer ends the command; only a lost one keeps its key for the retry. */
function answered(error: unknown): boolean {
  return !(error instanceof ApiError) || error.problem.kind !== 'unreachable'
}

export function useTaskRun(workflowId: string, version: number, task: number) {
  const queryClient = useQueryClient()
  const startCommand = useRef<Command | null>(null)
  const doneCommand = useRef<Command | null>(null)
  const executorCommand = useRef<Command | null>(null)

  const refresh = () => { invalidateRuns(queryClient) }

  const start = useMutation({
    mutationFn: () => {
      startCommand.current = attemptFor(startCommand.current, {})
      return startAttempt(workflowId, version, task, startCommand.current.commandId)
    },
    onSuccess: () => { startCommand.current = null },
    onError: (error) => { if (answered(error)) startCommand.current = null },
    // Whether it started or was refused, the state moved: read it again.
    onSettled: refresh,
  })
  const executor = useMutation({
    mutationFn: (input: ExecutorInput) => {
      executorCommand.current = attemptFor(executorCommand.current, input)
      return setTaskExecutor(workflowId, version, task, executorCommand.current.commandId, input)
    },
    onSuccess: () => { executorCommand.current = null },
    onError: (error) => { if (answered(error)) executorCommand.current = null },
    onSettled: () => invalidateExecutorChoices(queryClient),
  })

  // Core records the attempt as running before it opens the executor's session, and answers Start only once the
  // session and its first turn are up, which takes seconds: read the task while Start is in flight, so the
  // running attempt shows as soon as Core has it.
  const runOptions = taskRunQuery(workflowId, version, task)
  const run = useQuery(start.isPending ? { ...runOptions, refetchInterval: STARTING_READ_MS } : runOptions)
  const attempts = useQuery(attemptsQuery(workflowId, version, task))

  // The Planner can start a new attempt without the page asking: when the latest one changes, read the list again.
  const latestKey = run.data?.latest ? `${run.data.latest.id}:${run.data.latest.state}` : null
  const seenKey = useRef(latestKey)
  useEffect(() => {
    if (seenKey.current === latestKey) return
    seenKey.current = latestKey
    void queryClient.invalidateQueries({ queryKey: attemptsQuery(workflowId, version, task).queryKey })
  }, [latestKey, queryClient, workflowId, version, task])

  const stop = useMutation<Attempt, Error, string>({
    mutationFn: stopAttempt,
    onSettled: refresh,
  })

  const done = useMutation<TaskRun, Error, { done: boolean; reason?: string }>({
    mutationFn: (value) => {
      doneCommand.current = attemptFor(doneCommand.current, value)
      return markTaskDone(workflowId, version, task, doneCommand.current.commandId, value.done, value.reason)
    },
    onSuccess: async (answer) => {
      doneCommand.current = null
      await saveTaskRunAnswer(queryClient, workflowId, version, task, answer)
    },
    onError: (error) => { if (answered(error)) doneCommand.current = null },
  })

  return { run, attempts, start, stop, done, executor }
}
