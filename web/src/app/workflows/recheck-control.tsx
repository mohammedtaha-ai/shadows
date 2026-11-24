// One job: the plan page's project recheck, a button, the line about the newest batch, and the
// "Rechecking…" state while Core still counts rows unfinished.

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { type LastRecheck, invalidateRuns, lastRecheckQuery, recheckProject } from '@/api/runs'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'

const clock = new Intl.DateTimeFormat(undefined, { hour: '2-digit', minute: '2-digit', hour12: false })

function timeOf(recheck: LastRecheck): string {
  const at = new Date(recheck.finished_at ?? recheck.created_at)
  return Number.isNaN(at.getTime()) ? '' : clock.format(at)
}

/** The line between the progress bar and the buttons; nothing when the project had no recheck. */
export function RecheckLine({ workflowId }: { workflowId: string }) {
  const { data } = useQuery(lastRecheckQuery(workflowId))
  if (data == null) return null
  const { passed, failed, not_run: notRun } = data.counts
  return <p className="text-xs tabular-nums text-muted-foreground">
    Last project recheck {timeOf(data)} · {passed} passed
    {failed > 0 && <> · <span className="text-destructive">{failed} failed</span></>}
    {' '}· {notRun} not run
  </p>
}

/** "Recheck project", or "Rechecking…" in its place while a batch has unfinished rows. */
export function RecheckControl({ workflowId }: { workflowId: string }) {
  const queryClient = useQueryClient()
  const { data } = useQuery(lastRecheckQuery(workflowId))
  const recheck = useMutation({
    mutationFn: (commandId: string) => recheckProject(workflowId, commandId),
    onSettled: () => { invalidateRuns(queryClient) },
  })
  if ((data?.counts.unfinished ?? 0) > 0) {
    return <span role="status" className="px-2 text-xs text-muted-foreground">Rechecking…</span>
  }
  return <>
    <Button variant="outline" size="sm" disabled={recheck.isPending}
      className="text-xs shadow-2xs transition-all hover:bg-accent/40"
      onClick={() => { recheck.reset(); recheck.mutate(crypto.randomUUID()) }}>
      Recheck project
    </Button>
    {recheck.error !== null && <ErrorLine error={recheck.error} />}
  </>
}
