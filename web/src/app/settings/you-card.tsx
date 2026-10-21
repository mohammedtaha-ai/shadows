import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useRef, useState, type FormEvent } from 'react'
import { personQuery, savePerson, type Person } from '@/api/person'
import { attemptFor, type Attempt } from '@/api/command-id'
import { writeSaved } from '@/api/saved-answer'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ErrorLine } from '../error-line'

export function YouCard() {
  const person = useQuery(personQuery)
  if (person.error) return <ErrorLine error={person.error} />
  return <YouForm saved={person.data} />
}

function YouForm({ saved }: { saved: Person | undefined }) {
  const queryClient = useQueryClient()
  const [edited, setEdited] = useState<{ name: string; calls: string } | null>(null)
  const value = edited ?? { name: saved?.name ?? '', calls: saved?.aylul_calls_me ?? '' }
  const pending = useRef<Attempt | null>(null)
  const [invalid, setInvalid] = useState<string | null>(null)
  const save = useMutation({
    // A read sent before the save would answer the old record after it: cancel it, so it cannot
    // overwrite what was saved.
    onMutate: () => queryClient.cancelQueries({ queryKey: personQuery.queryKey }),
    mutationFn: (input: Parameters<typeof savePerson>[1]) => {
      pending.current = attemptFor(pending.current, input)
      return savePerson(pending.current.commandId, input)
    },
    onSuccess: async person => {
      await writeSaved(queryClient, personQuery.queryKey, person)
      pending.current = null
      // The query passes the new record to this form a tick later; showing the saved values now,
      // not clearing them, keeps the old name from flashing back in between.
      setEdited({ name: person.name ?? '', calls: person.aylul_calls_me ?? '' })
    },
  })
  function submit(event: FormEvent) {
    event.preventDefault()
    if (!saved || save.isPending) return
    if ([value.name, value.calls].some(text => [...text.trim()].length > 64)) return setInvalid('At most 64 characters.')
    if ([value.name, value.calls].some(text => /[\r\n]/.test(text))) return setInvalid('Names must be on one line.')
    setInvalid(null)
    save.mutate({ name: value.name.trim() || null, aylul_calls_me: value.calls.trim() || null, language: saved.language ?? null, theme: saved.theme })
  }
  const unchanged = value.name === (saved?.name ?? '') && value.calls === (saved?.aylul_calls_me ?? '')
  return <form onSubmit={submit} aria-label="You" className="space-y-4 rounded-lg border border-border p-4">
    <h2 className="font-medium">You</h2>
    <fieldset disabled={!saved || save.isPending} className="grid gap-4">
      <label className="grid gap-1 text-sm">Your name<Input aria-label="Your name" value={value.name} onChange={event => setEdited({ ...value, name: event.target.value })} /></label>
      <label className="grid gap-1 text-sm">What AYLUL calls you<Input aria-label="What AYLUL calls you" value={value.calls} onChange={event => setEdited({ ...value, calls: event.target.value })} /></label>
      <p className="text-xs text-muted-foreground">AYLUL uses this name when it speaks to you. Without it, AYLUL uses your name.</p>
      {invalid && <p role="alert">{invalid}</p>}
      {save.error && <ErrorLine error={save.error} />}
      <Button type="submit" disabled={unchanged}>Save</Button>
    </fieldset>
  </form>
}
