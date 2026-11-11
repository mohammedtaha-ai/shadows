// One job: one task's run state as the page reads it, and the three things a
// person does to it (Start, Stop, mark done), each retried under its own key.

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useEffect, useRef } from 'react'
import { type Attempt, type AttemptState, type TaskRun, attemptsQuery, invalidateRuns, markTaskDone, saveTaskRunAnswer,
  startAttempt, stopAttempt, taskRunQuery } from '@/api/runs'
import { ApiError } from '@/api/error'
import { attemptFor, type Attempt as Command } from '@/api/command-id'

const LABELS: Record<AttemptState, string> = {
  running: 'Running', committed: 'Committed', stopped: 'Stopped', interrupted: 'Interrupted', failed: 'Failed',
}

/** The words for where the latest attempt stands. */
export function attemptLabel(attempt: Attempt | null): string {
  return attempt === null ? 'Not started' : LABELS[attempt.state]
}

/** A failed call that got an answer ends the command; only a lost one keeps its key for the retry. */
function answered(error: unknown): boolean {
  return !(error instanceof ApiError) || error.problem.kind !== 'unreachable'
}

export function useTaskRun(workflowId: string, version: number, task: number) {
  const queryClient = useQueryClient()
  const run = useQuery(taskRunQuery(workflowId, version, task))
  const attempts = useQuery(attemptsQuery(workflowId, version, task))
  const startCommand = useRef<Command | null>(null)
  const doneCommand = useRef<Command | null>(null)

  const refresh = () => { invalidateRuns(queryClient) }

  // The Planner can start a new attempt without the page asking: when the latest one changes, read the list again.
  const latestKey = run.data?.latest ? `${run.data.latest.id}:${run.data.latest.state}` : null
  const seenKey = useRef(latestKey)
  useEffect(() => {
    if (seenKey.current === latestKey) return
    seenKey.current = latestKey
    void queryClient.invalidateQueries({ queryKey: attemptsQuery(workflowId, version, task).queryKey })
  }, [latestKey, queryClient, workflowId, version, task])

  const start = useMutation<Attempt, Error, string>({
    mutationFn: (model) => {
      startCommand.current = attemptFor(startCommand.current, { model })
      return startAttempt(workflowId, version, task, startCommand.current.commandId, model)
    },
    onSuccess: () => { startCommand.current = null },
    onError: (error) => { if (answered(error)) startCommand.current = null },
    // Whether it started or was refused, the state moved: read it again.
    onSettled: refresh,
  })

  const stop = useMutation<Attempt, Error, string>({
    mutationFn: stopAttempt,
    onSettled: refresh,
  })

  const done = useMutation<TaskRun, Error, boolean>({
    mutationFn: (value) => {
      doneCommand.current = attemptFor(doneCommand.current, { done: value })
      return markTaskDone(workflowId, version, task, doneCommand.current.commandId, value)
    },
    onSuccess: async (answer) => {
      doneCommand.current = null
      await saveTaskRunAnswer(queryClient, workflowId, version, task, answer)
    },
    onError: (error) => { if (answered(error)) doneCommand.current = null },
  })

  return { run, attempts, start, stop, done }
}
