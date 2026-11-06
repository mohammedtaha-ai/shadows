// A part edit retains local input until explicitly replaced or saved.
import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Link } from '@tanstack/react-router'
import type { Part, PartContent, PartView } from '@/api/design'
import { partQuery, partsQuery, visionQuery, standardsQuery } from '@/api/design'
import { plansQuery } from '@/api/workflows'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'
import { usePartPages } from '@/api/design'
import { type DesignEditorConfig, useDesignEditor } from './use-design-editor'
import { MapPartDetails } from './map-part-details'

const EDITOR: DesignEditorConfig<PartView, PartContent> = {
  entityKind: 'Part',
  empty: { title: '', responsibility: '', design: '', kind: null, serves:[],folder:'',layers:[],files:[],contract:{header:'',rules:[]} },
  idOf: view => view.part.id,
  contentOf: view => view.part.content,
  detailQuery: partQuery,
  listQuery: partsQuery,
  contentOp: (id, content, position) => {
    const cleaned = {...content,folder:content.folder?.trim() ?? '',contract:content.contract
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
  const standards = useQuery(standardsQuery(projectId))
  const children = usePartPages(projectId, saved?.part.id)
  const items = Object.values(vision.data?.content ?? {}).flat()
  const {
    base, draft, setDraft, linked, setLinked,
    moving, setMoving, before, setBefore, id, dirty, newer,
    createRevision, reloading, reloadError, save, submit, reload,
  } = useDesignEditor(EDITOR, { projectId, saved, destination: destination?.id, onCreated })
  const [adding, setAdding] = useState('')
  const kind = draft.kind
  const catalog = standards.data?.base.parts.find(part => part.name === kind?.trim())?.layers ?? []
  // Core sends no grouping fact: a typed part with a child of its own kind groups it, as the map rules read it.
  const groups = saved && kind != null ? children.items.filter(part => part.content.kind?.trim() === kind.trim()) : []
  const grouping = saved !== undefined && kind != null
  const childScopeKnown = !grouping || (!children.pending && !children.error && !children.next)
  const offCatalog = standards.data ? (draft.layers ?? []).filter(layer => !catalog.some(spec => spec.id === layer.layer)) : []
  const served = draft.serves ?? []
  const unserved = items.filter(item => !served.includes(item.id))
  const changeLayer = (layer: NonNullable<PartContent['layers']>[number]) => setDraft({
    ...draft, layers: [...(draft.layers ?? []).filter(item => item.layer !== layer.layer), layer],
  })
  const removeLayer = (id: string) => setDraft({...draft, layers: (draft.layers ?? []).filter(layer => layer.layer !== id)})
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
      <section className="space-y-2" aria-label="Served vision items"><h4>Served vision items</h4>
        {served.length === 0 && <p className="text-sm text-muted-foreground">No linked items</p>}
        {served.map(id => <div key={id} className="flex items-center gap-2 text-sm">
          <span className="flex-1" dir="auto">{id} · {items.find(i => i.id === id)?.text ?? 'Removed item'}</span>
          <Button size="sm" variant="ghost" aria-label={`Stop serving ${id}`}
            onClick={() => setDraft({...draft,serves:served.filter(i => i !== id)})}>Remove</Button>
        </div>)}
        {unserved.length > 0 && <div className="flex flex-wrap items-center gap-2 text-sm">
          <select aria-label="Vision item to serve" value={adding} onChange={e => setAdding(e.target.value)}
            className="min-w-0 flex-1 rounded border bg-background p-2">
            <option value="">Choose a vision item</option>
            {unserved.map(item => <option key={item.id} value={item.id}>{item.id} · {item.text}</option>)}
          </select>
          <Button size="sm" variant="secondary" disabled={!adding}
            onClick={() => { setDraft({...draft,serves:[...served,adding]}); setAdding('') }}>Add</Button>
        </div>}
        {vision.error && <ErrorLine error={vision.error} />}
      </section>
      <label className="block text-sm">Folder<input aria-label="Part folder"
        value={draft.folder ?? ''} onChange={e => setDraft({...draft,folder:e.target.value})}
        className="block w-full rounded border bg-transparent p-2" /></label>
      {grouping && children.pending && <p>Loading child parts…</p>}
      {grouping && children.error && <ErrorLine error={children.error} />}
      {grouping && children.next && <div className="space-y-2 text-sm">
        <p>Child list is incomplete. Load the remaining parts to inspect grouping.</p>
        <Button size="sm" variant="secondary" disabled={children.loading}
          onClick={() => void children.more()}>Load more child parts</Button>
      </div>}
      {groups.length > 0 || !childScopeKnown ? null : <section className="space-y-2" aria-label="Part layers">
        <h4 className="font-medium">Layers</h4>
        {kind == null && <p className="text-sm text-muted-foreground">Choose a kind to see its layers.</p>}
        {catalog.map(spec => {
          const layer = draft.layers?.find(layer => layer.layer === spec.id)
          if (!spec.required && !layer) return <Button key={spec.id} size="sm" variant="secondary"
            onClick={() => changeLayer({layer:spec.id,place:'',not_used:null})}>Add {spec.id} layer</Button>
          const value = layer ?? {layer:spec.id,place:'',not_used:null}
          const unused = value.not_used != null
          return <div key={spec.id} className="space-y-2 rounded-lg border border-border bg-card p-3 text-sm">
            <p className="font-medium">{spec.id} · {spec.required ? 'required' : 'optional'}</p>
            <p className="text-muted-foreground">{spec.role}</p>
            {spec.required ? <label>Usage<select aria-label={`${spec.id} usage`} value={unused ? 'not-used' : 'used'}
              onChange={e => changeLayer({...value,place:'',not_used:e.target.value === 'not-used' ? '' : null})}
              className="ml-2 rounded border bg-input-background p-2">
              <option value="used">Used</option><option value="not-used">Not used</option>
            </select></label> : <Button size="sm" variant="ghost" onClick={() => removeLayer(spec.id)}>Remove {spec.id} layer</Button>}
            {unused ? <label className="block">Reason<input aria-label={`${spec.id} reason`} dir="auto"
              value={value.not_used ?? ''} onChange={e => changeLayer({...value,not_used:e.target.value})}
              className="block w-full rounded border bg-input-background p-2" /></label>
              : <label className="block">Place<input aria-label={`${spec.id} place`} value={value.place}
                onChange={e => changeLayer({...value,place:e.target.value})}
                className="block w-full rounded border bg-input-background p-2" /></label>}
          </div>
        })}
        {offCatalog.map(layer => <div key={layer.layer} className="space-y-2 rounded-lg border border-border bg-card p-3 text-sm">
          <p className="font-medium">{layer.layer} · not in the {kind?.trim() || 'untyped'} catalog</p>
          <p className="break-words text-muted-foreground">{layer.not_used != null ? `Not used: ${layer.not_used}` : layer.place || 'Place not recorded'}</p>
          <Button size="sm" variant="ghost" onClick={() => removeLayer(layer.layer)}>Remove {layer.layer} layer</Button>
        </div>)}
        {standards.isPending && <p>Loading layer catalog…</p>}
        {standards.error && <ErrorLine error={standards.error} />}
      </section>}
      <label className="block text-sm">Contract header<textarea aria-label="Part contract header" dir="auto"
        value={draft.contract?.header ?? ''} onChange={e => setDraft({...draft,
          contract:{header:e.target.value,rules:draft.contract?.rules ?? []}})}
        className="block w-full rounded border bg-transparent p-2" /></label>
      <label className="block text-sm">Contract rule ids<textarea aria-label="Part contract rules"
        value={(draft.contract?.rules ?? []).join('\n')} onChange={e => setDraft({...draft,
          contract:{header:draft.contract?.header ?? '',rules:e.target.value.split('\n')}})}
        className="block w-full rounded border bg-transparent p-2" /></label>
      <MapPartDetails content={draft} projectId={projectId}
        groups={groups.map(child => ({title:child.content.title,part:child.id}))} />
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
