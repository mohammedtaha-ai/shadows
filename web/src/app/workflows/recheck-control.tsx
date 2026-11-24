// One job: the plan page's project recheck, a button, the line about the newest batch, and the
// "Rechecking…" state while Core still counts rows unfinished.

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { type LastRecheck, invalidateRuns, lastRecheckQuery, recheckProject } from '@/api/runs'
import { Terminal } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'

const clock = new Intl.DateTimeFormat(undefined, { hour: '2-digit', minute: '2-digit', hour12: false })

function timeOf(recheck: LastRecheck): string {
  const at = new Date(recheck.finished_at ?? recheck.created_at)
  return Number.isNaN(at.getTime()) ? '' : clock.format(at)
}

/** The line between the progress bar and the buttons; nothing when the project had no recheck. */
export function RecheckLine({ workflowId, open = false, onToggle }: {
  workflowId: string; open?: boolean; onToggle?: () => void
}) {
  const { data } = useQuery(lastRecheckQuery(workflowId))
  if (data == null) return null
  const { passed, failed, not_run: notRun } = data.counts
  return <p className="flex items-center gap-1.5 text-xs tabular-nums text-muted-foreground">
    <span>
      Last project recheck {timeOf(data)} · {passed} passed
      {failed > 0 && <> · <span className="text-destructive">{failed} failed</span></>}
      {' '}· {notRun} not run
    </span>
    {onToggle !== undefined && <Button variant="ghost" size="icon-xs" aria-pressed={open}
      aria-label={open ? 'Close the recheck output' : 'Open the recheck output'} onClick={onToggle}>
      <Terminal aria-hidden className="size-3.5" />
    </Button>}
  </p>
}

/** "Recheck project", or "Rechecking…" in its place while a batch has unfinished rows. */
export function RecheckControl({ workflowId, onStarted }: { workflowId: string; onStarted?: () => void }) {
  const queryClient = useQueryClient()
  const { data } = useQuery(lastRecheckQuery(workflowId))
  const recheck = useMutation({
    mutationFn: (commandId: string) => recheckProject(workflowId, commandId),
    onSuccess: () => { onStarted?.() },
    onSettled: () => { invalidateRuns(queryClient) },
  })
  if (data != null && data.counts.unfinished > 0) {
    const { passed, failed, timed_out, inconclusive, not_run, unfinished } = data.counts
    const total = passed + failed + timed_out + inconclusive + not_run + unfinished
    return <span role="status" className="px-2 text-xs tabular-nums text-muted-foreground">
      Rechecking… {total - unfinished} of {total}
    </span>
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
