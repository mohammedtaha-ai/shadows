// A part edit retains local input until explicitly replaced or saved.
import { useQuery } from '@tanstack/react-query'
import { Link } from '@tanstack/react-router'
import type { Part, PartContent, PartView } from '@/api/design'
import { partQuery, partsQuery, plansQuery } from '@/api/queries'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'
import { usePartPages } from './use-part-pages'
import { type DesignEditorConfig, useDesignEditor } from './use-design-editor'

const EDITOR: DesignEditorConfig<PartView, PartContent> = {
  entityKind: 'Part',
  empty: { title: '', responsibility: '', design: '', kind: null },
  idOf: view => view.part.id,
  contentOf: view => view.part.content,
  detailQuery: partQuery,
  listQuery: partsQuery,
  contentOp: (id, content, position) => position
    ? { kind: 'PartCreate', id, content, ...position }
    : { kind: 'PartPut', id, content },
}

export function PartEditor({ projectId, saved, destination, onCreated }: {
  projectId: string
  saved?: PartView
  destination: Part | null
  onCreated?: () => void
}) {
  const plans = useQuery(plansQuery(projectId, true))
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
      {base && <label className="block text-sm"><input type="checkbox" checked={moving} onChange={e => { setMoving(e.target.checked); setBefore('') }} /> Move part to {destination?.content.title ?? 'Project root'}</label>}
      {(moving || !base) && <PartOrder key={destination?.id ?? 'root'} projectId={projectId} parent={destination?.id} id={id} value={before} change={setBefore} />}
      <div className="space-y-1"><h4 className="text-sm">Linked plans (including archived)</h4>
        {plans.data?.map(p => <label key={p.plan_id} className="flex gap-2 text-sm"><input type="checkbox" checked={linked.includes(p.plan_id)} onChange={e => setLinked(e.target.checked ? [...linked, p.plan_id].sort() : linked.filter(id => id !== p.plan_id))} />
          <Link to="/projects/$projectId/workflows/$workflowId" params={{ projectId, workflowId: p.id }}>{p.title}</Link>{p.plan_state === 'Archived' && <span>Archived</span>}</label>)}
      </div>
    </fieldset>
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
