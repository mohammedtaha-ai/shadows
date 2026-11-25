// One job: Shadows' recheck card, the done tasks whose checks no longer pass, drawn as an
// activity item that names each task's plan and links to the task's own version.

import { useQuery } from '@tanstack/react-query'
import { Link } from '@tanstack/react-router'
import { Bell, CircleCheck } from 'lucide-react'
import type { RecheckNotice } from '@/api/runs'
import { planQuery } from '@/api/workflows'
import { CheckRow } from '../check-row'

/** A recheck covers every plan of the project, so each failure says which plan it belongs to; until
 * the plan is read, or if it cannot be, the version alone. */
function PlanName({ workflowId, version }: { workflowId: string; version: number }) {
  const plan = useQuery(planQuery(workflowId))
  return <p className="text-muted-foreground">
    {plan.data !== undefined && <><span dir="auto" className="text-secondary-foreground">{plan.data.title}</span>, </>}
    version {version}
  </p>
}

export function RecheckNoticeItem({ recheck, projectId, body = '' }: { recheck: RecheckNotice; projectId: string; body?: string }) {
  // Core sends the all-passed notice with an empty failed list and its own sentence as the entry body.
  if (recheck.failed.length === 0) {
    return <section aria-label="Recheck passed" className="surface-panel space-y-2 rounded-xl border border-success/30 bg-success/10 p-3 text-xs">
      <p className="flex items-center gap-2.5 font-medium text-success">
        <CircleCheck aria-hidden className="size-3.5 shrink-0" />
        Project recheck passed
      </p>
      {body !== '' && <p dir="auto" className="whitespace-pre-wrap text-secondary-foreground">{body}</p>}
    </section>
  }
  return <section aria-label="Recheck" className="surface-panel space-y-2.5 rounded-xl p-3 text-xs">
    <p className="flex items-center gap-2.5 font-medium text-foreground">
      <Bell aria-hidden className="size-3.5 shrink-0 text-accent-line/85" />
      Shadows · recheck after T{recheck.cause_task} (version {recheck.cause_version})
    </p>
    <div className="space-y-2 border-t border-border/60 pt-2.5">
      <p className="text-muted-foreground">These done tasks no longer pass:</p>
      <ul className="space-y-2">
        {recheck.failed.map(failure => <li key={failure.attempt_id} className="space-y-1.5">
          <PlanName workflowId={failure.workflow_id} version={failure.version} />
          <Link to="/projects/$projectId/workflows/$workflowId/versions/$version/tasks/$task"
            params={{ projectId, workflowId: failure.workflow_id, version: String(failure.version), task: String(failure.task) }}
            className="font-medium text-accent-line underline underline-offset-2">
            <span dir="auto">T{failure.task} · {failure.title}</span>
          </Link>
          <ul className="space-y-2">
            {failure.checks.map(check => <CheckRow key={`${check.for_task}:${check.id}`} check={check} reported={false} />)}
          </ul>
        </li>)}
      </ul>
      <p className="text-muted-foreground">Their done marks stand.</p>
    </div>
  </section>
}
