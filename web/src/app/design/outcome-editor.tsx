// An outcome edit retains local input until explicitly replaced or saved.
import { useQuery } from '@tanstack/react-query'
import { Link } from '@tanstack/react-router'
import type { Outcome, OutcomeContent, OutcomeView } from '@/api/design'
import { outcomeQuery, outcomesQuery, plansQuery } from '@/api/queries'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'
import { useOutcomePages } from './use-outcome-pages'
import { type DesignEditorConfig, useDesignEditor } from './use-design-editor'
import { PartReferences } from './part-references'

const EDITOR: DesignEditorConfig<OutcomeView, OutcomeContent> = {
  entityKind: 'Outcome',
  empty: { title: '', intended_result: '', acceptance: [] },
  idOf: view => view.outcome.id,
  contentOf: view => view.outcome.content,
  detailQuery: outcomeQuery,
  listQuery: outcomesQuery,
  contentOp: (id, content, position) => position
    ? { kind: 'OutcomeCreate', id, content, ...position }
    : { kind: 'OutcomePut', id, content },
  relations: {
    read: view => view.parts,
    ops: (id, selected, previous) => [
      ...selected.filter(p => !previous.includes(p)).map(part => (
        { kind: 'OutcomePartPut' as const, outcome: id, part }
      )),
      ...previous.filter(p => !selected.includes(p)).map(part => (
        { kind: 'OutcomePartRemove' as const, outcome: id, part }
      )),
    ],
  },
}

export function OutcomeEditor({ projectId, saved, destination, onCreated }: {
  projectId: string
  saved?: OutcomeView
  destination: Outcome | null
  onCreated?: () => void
}) {
  const plans = useQuery(plansQuery(projectId, true))
  const {
    base, draft, setDraft, linked, setLinked, related: parts, setRelated: setParts,
    moving, setMoving, before, setBefore, id, dirty, newer,
    createRevision, reloading, reloadError, save, submit, reload,
  } = useDesignEditor(EDITOR, { projectId, saved, destination: destination?.id, onCreated })
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
