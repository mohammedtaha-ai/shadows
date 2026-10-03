// An outcome edit retains local input until explicitly replaced or saved.
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link } from '@tanstack/react-router'
import { useRef, useState } from 'react'
import { type Attempt, attemptFor } from '@/api/command-id'
import { editDesign, type DesignEdit, type Outcome, type OutcomeContent, type OutcomeView } from '@/api/design'
import { ApiError } from '@/api/error'
import { outcomeQuery, outcomesQuery, plansQuery } from '@/api/queries'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'
import { useOutcomePages } from './use-outcome-pages'
import { PartReferences } from './part-references'

const EMPTY: OutcomeContent = { title: '', intended_result: '', acceptance: [] }
export function OutcomeEditor({ projectId, saved, destination, onCreated }: { projectId: string; saved?: OutcomeView; destination: Outcome | null; onCreated?: () => void }) {
  const client = useQueryClient()
  const root = useQuery({ ...outcomesQuery(projectId), enabled: saved === undefined })
  const plans = useQuery(plansQuery(projectId, true))
  const [base, setBase] = useState(saved)
  const [draft, setDraft] = useState(saved?.outcome.content ?? EMPTY)
  const [linked, setLinked] = useState(saved?.plans ?? [])
  const [parts, setParts] = useState(saved?.parts ?? [])
  const [moving, setMoving] = useState(false)
  const [order, setOrder] = useState({ destination: destination?.id, before: '' })
  const before = order.destination === destination?.id ? order.before : ''
  const setBefore = (value: string) => setOrder({ destination: destination?.id, before: value })
  const [conflict, setConflict] = useState(false)
  const [reloading, setReloading] = useState(false)
  const [reloadError, setReloadError] = useState<Error | null>(null)
  const [createRevision, setCreateRevision] = useState<number | undefined>(root.data?.revision)
  if (createRevision === undefined && root.data) setCreateRevision(root.data.revision)
  const [id] = useState(() => saved?.outcome.id ?? crypto.randomUUID())
  const attempt = useRef<Attempt | null>(null)
  const dirty = JSON.stringify(draft) !== JSON.stringify(base?.outcome.content ?? EMPTY) || JSON.stringify(linked) !== JSON.stringify(base?.plans ?? []) || JSON.stringify(parts) !== JSON.stringify(base?.parts ?? []) || moving
  if (saved && base && saved.revision > base.revision && !dirty) {
    setBase(saved); setDraft(saved.outcome.content); setLinked(saved.plans); setParts(saved.parts)
  }
  const newer = conflict || (saved !== undefined && base !== undefined && saved.revision > base.revision) ||
    (base === undefined && createRevision !== undefined && root.data !== undefined && root.data.revision > createRevision)
  const save = useMutation({
    mutationFn: (body: DesignEdit) => editDesign(projectId, body),
    onSuccess: async () => {
      attempt.current = null; setConflict(false)
      await client.invalidateQueries({ queryKey: ['projects', projectId, 'design'] })
      if (!base) { onCreated?.(); return }
      const view = await client.fetchQuery({ ...outcomeQuery(projectId, id), staleTime: 0 })
      setBase(view); setDraft(view.outcome.content); setLinked(view.plans); setParts(view.parts); setMoving(false); setBefore('')
    },
    onError: e => {
      if (e instanceof ApiError && e.problem.kind === 'daemon' && e.problem.code === 'REVISION_CONFLICT') {
        setConflict(true); void client.invalidateQueries({ queryKey: ['projects', projectId, 'design'] })
      }
    },
  })
  const submit = () => {
    const ops: DesignEdit['ops'] = base
      ? [{ kind: 'OutcomePut', id, content: draft }]
      : [{ kind: 'OutcomeCreate', id, parent: destination?.id ?? null, before: before || null, content: draft }]
    if (base && moving) ops.push({ kind: 'OutcomeMove', id, parent: destination?.id ?? null, before: before || null })
    for (const plan of linked.filter(p => !base?.plans.includes(p))) ops.push({ kind: 'PlanLinkPut', anchor: { kind: 'Outcome', id }, plan })
    for (const plan of (base?.plans ?? []).filter(p => !linked.includes(p))) ops.push({ kind: 'PlanLinkRemove', anchor: { kind: 'Outcome', id }, plan })
    for (const part of parts.filter(p => !base?.parts.includes(p))) ops.push({ kind: 'OutcomePartPut', outcome: id, part })
    for (const part of (base?.parts ?? []).filter(p => !parts.includes(p))) ops.push({ kind: 'OutcomePartRemove', outcome: id, part })
    const expected_revision = base?.revision ?? createRevision
    if (expected_revision === undefined) return
    const request = { expected_revision, ops }
    attempt.current = attemptFor(attempt.current, request)
    save.mutate({ ...request, command_id: attempt.current.commandId })
  }
  const reload = async () => {
    setReloading(true); setReloadError(null)
    try {
      if (!base) {
        const page = await client.fetchQuery({ ...outcomesQuery(projectId), staleTime: 0 })
        setCreateRevision(page.revision); setDraft(EMPTY); setLinked([]); setParts([]); setBefore(''); setConflict(false)
        attempt.current = null; save.reset()
        return
      }
      const view = await client.fetchQuery({ ...outcomeQuery(projectId, id), staleTime: 0 })
      setBase(view); setDraft(view.outcome.content); setLinked(view.plans); setParts(view.parts); setMoving(false); setBefore(''); setConflict(false)
      attempt.current = null; save.reset()
    } catch (e) { setReloadError(e instanceof Error ? e : new Error(String(e))) }
    finally { setReloading(false) }
  }
  return <div className="space-y-3 rounded border p-4">
    <h3>{base ? 'Outcome design' : 'New outcome'}</h3>
    {saved && <nav aria-label="Outcome breadcrumb" className="flex flex-wrap gap-2 text-sm">
      <Link to="/projects/$projectId/workspace" params={{ projectId }} search={{ view: 'roadmap' }}>Project</Link>
      {saved.ancestors.map(p => <Link key={p.id} to="/projects/$projectId/workspace" params={{ projectId }} search={{ view: 'roadmap', outcome: p.id }} dir="auto">/ {p.content.title}</Link>)}
      <span dir="auto">/ {saved.outcome.content.title}</span>
    </nav>}
    {newer && <div role="status">Newer outcome data exists. Your unsaved text is kept.
      <Button size="sm" variant="secondary" disabled={save.isPending || reloading} onClick={() => void reload()}>Reload</Button></div>}
    <fieldset disabled={save.isPending || reloading} className="space-y-3">
      <label className="block text-sm">Title<input aria-label="Outcome title" dir="auto" value={draft.title} onChange={e => setDraft({ ...draft, title: e.target.value })} className="block w-full rounded border bg-transparent p-2" /></label>
      <label className="block text-sm">Intended result<textarea aria-label="Outcome result" dir="auto" rows={4} value={draft.intended_result} onChange={e => setDraft({ ...draft, intended_result: e.target.value })} className="block w-full rounded border bg-transparent p-2" /></label>
      <label className="block text-sm">Acceptance items (one per line)<textarea aria-label="Outcome acceptance" dir="auto" rows={4} value={draft.acceptance.join('\n')} onChange={e => setDraft({ ...draft, acceptance: e.target.value === '' ? [] : e.target.value.split('\n') })} className="block w-full rounded border bg-transparent p-2" /></label>
      <PartReferences projectId={projectId} selected={parts} change={setParts} />
      {base && <label className="block text-sm"><input type="checkbox" checked={moving} onChange={e => { setMoving(e.target.checked); setBefore('') }} /> Move outcome to {destination?.content.title ?? 'Roadmap root'}</label>}
      {(moving || !base) && <OutcomeOrder key={destination?.id ?? 'root'} projectId={projectId} parent={destination?.id} id={id} value={before} change={setBefore} />}
      <div className="space-y-1"><h4 className="text-sm">Linked plans (including archived)</h4>
        {plans.data?.map(p => <label key={p.plan_id} className="flex gap-2 text-sm"><input type="checkbox" checked={linked.includes(p.plan_id)} onChange={e => setLinked(e.target.checked ? [...linked, p.plan_id].sort() : linked.filter(id => id !== p.plan_id))} />
          <Link to="/projects/$projectId/workflows/$workflowId" params={{ projectId, workflowId: p.id }}>{p.title}</Link>{p.plan_state === 'Archived' && <span>Archived</span>}</label>)}
      </div>
    </fieldset>
    <Button size="sm" onClick={submit} disabled={!draft.title.trim() || !dirty || save.isPending || reloading || (!base && createRevision === undefined)}>{base ? 'Save outcome' : 'Create'}</Button>
    {save.error && <ErrorLine error={save.error} />}{reloadError && <ErrorLine error={reloadError} />}
    {plans.error && <ErrorLine error={plans.error} />}
  </div>
}

function OutcomeOrder({ projectId, parent, id, value, change }: { projectId: string; parent?: string; id: string; value: string; change: (value: string) => void }) {
  const branch = useOutcomePages(projectId, parent)
  return <div><label className="block text-sm">Place before<select aria-label="Place before" className="ml-2 rounded border bg-background p-2" value={value} onChange={e => change(e.target.value)}>
    <option value="">End of branch</option>{branch.items.filter(p => p.id !== id).map(p => <option value={p.id} key={p.id}>{p.content.title}</option>)}
  </select></label>
    {branch.next && <Button size="sm" variant="secondary" disabled={branch.loading} onClick={() => void branch.more()}>More destinations</Button>}
    {branch.error && <ErrorLine error={branch.error} />}
  </div>
}
