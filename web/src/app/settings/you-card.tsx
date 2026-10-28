import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useRef, useState, type FormEvent } from 'react'
import { personQuery, savePerson, savePersonAnswer, type Person, type PersonPatch } from '@/api/person'
import { attemptFor, type Attempt } from '@/api/command-id'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ErrorLine } from '../error-line'

type NameDraft = Partial<{ name: string; calls: string }>

export function YouCard() {
  const person = useQuery(personQuery)
  if (person.error) return <ErrorLine error={person.error} />
  return <YouForm saved={person.data} />
}

function YouForm({ saved }: { saved: Person | undefined }) {
  const queryClient = useQueryClient()
  const [accepted, setAccepted] = useState<{ before: Person | undefined; person: Person } | null>(null)
  // Keep the acknowledged record visible until the query observer delivers its cache update.
  const record = accepted && saved === accepted.before ? accepted.person : saved
  const [edited, setEdited] = useState<NameDraft>({})
  const value = { name: record?.name ?? '', calls: record?.aylul_calls_me ?? '', ...edited }
  const patch: PersonPatch = {}
  const name = value.name.trim() || null
  const calls = value.calls.trim() || null
  if (name !== (record?.name ?? null)) patch.name = name
  if (calls !== (record?.aylul_calls_me ?? null)) patch.aylul_calls_me = calls
  const unchanged = Object.keys(patch).length === 0
  const pending = useRef<Attempt | null>(null)
  const [invalid, setInvalid] = useState<string | null>(null)
  const save = useMutation({
    // A read sent before the save would answer the old record after it: cancel it, so it cannot
    // overwrite what was saved.
    onMutate: () => queryClient.cancelQueries({ queryKey: personQuery.queryKey }),
    mutationFn: ({ input }: { input: PersonPatch; draft: NameDraft }) => {
      pending.current = attemptFor(pending.current, input)
      return savePerson(pending.current.commandId, input)
    },
    onSuccess: async (person, { draft }) => {
      await savePersonAnswer(queryClient, person)
      pending.current = null
      setAccepted({ before: saved, person })
      // Only the submitted draft can be cleared; typing may have continued during the save.
      setEdited(current => {
        const next = { ...current }
        for (const key of ['name', 'calls'] as const) {
          if (current[key] === draft[key]) delete next[key]
        }
        return next
      })
    },
  })
  function submit(event: FormEvent) {
    event.preventDefault()
    if (!saved || save.isPending || unchanged) return
    if ([value.name, value.calls].some(text => [...text.trim()].length > 64)) return setInvalid('At most 64 characters.')
    if ([value.name, value.calls].some(text => /[\r\n]/.test(text))) return setInvalid('Names must be on one line.')
    setInvalid(null)
    save.mutate({ input: patch, draft: edited })
  }
  return <form onSubmit={submit} aria-label="You" className="space-y-4 rounded-lg border border-border p-4">
    <h2 className="font-medium">You</h2>
    <fieldset disabled={!saved} className="grid gap-4">
      <label className="grid gap-1 text-sm">Your name<Input aria-label="Your name" value={value.name} onChange={event => { const name = event.target.value; setEdited(current => ({ ...current, name })) }} /></label>
      <label className="grid gap-1 text-sm">What AYLUL calls you<Input aria-label="What AYLUL calls you" value={value.calls} onChange={event => { const calls = event.target.value; setEdited(current => ({ ...current, calls })) }} /></label>
      <p className="text-xs text-muted-foreground">AYLUL uses this name when it speaks to you. Without it, AYLUL uses your name.</p>
      {invalid && <p role="alert">{invalid}</p>}
      {save.error && <ErrorLine error={save.error} />}
      <Button type="submit" disabled={save.isPending || unchanged}>Save</Button>
    </fieldset>
  </form>
}
