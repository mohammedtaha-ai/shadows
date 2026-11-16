// One job: Shadows' recheck card, the done tasks whose checks no longer pass, drawn as an
// activity item that links to each task's own version.

import { Link } from '@tanstack/react-router'
import { Bell } from 'lucide-react'
import type { RecheckNotice } from '@/api/runs'
import { CheckRow } from '../check-row'

export function RecheckNoticeItem({ recheck, projectId }: { recheck: RecheckNotice; projectId: string }) {
  return <section aria-label="Recheck" className="surface-panel space-y-2.5 rounded-xl p-3 text-xs">
    <p className="flex items-center gap-2.5 font-medium text-foreground">
      <Bell aria-hidden className="size-3.5 shrink-0 text-accent-line/85" />
      Shadows · recheck after T{recheck.cause_task} (version {recheck.cause_version})
    </p>
    <div className="space-y-2 border-t border-border/60 pt-2.5">
      <p className="text-muted-foreground">These done tasks no longer pass:</p>
      <ul className="space-y-2">
        {recheck.failed.map(failure => <li key={failure.attempt_id} className="space-y-1.5">
          <Link to="/projects/$projectId/workflows/$workflowId/versions/$version/tasks/$task"
            params={{ projectId, workflowId: failure.workflow_id, version: String(failure.version), task: String(failure.task) }}
            className="font-medium text-accent-line underline underline-offset-2">
            <span dir="auto">T{failure.task} · {failure.title}</span>
          </Link>
          <ul className="space-y-2">
            {failure.checks.map(check => <CheckRow key={`${check.for_task}:${check.id}`} check={check} />)}
          </ul>
        </li>)}
      </ul>
      <p className="text-muted-foreground">Their done marks stand.</p>
    </div>
  </section>
}
