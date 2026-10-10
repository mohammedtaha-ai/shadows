// One job: the project's standards (§23.2): Shadows' base, read only, and the project's additions, edited and saved as a new version.
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useRef, useState } from 'react'
import { type EffectiveStandards, type StandardsAdditions, saveStandardsAdditions } from '@/api/client'
import { type Attempt, attemptFor } from '@/api/command-id'
import { standardsQuery, stageQuery } from '@/api/queries'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'

const inputClass = 'w-full rounded-md border border-input bg-input-background px-3 py-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring'
const empty: StandardsAdditions = { parts: [], rules: [] }

function draftOf(content: StandardsAdditions) {
  return { parts: content.parts.map(part => ({ name: part.name, owns: part.owns })),
    rules: content.rules.map(rule => ({ id: rule.id, text: rule.text, partsText: rule.parts.join(', ') })) }
}
type Draft = ReturnType<typeof draftOf>
function contentOf(draft: Draft): StandardsAdditions {
  return { parts: draft.parts, rules: draft.rules.map(rule => ({ id: rule.id, text: rule.text,
    parts: rule.partsText.split(',').map(part => part.trim()).filter(Boolean) })) }
}
function same(a: Draft, b: Draft) { return JSON.stringify(a) === JSON.stringify(b) }

export function StandardsView({ projectId }: { projectId: string }) {
  const saved = useQuery(standardsQuery(projectId))
  return <section className="space-y-5">
    <h2 className="text-lg font-medium">Standards</h2>
    {saved.isPending && <p>Loading standards…</p>}
    {saved.error !== null && <ErrorLine error={saved.error} />}
    {saved.data !== undefined && <>
      <section className="space-y-2">
        <h3 className="text-sm font-medium">Mandatory parts</h3>
        {saved.data.base.parts.map(part => <div key={part.name} className="rounded-md border p-3 text-sm">
          <p><bdi>{part.name}</bdi> · {part.waivable ? 'waivable with a reason' : 'never waived'}</p>
          <p dir="auto">{part.owns}</p>
        </div>)}
      </section>
      <section className="space-y-2">
        <h3 className="text-sm font-medium">Rules</h3>
        {saved.data.base.rules.map(rule => <div key={rule.id} className="rounded-md border p-3 text-sm">
          <p><bdi>{rule.id}</bdi> · <bdi>{rule.parts.length > 0 ? rule.parts.join(', ') : 'all parts'}</bdi></p>
          <p dir="auto">{rule.text}</p>
        </div>)}
      </section>
      <section className="space-y-2 text-sm">
        <h3 className="font-medium">Contract template</h3>
        <ul className="list-disc space-y-1 ps-5">{saved.data.base.contract_template.rules.map((rule, index) => <li key={index} dir="auto">{rule}</li>)}</ul>
        <p dir="auto">{saved.data.base.contract_template.shape.join(', ')}</p>
      </section>
      <Editor key={projectId} projectId={projectId} saved={saved.data} />
    </>}
  </section>
}

function Editor({ projectId, saved }: { projectId: string; saved: EffectiveStandards }) {
  const client = useQueryClient()
  const [base, setBase] = useState(saved.additions)
  const [draft, setDraft] = useState(() => draftOf(saved.additions?.content ?? empty))
  const pending = useRef<Attempt | null>(null)
  if ((base?.number ?? 0) !== (saved.additions?.number ?? 0)) {
    setBase(saved.additions)
    if (same(draft, draftOf(base?.content ?? empty))) setDraft(draftOf(saved.additions?.content ?? empty))
  }
  const save = useMutation({
    mutationFn: ({ commandId, content }: { commandId: string; content: StandardsAdditions }) => saveStandardsAdditions(projectId, commandId, content),
    onSuccess: version => {
      pending.current = null
      // Keep a newer save learned through the journal while this response was in flight.
      client.setQueryData(standardsQuery(projectId).queryKey, (current: EffectiveStandards | undefined) =>
        (current?.additions?.number ?? 0) > version.number ? current : { ...(current ?? saved), additions: version })
      void client.invalidateQueries({ queryKey: stageQuery(projectId).queryKey })
    },
  })
  const submit = () => {
    const content = contentOf(draft)
    pending.current = attemptFor(pending.current, { content })
    save.mutate({ commandId: pending.current.commandId, content })
  }
  const addRule = () => {
    let number = 1
    while (draft.rules.some(rule => rule.id === `P${number}`)) number++
    setDraft({ ...draft, rules: [...draft.rules, { id: `P${number}`, text: '', partsText: '' }] })
  }
  const dirty = JSON.stringify(contentOf(draft)) !== JSON.stringify(contentOf(draftOf(saved.additions?.content ?? empty)))
  return <section className="space-y-3">
    <h3 className="text-sm font-medium">Project additions</h3>
    {draft.parts.map((part, index) => <div key={index} className="space-y-2 rounded-md border p-3">
      <label className="block space-y-1 text-sm"><span>Part name</span>
        <input dir="auto" aria-label={`Part ${index + 1} name`} value={part.name} className={inputClass}
          onChange={event => setDraft({ ...draft, parts: draft.parts.map((p, i) => i === index ? { ...p, name: event.target.value } : p) })} />
      </label>
      <label className="block space-y-1 text-sm"><span>Owns</span>
        <textarea dir="auto" aria-label={`Part ${index + 1} owns`} value={part.owns} className={inputClass}
          onChange={event => setDraft({ ...draft, parts: draft.parts.map((p, i) => i === index ? { ...p, owns: event.target.value } : p) })} />
      </label>
      <Button size="sm" variant="secondary" aria-label={`Remove part ${index + 1}`} onClick={() => setDraft({ ...draft, parts: draft.parts.filter((_, i) => i !== index) })}>Remove</Button>
    </div>)}
    {draft.rules.map((rule, index) => <div key={rule.id} className="space-y-2 rounded-md border p-3">
      <p className="text-sm"><bdi dir="auto">{rule.id}</bdi></p>
      <label className="block space-y-1 text-sm"><span>Rule text</span>
        <textarea dir="auto" aria-label={`Rule ${rule.id} text`} value={rule.text} className={inputClass}
          onChange={event => setDraft({ ...draft, rules: draft.rules.map((r, i) => i === index ? { ...r, text: event.target.value } : r) })} />
      </label>
      <label className="block space-y-1 text-sm"><span>Parts (comma-separated; empty means all)</span>
        <input dir="auto" aria-label={`Rule ${rule.id} parts`} value={rule.partsText} className={inputClass}
          onChange={event => setDraft({ ...draft, rules: draft.rules.map((r, i) => i === index ? { ...r, partsText: event.target.value } : r) })} />
      </label>
      <Button size="sm" variant="secondary" aria-label={`Remove rule ${rule.id}`} onClick={() => setDraft({ ...draft, rules: draft.rules.filter((_, i) => i !== index) })}>Remove</Button>
    </div>)}
    <div className="flex gap-2">
      <Button size="sm" variant="secondary" onClick={() => setDraft({ ...draft, parts: [...draft.parts, { name: '', owns: '' }] })}>Add part</Button>
      <Button size="sm" variant="secondary" onClick={addRule}>Add rule</Button>
    </div>
    <div className="flex items-center gap-3">
      <Button size="sm" onClick={submit} disabled={save.isPending || !dirty}>Save</Button>
      <span className="text-xs text-muted-foreground">{saved.additions == null ? 'Not saved yet' : `version ${saved.additions.number}`}</span>
    </div>
    {save.error !== null && <ErrorLine error={save.error} />}
  </section>
}
