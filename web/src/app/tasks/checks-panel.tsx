import { useQuery } from '@tanstack/react-query'
import { useId, useState } from 'react'
import { attemptChecksQuery, type CheckResult } from '@/api/runs'
import { SkeletonContent } from '@/components/ui/skeleton'
import { ErrorLine } from '../error-line'

function CheckRow({ check }: { check: CheckResult }) {
  const [open, setOpen] = useState(false)
  const outputId = useId()
  const outcome = (value: string | null | undefined) => value?.replaceAll('_', ' ') ?? 'pending'
  return <li className="space-y-2 rounded-lg border border-border p-3">
    <div className="flex flex-wrap items-center gap-2">
      <h4 className="font-semibold">T{check.for_task} · {check.id}</h4>
      {check.matches === false && <span className="rounded bg-warning/15 px-2 py-0.5 text-xs text-warning">Mismatch</span>}
    </div>
    <p className="break-words font-mono text-xs">{check.run.join(' ')}</p>
    <div className="flex flex-wrap gap-x-5 gap-y-1 text-xs">
      <span>Reported: {outcome(check.reported)}</span>
      <span className={check.outcome === 'passed' ? 'text-success' : check.outcome != null ? 'text-warning' : 'text-muted-foreground'}>
        Confirmed: {outcome(check.outcome)}
      </span>
      {check.exit_code != null && <span>Exit code: {check.exit_code}</span>}
    </div>
    {check.reported_note != null && <p dir="auto" className="whitespace-pre-wrap break-words text-xs text-muted-foreground">{check.reported_note}</p>}
    {check.folder != null && <p className="break-all text-xs text-muted-foreground">Folder: {check.folder}</p>}
    {check.head_before != null && <p className="break-all font-mono text-xs text-muted-foreground">HEAD before: {check.head_before}</p>}
    {check.head_after != null && <p className="break-all font-mono text-xs text-muted-foreground">HEAD after: {check.head_after}</p>}
    {check.output != null && <div>
      <button type="button" aria-expanded={open} aria-controls={outputId}
        aria-label={`${open ? 'Hide' : 'Show'} output for T${check.for_task} ${check.id}`} onClick={() => setOpen(!open)}
        className="cursor-pointer text-xs text-accent-line underline underline-offset-2">{open ? 'Hide output' : 'Show output'}</button>
      {open && <div id={outputId} className="mt-2">
        {check.truncated && <p className="text-xs text-warning">Output truncated; showing its end.</p>}
        <pre aria-label="Check output" dir="auto" className="max-h-80 overflow-auto whitespace-pre-wrap break-words rounded bg-muted/40 p-3 text-xs">{check.output || '(empty output)'}</pre>
      </div>}
    </div>}
  </li>
}

/** The executor's report and Shadows' confirmation stay separate, as Core records them. */
export function ChecksPanel({ attemptId, watch }: { attemptId: string; watch: boolean }) {
  const checks = useQuery(attemptChecksQuery(attemptId, watch))
  return <section aria-label="Checks" className="space-y-2 border-t border-border pt-3">
    <h3 className="text-sm font-semibold">Checks</h3>
    {checks.data !== undefined ? (checks.data.length === 0
      ? <p className="text-xs text-muted-foreground">No check results recorded.</p>
      : <ul className="space-y-2">{checks.data.map((check) => <CheckRow key={`${check.for_task}:${check.id}`} check={check} />)}</ul>)
      : checks.error === null && <SkeletonContent label="Reading the checks…" layout="detail" />}
    {checks.error !== null && <ErrorLine error={checks.error} />}
  </section>
}
