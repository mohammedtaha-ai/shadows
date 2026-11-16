import { useQuery } from '@tanstack/react-query'
import { attemptChecksQuery } from '@/api/runs'
import { SkeletonContent } from '@/components/ui/skeleton'
import { CheckRow } from '../check-row'
import { ErrorLine } from '../error-line'

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
