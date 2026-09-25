// One job: approving a Draft as the person saw it — what blocks it, the
// button, and why an approval was refused (§13.11, Review Focus 4).

import { useMutation, useQueryClient } from '@tanstack/react-query'
import { CircleAlert, CircleCheck } from 'lucide-react'
import { useRef } from 'react'
import { type Plan, approvePlan } from '@/api/client'
import { type Attempt, attemptFor } from '@/api/command-id'
import { ApiError } from '@/api/error'
import { workflowsKey } from '@/api/queries'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'

export const CHANGED_WHILE_LOOKING = 'The plan changed while you were looking; review it again'

/** Approve, with the validator's list above it. It stays enabled with
 * blockers: pressed anyway, the daemon's `422` lists what is missing. */
export function ApproveBar({ plan }: { plan: Plan }) {
  const queryClient = useQueryClient()
  const attempt = useRef<Attempt | null>(null)
  const refetch = () => void queryClient.invalidateQueries({ queryKey: workflowsKey })

  const approve = useMutation({
    mutationFn: ({ commandId, revision }: { commandId: string; revision: number }) =>
      approvePlan(plan.id, commandId, revision),
    // The answer is an `Approved`, not the plan: read the plan again.
    onSuccess: () => {
      attempt.current = null
      refetch()
    },
    // A plan that moved on (a new revision, or approved elsewhere) is read
    // again, so what is on screen is what the next press approves.
    onError: (error) => {
      const code = daemonCode(error)
      if (code === 'REVISION_CONFLICT' || code === 'WORKFLOW_FROZEN_IMMUTABLE') refetch()
    },
  })

  const press = () => {
    if (approve.isPending) return
    // The same revision pressed again after a lost answer is a retry of the
    // same command; the plan at another revision is a new one.
    attempt.current = attemptFor(attempt.current, { workflow: plan.id, revision: plan.revision })
    approve.mutate({ commandId: attempt.current.commandId, revision: plan.revision })
  }

  return (
    <section
      aria-label="Approval"
      className="flex items-start justify-between gap-4 border-b border-border px-6 py-3"
    >
      <div className="min-w-0 space-y-2 text-xs">
        {plan.blockers.length === 0 ? (
          <p className="flex items-center gap-1.5 text-muted-foreground">
            <CircleCheck className="size-3.5 text-success" aria-hidden />
            Nothing blocks approval.
          </p>
        ) : (
          <div>
            <p className="mb-1 flex items-center gap-1.5 text-muted-foreground">
              <CircleAlert className="size-3.5 text-destructive-foreground" aria-hidden />
              Before it can be approved:
            </p>
            <ul className="ml-5 list-disc space-y-0.5 text-foreground">
              {plan.blockers.map((b) => (
                <li key={b.message} dir="auto">
                  {b.message}
                </li>
              ))}
            </ul>
          </div>
        )}
        {approve.error !== null && <Refusal error={approve.error} />}
      </div>
      <Button onClick={press} disabled={approve.isPending} className="shrink-0">
        {approve.isPending ? 'Approving…' : 'Approve'}
      </Button>
    </section>
  )
}

function Refusal({ error }: { error: Error }) {
  const code = daemonCode(error)
  if (code === 'REVISION_CONFLICT') {
    return (
      <p role="alert" className="text-destructive-foreground">
        {CHANGED_WHILE_LOOKING}
      </p>
    )
  }
  const problems =
    error instanceof ApiError && error.problem.kind === 'daemon' ? error.problem.problems : undefined
  if (code === 'WORKFLOW_VALIDATION_FAILED' && problems !== undefined) {
    return (
      <div role="alert" className="text-destructive-foreground">
        <p>Not approved:</p>
        <ul className="ml-5 list-disc space-y-0.5">
          {problems.map((p) => (
            <li key={p} dir="auto">
              {p}
            </li>
          ))}
        </ul>
      </div>
    )
  }
  return <ErrorLine error={error} />
}

function daemonCode(error: Error): string | null {
  return error instanceof ApiError && error.problem.kind === 'daemon' ? error.problem.code : null
}
