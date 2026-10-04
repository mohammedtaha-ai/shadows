// Browser entry for shared agreement identities and exact versions.
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link, useParams, useSearch, useNavigate } from '@tanstack/react-router'
import { useRef, useState } from 'react'
import { listAgreements, getAgreement, startAgreement } from '@/api/client'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'
import { AgreementEditor } from './agreement-editor'

export function AgreementsPage() {
  const { projectId } = useParams({ from: '/projects/$projectId/agreements' })
  const { agreement, version } = useSearch({ from: '/projects/$projectId/agreements' })
  return <AgreementsView key={projectId} projectId={projectId} id={agreement} number={version} />
}
function AgreementsView({ projectId, id, number }: { projectId: string; id?: string; number?: number }) {
  const client = useQueryClient(), navigate = useNavigate()
  const list = useQuery({ queryKey: ['projects', projectId, 'agreements'], queryFn: () => listAgreements(projectId) })
  const selected = useQuery({ queryKey: ['projects', projectId, 'agreement', id, number],
    queryFn: () => getAgreement(projectId, id!, number), enabled: id !== undefined })
  const [reload, setReload] = useState(0)
  const [reason, setReason] = useState('')
  const [title, setTitle] = useState('Login')
  const command = useRef<{ kind: 'new' | 'continue'; id: string } | null>(null)
  const start = useMutation({ mutationFn: (kind: 'new' | 'continue') => {
    if (command.current && command.current.kind !== kind) throw new Error('Retry the pending action first')
    command.current ??= { kind, id: crypto.randomUUID() }
    return kind === 'continue'
      ? startAgreement(projectId, command.current.id, undefined, id, reason)
      : startAgreement(projectId, command.current.id, { capability: title, purpose: '', behavior: '',
        acceptance: [], parties: [], openapi: { openapi: '3.1.0', info: { title, version: '1' }, paths: {} } })
  }, onSuccess: async v => {
    client.setQueryData(['projects', projectId, 'agreement', v.agreement_id, undefined], v)
    await client.invalidateQueries({ queryKey: ['projects', projectId, 'agreements'] })
    await navigate({ to: '/projects/$projectId/agreements', params: { projectId }, search: { agreement: v.agreement_id } })
    command.current = null
  } })
  return <main className="flex-1 overflow-y-auto p-6"><div className="mx-auto max-w-5xl space-y-5">
    <h1 className="text-lg font-semibold">Shared API contracts</h1>
    <Link to="/projects/$projectId/workspace" params={{ projectId }}>Workspace</Link>
    {list.isError && <ErrorLine error={list.error} />}
    <nav className="flex flex-wrap gap-4" aria-label="Contracts">{list.data?.map(v =>
      <Link key={v.agreement_id} to="/projects/$projectId/agreements" params={{ projectId }}
        search={{ agreement: v.agreement_id }}>{v.content.capability} · v{v.version} · {v.state}</Link>)}</nav>
    <div className="flex gap-2"><input className="rounded border border-border p-2" aria-label="New contract name"
      value={title} disabled={start.isPending || start.isError} onChange={e => setTitle(e.target.value)} />
      <Button onClick={() => start.mutate('new')} disabled={start.isPending}>New contract</Button></div>
    {start.isError && <ErrorLine error={start.error} />}
    {selected.isError && <ErrorLine error={selected.error} />}
    {selected.data && <>
      <label>Version <select aria-label="Contract version" value={selected.data.version} onChange={e => {
        void navigate({ to: '/projects/$projectId/agreements', params: { projectId },
          search: { agreement: id, version: Number(e.target.value) } })
      }}>{Array.from({ length: list.data?.find(v => v.agreement_id === id)?.version ?? selected.data.version }, (_, i) =>
        <option key={i + 1} value={i + 1}>v{i + 1}</option>)}</select></label>
      {selected.data.state === 'Agreed' && <div className="flex gap-2">
        <input className="rounded border border-border p-2" aria-label="Change reason" value={reason}
          disabled={start.isPending || start.isError} onChange={e => setReason(e.target.value)} placeholder="Why change this agreement?" />
        <Button disabled={!reason.trim() || start.isPending} onClick={() => start.mutate('continue')}>Start next Draft</Button>
      </div>}
      <AgreementEditor key={`${id}:${selected.data.version}:${reload}`} version={selected.data} projectId={projectId}
        onSaved={v => {
          client.setQueryData(['projects', projectId, 'agreement', id, number], v)
          void client.invalidateQueries({ queryKey: ['projects', projectId, 'agreements'] })
        }} onReload={() => { start.reset(); void selected.refetch().then(() => setReload(n => n + 1)) }} />
    </>}
  </div></main>
}
