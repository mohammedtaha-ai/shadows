// A task adopts an exact Agreed contract; retry retains its command until the read succeeds.
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link } from '@tanstack/react-router'
import { useRef, useState } from 'react'
import { type Plan, type PlanOp, listAgreements, getAgreement, editBindings, getPlan } from '@/api/client'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'
import { operations as documentOperations } from './agreement-document'
import { partQuery } from '@/api/queries'
export function BindingEditor({ plan, task }: { plan: Plan; task: number }) {
  const client = useQueryClient()
  const [id, setId] = useState('')
  const [version, setVersion] = useState(1)
  const [party, setParty] = useState('')
  const [operations, setOperations] = useState<string[]>([])
  const pending = useRef<{ id: string; revision: number; ops: PlanOp[] } | null>(null)
  const list = useQuery({ queryKey: ['projects', plan.project_id, 'agreements'],
    queryFn: () => listAgreements(plan.project_id) })
  const selected = useQuery({ queryKey: ['projects', plan.project_id, 'agreement', id, version],
    queryFn: () => getAgreement(plan.project_id, id, version), enabled: !!id })
  const save = useMutation({ mutationFn: async (ops: PlanOp[]) => {
    pending.current ??= { id: crypto.randomUUID(), revision: plan.revision, ops }
    const request = pending.current
    await editBindings(plan.id, request.id, request.revision, request.ops)
    return getPlan(plan.id)
  }, onSuccess: p => {
    pending.current = null
    client.setQueryData(['workflows', 'plan', plan.id], p)
    void client.invalidateQueries({ queryKey: ['workflows'] })
    void client.invalidateQueries({ queryKey: ['projects', plan.project_id, 'agreement'] })
    void client.invalidateQueries({ queryKey: ['projects', plan.project_id, 'design'] })
  } })
  const readonly = plan.state !== 'Draft' || plan.plan_state === 'Archived'
  const locked = readonly || save.isPending || save.isError
  const declared = selected.data?.content.parties.find(p => `${p.part_id}:${p.role}` === party)
  const choices = documentOperations(selected.data?.content.openapi)
  return <section className="mt-4 space-y-2 border-t border-border pt-3">
    <h3>Shared contracts</h3>
    {list.isError && <ErrorLine error={list.error} />}
    {(plan.bindings ?? []).filter(b => b.task === task).map(b => <div key={`${b.agreement_id}:${b.role}`}>
      <Link to="/projects/$projectId/agreements" params={{ projectId: plan.project_id }}
        search={{ agreement: b.agreement_id, version: b.version }}>{list.data?.find(v =>
          v.agreement_id === b.agreement_id)?.content.capability ?? 'Contract'} · {b.role} · v{b.version}</Link>
      <PinnedOperations projectId={plan.project_id} binding={b} />
      {(list.data?.find(v => v.agreement_id === b.agreement_id)?.version ?? b.version) > b.version &&
        <p className="text-xs">A newer {list.data?.find(v => v.agreement_id === b.agreement_id)?.state === 'Draft'
          ? 'proposal' : 'Agreed version'} is available. This task still uses v{b.version}.</p>}
      {!readonly && <Button size="sm" variant="outline" disabled={locked} onClick={() => save.mutate([
        { op: 'binding_remove', task, agreement_id: b.agreement_id, role: b.role },
      ])}>Remove binding</Button>}
    </div>)}
    {readonly ? <p className="text-xs">{plan.plan_state === 'Archived' ? 'Unarchive before adoption.' :
      'Continue this plan to a Draft before adoption.'}</p> : <fieldset disabled={locked} className="space-y-2">
      <select className="w-full" aria-label="Agreement" value={id} onChange={e => { setId(e.target.value); setParty(''); setOperations([]) }}>
        <option value="">Choose contract</option>{list.data?.map(v => <option key={v.agreement_id}
          value={v.agreement_id}>{v.content.capability}</option>)}</select>
      <label>Exact version <input className="w-16" aria-label="Agreement version" type="number" min={1}
        value={version} onChange={e => { setVersion(Number(e.target.value)); setParty(''); setOperations([]) }} /></label>
      {selected.isError && <ErrorLine error={selected.error} />}
      {selected.data && <>
        <p>{selected.data.state} v{selected.data.version}</p>
        <select className="w-full" aria-label="Participant role" value={party} onChange={e => setParty(e.target.value)}>
          <option value="">Choose declared participant</option>{selected.data.content.parties.map(p =>
            <PartyOption key={`${p.part_id}:${p.role}`} projectId={plan.project_id} id={p.part_id} role={p.role} />)}</select>
        {choices.map(op => <label key={op.key} className="block text-xs"><input type="checkbox"
          checked={operations.includes(op.key)} onChange={e => setOperations(e.target.checked
            ? [...operations, op.key] : operations.filter(n => n !== op.key))} /> {op.label}</label>)}
        <Button size="sm" disabled={selected.data.state !== 'Agreed' || !declared || operations.length === 0}
          onClick={() => { if (declared) save.mutate([{ op: 'binding_put', binding: {
            task, agreement_id: id, version, part_id: declared.part_id, role: declared.role, operations,
          } }]) }}>Adopt exact version</Button>
      </>}
    </fieldset>}
    {save.isError && <><ErrorLine error={save.error} /><Button variant="outline" size="sm"
      onClick={() => save.mutate(pending.current?.ops ?? [])}>Retry same command</Button>
      <Button variant="outline" size="sm" onClick={() => {
        pending.current = null; save.reset(); void client.invalidateQueries({ queryKey: ['workflows'] })
      }}>Reload current binding</Button></>}
  </section>
}
function PartyOption({ projectId, id, role }: { projectId: string; id: string; role: string }) {
  const part = useQuery(partQuery(projectId, id))
  return <option value={`${id}:${role}`} disabled={part.isError}>
    {role} · {part.data?.part.content.title ?? (part.isError ? 'Part unavailable' : 'Loading part…')}
  </option>
}
function PinnedOperations({ projectId, binding }: { projectId: string; binding: NonNullable<Plan['bindings']>[number] }) {
  const version = useQuery({ queryKey: ['projects', projectId, 'agreement', binding.agreement_id, binding.version],
    queryFn: () => getAgreement(projectId, binding.agreement_id, binding.version) })
  return <div className="text-xs text-faint-foreground">
    {version.data && <p>{documentOperations(version.data.content.openapi)
      .filter(op => binding.operations.includes(op.key)).map(op => op.label).join(', ')}</p>}
    {version.isError && <ErrorLine error={version.error} />}
    <p>Execution: Not recorded</p>
  </div>
}
