// One agreement version's editor; refresh never overwrites unsaved fields.
import { useMutation } from '@tanstack/react-query'
import { useRef, useState } from 'react'
import { type AgreementVersion, type AgreementContent, type AgreementReview,
  editAgreement, reviewAgreement, agreeAgreement } from '@/api/client'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'
import { PartyPicker } from './party-picker'
import { OperationsEditor } from './operations-editor'

export function AgreementEditor({ version, projectId, onSaved, onReload }: {
  version: AgreementVersion; projectId: string; onSaved: (v: AgreementVersion) => void; onReload: () => void
}) {
  const [content, setContent] = useState(version.content)
  const [revision, setRevision] = useState(version.revision)
  const [json, setJson] = useState<string | null>(null)
  const [jsonError, setJsonError] = useState('')
  const [invalidFields, setInvalidFields] = useState<Record<string, boolean>>({})
  const [review, setReview] = useState<AgreementReview | null>(null)
  const pending = useRef<{ command: string; revision: number; content: AgreementContent } | null>(null)
  const approval = useRef<string | null>(null)
  const readonly = version.state === 'Agreed'
  const save = useMutation({ mutationFn: () => {
    pending.current ??= { command: crypto.randomUUID(), revision, content: structuredClone(content) }
    const request = pending.current
    return editAgreement(projectId, version.agreement_id, request.command, request.revision, request.content)
  }, onSuccess: v => {
    pending.current = null; setRevision(v.revision); setContent(v.content); setReview(null)
    if (json !== null) setJson(JSON.stringify(v.content, null, 2))
    onSaved(v)
  } })
  const inspect = useMutation({ mutationFn: () => reviewAgreement(projectId, version.agreement_id),
    onSuccess: v => { setReview(v); approval.current = null } })
  const agree = useMutation({ mutationFn: () => {
    if (!review) throw new Error('Review the current Draft first')
    approval.current ??= crypto.randomUUID()
    return agreeAgreement(projectId, version.agreement_id, approval.current, review.revision, review.review_id)
  }, onSuccess: v => { approval.current = null; onSaved(v); onReload() } })
  const changed = JSON.stringify(content) !== JSON.stringify(version.content)
  const locked = readonly || save.isPending || save.isError
  const fieldsLocked = locked || json !== null
  const field = (label: string, key: 'purpose' | 'behavior' | 'capability') => <label className="block">{label}
    <textarea className="block w-full rounded border border-border p-2" aria-label={label}
      disabled={fieldsLocked} value={content[key]} onChange={e => setContent({ ...content, [key]: e.target.value })} /></label>
  return <section className="space-y-4 rounded-xl border border-border p-5">
    <h2 className="text-lg">{version.content.capability} · v{version.version} · {version.state}</h2>
    <p className="text-xs text-faint-foreground">Written by {version.writer.kind} · {version.writer.id}
      {version.reason && ` · ${version.reason}`}</p>
    {version.revision !== revision && <p>Server revision changed. Reload to inspect it; your text is retained.</p>}
    {field('Capability', 'capability')}{field('Purpose', 'purpose')}{field('Shared behavior', 'behavior')}
    <label className="block">Acceptance items<textarea className="block w-full rounded border border-border p-2"
      aria-label="Acceptance items" disabled={fieldsLocked} value={content.acceptance.join('\n')}
      onChange={e => setContent({ ...content, acceptance: e.target.value.split('\n') })} /></label>
    <PartyPicker projectId={projectId} parties={content.parties} disabled={fieldsLocked}
      onChange={parties => setContent({ ...content, parties })} />
    <Button variant="outline" onClick={() => {
      if (json === null) setJson(JSON.stringify(content, null, 2)); else { setJson(null); setJsonError('') }
    }}>{json === null ? 'JSON view' : 'Structured view'}</Button>
    {json === null ? <OperationsEditor value={content.openapi} readOnly={locked}
      onValidity={(label, invalid) => setInvalidFields(fields => ({ ...fields, [label]: invalid }))}
      onChange={openapi => setContent({ ...content, openapi })} /> : <label className="block">Canonical contract JSON
      <textarea className="min-h-80 w-full rounded border border-border p-3 font-mono text-xs"
        aria-label="Canonical contract JSON" disabled={locked} value={json} onChange={e => {
          setJson(e.target.value)
          try {
            const parsed: unknown = JSON.parse(e.target.value)
            if (!isContent(parsed)) throw new Error('Invalid content shape')
            setContent(parsed); setJsonError('')
          }
          catch { setJsonError('Invalid JSON; fix it before saving') }
        }} /></label>}
    {jsonError && <p role="alert">{jsonError}</p>}
    {version.issues.length > 0 && <ul className="text-sm text-danger">{version.issues.map((p, i) =>
      <li key={i}>{p.path}: {p.message}</li>)}</ul>}
    {save.isError && <ErrorLine error={save.error} />}
    {inspect.isError && <ErrorLine error={inspect.error} />}
    {agree.isError && <ErrorLine error={agree.error} />}
    <div className="flex gap-2">
      {!readonly && <Button disabled={save.isPending || !!jsonError || Object.values(invalidFields).some(Boolean)} onClick={() => save.mutate()}>Save Draft</Button>}
      <Button variant="outline" onClick={onReload}>Reload</Button>
      {!readonly && <Button variant="outline" disabled={changed || save.isPending || save.isError}
        onClick={() => inspect.mutate()}>Review impact</Button>}
    </div>
    {review && <section className="space-y-3 rounded border border-border p-4">
      <h3>Impact review · {review.compatibility}</h3>
      <p>{review.limits.join('. ')}</p>
      <pre className="overflow-x-auto text-xs">{JSON.stringify(review.changes, null, 2)}</pre>
      <ul>{review.parties.map(p => <li key={`${p.party.part_id}:${p.party.role}`}>
        {p.title} · {p.party.role} · {p.change} · revision {p.revision}</li>)}</ul>
      {review.participants.map((p, i) => <article key={i} className="rounded border border-border p-3">
        <strong>{p.participant.title} · T{p.participant.binding.task}</strong>
        <p>{p.participant.current ? 'Current' : 'Historical'} · pinned v{p.participant.binding.version}
          · {p.affected ? 'Affected' : 'No registered impact'} · {p.execution}</p>
        <p>{p.next_action}</p><pre className="overflow-x-auto text-xs">{JSON.stringify(p.changes, null, 2)}</pre>
      </article>)}
      <Button disabled={changed || agree.isPending || version.issues.length > 0}
        onClick={() => agree.mutate()}>Agree reviewed version</Button>
      <p className="text-xs">Agreement keeps every participant's current version pin.</p>
    </section>}
  </section>
}
function isContent(value: unknown): value is AgreementContent {
  if (typeof value !== 'object' || value === null) return false
  const v = value as Record<string, unknown>
  return ['capability', 'purpose', 'behavior'].every(key => typeof v[key] === 'string') &&
    Array.isArray(v.acceptance) && v.acceptance.every(a => typeof a === 'string') &&
    Array.isArray(v.parties) && v.parties.every(p => typeof p === 'object' && p !== null &&
      typeof p.part_id === 'string' && (p.role === 'provides' || p.role === 'uses')) && 'openapi' in v
}
