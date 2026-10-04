// The project plan list includes unassigned plans, retaining its archive filter.
import { useState } from 'react'
import { Link } from '@tanstack/react-router'
import { useQuery } from '@tanstack/react-query'
import { plansQuery } from '@/api/queries'
import { ErrorLine } from '../error-line'
export function WorkspacePlans({ projectId }: { projectId: string }) {
  const [archived, setArchived] = useState(false)
  const plans = useQuery(plansQuery(projectId, archived))
  return <section className="space-y-3"><h2>Project plans</h2>
    <p className="text-sm text-muted-foreground">All plans stay accessible here, including those without workspace associations.</p>
    <label className="text-sm"><input type="checkbox" checked={archived} onChange={e => setArchived(e.target.checked)} /> Include archived</label>
    {plans.isPending && <p>Loading plans…</p>}{plans.error && <ErrorLine error={plans.error} />}
    <ul className="space-y-2">{plans.data?.map(p => <li key={p.plan_id} className="rounded border p-3 text-sm">
      <Link to="/projects/$projectId/workflows/$workflowId" params={{ projectId, workflowId: p.id }} dir="auto">{p.title}</Link>
      <span className="ml-3 text-muted-foreground">v{p.version} · {p.state}{p.plan_state === 'Archived' ? ' · Archived' : ''}</span>
    </li>)}</ul>
  </section>
}
