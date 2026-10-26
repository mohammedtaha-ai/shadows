// A part edit retains local input until explicitly replaced or saved.
import { useQuery } from '@tanstack/react-query'
import { Link } from '@tanstack/react-router'
import type { Part, PartContent, PartView } from '@/api/design'
import { partQuery, partsQuery, plansQuery, visionQuery } from '@/api/queries'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'
import { usePartPages } from '@/api/design'
import { type DesignEditorConfig, useDesignEditor } from './use-design-editor'
import { MapPartDetails } from './map-part-details'

const EDITOR: DesignEditorConfig<PartView, PartContent> = {
  entityKind: 'Part',
  empty: { title: '', responsibility: '', design: '', kind: null, serves:[],folders:[],files:[],contract:{header:'',rules:[]} },
  idOf: view => view.part.id,
  contentOf: view => view.part.content,
  detailQuery: partQuery,
  listQuery: partsQuery,
  contentOp: (id, content, position) => {
    const cleaned = {...content,folders:content.folders?.filter(Boolean),contract:content.contract
      ? {...content.contract,rules:content.contract.rules.filter(Boolean)} : undefined}
    return position ? {kind:'PartCreate',id,content:cleaned,...position} : {kind:'PartPut',id,content:cleaned}
  },
}

export function PartEditor({ projectId, saved, destination, onCreated }: {
  projectId: string
  saved?: PartView
  destination: Part | null
  onCreated?: () => void
}) {
  const plans = useQuery(plansQuery(projectId, true))
  const vision = useQuery(visionQuery(projectId))
  const items = Object.values(vision.data?.content ?? {}).flat()
  const {
    base, draft, setDraft, linked, setLinked,
    moving, setMoving, before, setBefore, id, dirty, newer,
    createRevision, reloading, reloadError, save, submit, reload,
  } = useDesignEditor(EDITOR, { projectId, saved, destination: destination?.id, onCreated })
  return <div className="space-y-3 rounded border p-4">
    <h3>{base ? 'Part design' : 'New part'}</h3>
    {saved && <nav aria-label="Part breadcrumb" className="flex flex-wrap gap-2 text-sm">
      <Link to="/projects/$projectId/workspace" params={{ projectId }} search={{ view: 'map' }}>Project</Link>
      {saved.ancestors.map(p => <Link key={p.id} to="/projects/$projectId/workspace" params={{ projectId }} search={{ view: 'map', part: p.id }} dir="auto">/ {p.content.title}</Link>)}
      <span dir="auto">/ {saved.part.content.title}</span>
    </nav>}
    {newer && <div role="status">Newer part data exists. Your unsaved text is kept.
      <Button size="sm" variant="secondary" disabled={save.isPending || reloading} onClick={() => void reload()}>Reload</Button></div>}
    <fieldset disabled={save.isPending || reloading} className="space-y-3">
      <label className="block text-sm">Title<input aria-label="Part title" dir="auto" value={draft.title} onChange={e => setDraft({ ...draft, title: e.target.value })} className="block w-full rounded border bg-transparent p-2" /></label>
      {(['responsibility', 'design', 'kind'] as const).map(field => <label key={field} className="block text-sm">{field}
        <textarea aria-label={`Part ${field}`} dir="auto" rows={field === 'design' ? 5 : 2} value={draft[field] ?? ''} onChange={e => setDraft({ ...draft, [field]: field === 'kind' ? e.target.value || null : e.target.value })} className="block w-full rounded border bg-transparent p-2" /></label>)}
      <div><h4>Served vision items</h4>{[...new Set([...items.map(i => i.id),...(draft.serves ?? [])])].map(id =>
        <label key={id} className="block text-sm"><input type="checkbox" aria-label={`Serves ${id}`}
          checked={(draft.serves ?? []).includes(id)} onChange={e => setDraft({...draft,serves:e.target.checked
            ? [...(draft.serves ?? []),id] : (draft.serves ?? []).filter(i => i !== id)})} />
          {id} · {items.find(i => i.id === id)?.text ?? 'Removed item'}</label>)}</div>
      <label className="block text-sm">Folders<textarea aria-label="Part folders" rows={3}
        value={(draft.folders ?? []).join('\n')} onChange={e => setDraft({...draft,folders:e.target.value.split('\n')})}
        className="block w-full rounded border bg-transparent p-2" /></label>
      <label className="block text-sm">Contract header<textarea aria-label="Part contract header" dir="auto"
        value={draft.contract?.header ?? ''} onChange={e => setDraft({...draft,
          contract:{header:e.target.value,rules:draft.contract?.rules ?? []}})}
        className="block w-full rounded border bg-transparent p-2" /></label>
      <label className="block text-sm">Contract rule ids<textarea aria-label="Part contract rules"
        value={(draft.contract?.rules ?? []).join('\n')} onChange={e => setDraft({...draft,
          contract:{header:draft.contract?.header ?? '',rules:e.target.value.split('\n')}})}
        className="block w-full rounded border bg-transparent p-2" /></label>
      <MapPartDetails content={draft} projectId={projectId} />
      {base && <label className="block text-sm"><input type="checkbox" checked={moving} onChange={e => { setMoving(e.target.checked); setBefore('') }} /> Move part to {destination?.content.title ?? 'Project root'}</label>}
      {(moving || !base) && <PartOrder key={destination?.id ?? 'root'} projectId={projectId} parent={destination?.id} id={id} value={before} change={setBefore} />}
      <div className="space-y-1"><h4 className="text-sm">Linked plans (including archived)</h4>
        {plans.data?.map(p => <label key={p.plan_id} className="flex gap-2 text-sm"><input type="checkbox" checked={linked.includes(p.plan_id)} onChange={e => setLinked(e.target.checked ? [...linked, p.plan_id].sort() : linked.filter(id => id !== p.plan_id))} />
          <Link to="/projects/$projectId/workflows/$workflowId" params={{ projectId, workflowId: p.id }}>{p.title}</Link>{p.plan_state === 'Archived' && <span>Archived</span>}</label>)}
      </div>
    </fieldset>
    {(saved?.binding_plans?.length ?? 0) > 0 && <section className="space-y-2">
      <h4>Plans adopting this part's contracts</h4>
      {saved?.binding_plans?.map(p => <div key={`${p.workflow_id}:${p.binding.agreement_id}:${p.binding.task}:${p.binding.role}`}
        className="text-sm"><Link to="/projects/$projectId/workflows/$workflowId"
          params={{ projectId, workflowId: p.workflow_id }}>{p.title} · T{p.binding.task}</Link>
        <p>{p.current ? 'Current' : 'Historical'} · {p.plan_state} · {p.binding.role} · contract v{p.binding.version}</p>
      </div>)}
    </section>}
    <Button size="sm" onClick={submit} disabled={!draft.title.trim() || !dirty || save.isPending || reloading || (!base && createRevision === undefined)}>{base ? 'Save part' : 'Create'}</Button>
    {save.error && <ErrorLine error={save.error} />}{reloadError && <ErrorLine error={reloadError} />}
    {plans.error && <ErrorLine error={plans.error} />}
  </div>
}

function PartOrder({ projectId, parent, id, value, change }: { projectId: string; parent?: string; id: string; value: string; change: (value: string) => void }) {
  const branch = usePartPages(projectId, parent)
  return <div><label className="block text-sm">Place before<select aria-label="Place before" className="ml-2 rounded border bg-background p-2" value={value} onChange={e => change(e.target.value)}>
    <option value="">End of branch</option>{branch.items.filter(p => p.id !== id).map(p => <option value={p.id} key={p.id}>{p.content.title}</option>)}
  </select></label>
    {branch.next && <Button size="sm" variant="secondary" disabled={branch.loading} onClick={() => void branch.more()}>More destinations</Button>}
    {branch.error && <ErrorLine error={branch.error} />}
  </div>
}
