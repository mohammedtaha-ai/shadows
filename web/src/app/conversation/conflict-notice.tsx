// One job: the copy of an executor's contract conflict inside the Planner's notice, a history card with no controls.

import { Link } from '@tanstack/react-router'
import type { TaskConflict } from '@/api/runs'

/** What the executor reported when the notice was made, linked to the attempt's Report where the current state lives. */
export function ConflictNotice({ conflict, attemptId }: { conflict: TaskConflict; attemptId: string }) {
  return (
    <section aria-label="Contract conflict notice" className="space-y-1 text-xs">
      <p className="font-medium text-warning">Executor reported a contract conflict on T{conflict.task}</p>
      <p><span className="font-medium">Expected</span> <span dir="auto" className="whitespace-pre-wrap break-words">{conflict.expected}</span></p>
      <p><span className="font-medium">Observed</span> <span dir="auto" className="whitespace-pre-wrap break-words">{conflict.observed}</span></p>
      <ul className="list-disc ps-5 text-muted-foreground">
        {conflict.evidence.map((e, i) => (
          <li key={i}>
            <span className="font-mono">{e.line === null ? e.path : `${e.path}:${e.line}`}</span>{' '}
            <span dir="auto" className="whitespace-pre-wrap break-words">{e.summary}</span>
          </li>
        ))}
      </ul>
      <Link
        to="/projects/$projectId/workflows/$workflowId/versions/$version/tasks/$task"
        params={{
          projectId: conflict.project_id, workflowId: conflict.workflow_id,
          version: String(conflict.version), task: String(conflict.task),
        }}
        hash={`attempt-${attemptId}`}
        className="text-accent-line underline underline-offset-2"
      >
        Open the attempt's report
      </Link>
    </section>
  )
}
