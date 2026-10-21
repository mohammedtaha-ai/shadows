import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { usageQuery } from '@/api/usage'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../../error-line'
import { usageRange, type UsagePeriod } from '../usage-period'
import { UsageTables } from '../usage-tables'

export function UsageSection() {
  const [selection, setSelection] = useState(() => ({ period: '7d' as UsagePeriod, range: usageRange('7d', new Date(Date.now())) }))
  const usage = useQuery(usageQuery(selection.range.from, selection.range.to))
  return <div className="space-y-5">
    <label className="grid gap-2 text-sm">Period
      <select aria-label="Period" value={selection.period} className="rounded border border-input bg-background p-2" onChange={event => {
        const period = event.target.value as UsagePeriod
        setSelection({ period, range: usageRange(period, new Date(Date.now())) })
      }}>
        <option value="7d">Last 7 days</option><option value="30d">Last 30 days</option><option value="month">This month</option>
      </select>
    </label>
    {usage.isPending ? <p role="status">Loading usage…</p> : usage.error ? <div className="space-y-3"><ErrorLine error={usage.error} /><Button type="button" variant="outline" disabled={usage.isFetching} onClick={() => { void usage.refetch() }}>Retry</Button></div> :
      usage.data.by_provider.length === 0 && usage.data.by_model.length === 0 ? <p>No usage in this period.</p> : <UsageTables report={usage.data} />}
  </div>
}
