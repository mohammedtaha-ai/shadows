import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useRef, useState, type FormEvent } from 'react'
import { personQuery, savePerson, savePersonAnswer, type Person, type PersonInput } from '@/api/person'
import { attemptFor, type Attempt } from '@/api/command-id'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'

type Preferences = Pick<PersonInput, 'language' | 'theme'>

export function GeneralPreferences() {
  const person = useQuery(personQuery)
  const client = useQueryClient()
  const [edited, setEdited] = useState<Partial<Preferences>>({})
  const attempt = useRef<Attempt | null>(null)
  const locked = useRef(false)
  const value = { language: person.data?.language ?? null, theme: person.data?.theme ?? 'system', ...edited }
  const save = useMutation({ retry: false,
    mutationFn: (input: PersonInput) => {
      attempt.current = attemptFor(attempt.current, input)
      return savePerson(attempt.current.commandId, input)
    },
    onSuccess: async answer => {
      await savePersonAnswer(client, answer)
      attempt.current = null; setEdited({})
    },
    onSettled: () => { locked.current = false },
  })
  function submit(event: FormEvent) {
    event.preventDefault()
    const latest = client.getQueryData<Person>(personQuery.queryKey)
    if (!latest || locked.current) return
    locked.current = true
    save.mutate({ name: latest.name ?? null, aylul_calls_me: latest.aylul_calls_me ?? null,
      language: latest.language ?? null, theme: latest.theme, ...edited })
  }
  if (person.error) return <ErrorLine error={person.error} />
  const disabled = !person.data || save.isPending
  const unchanged = value.language === (person.data?.language ?? null) && value.theme === person.data?.theme
  return <form aria-label="General preferences" onSubmit={submit} className="space-y-4 rounded-lg border border-border p-4">
    <h2 className="font-medium">Language and theme</h2>
    {!person.data && <p role="status">Loading preferences…</p>}
    <fieldset disabled={disabled} className="grid gap-4">
      <label className="grid gap-2 text-sm">Language
        <select aria-label="Language" disabled={disabled} value={value.language ?? ''} className="rounded border border-input bg-background p-2" onChange={event => {
          const next = event.target.value
          if (next === '' || next === 'en') setEdited(current => ({ ...current, language: next || null }))
        }}>
          <option value="">Default (English)</option><option value="en">English</option>
          {value.language && value.language !== 'en' && <option value={value.language} disabled>Saved ({value.language})</option>}
        </select>
      </label>
      <label className="grid gap-2 text-sm">Theme
        <select aria-label="Theme" disabled={disabled} value={value.theme} className="rounded border border-input bg-background p-2" onChange={event => {
          const next = event.target.value
          if (next === 'dark' || next === 'light' || next === 'system') setEdited(current => ({ ...current, theme: next }))
        }}><option value="system">System</option><option value="dark">Dark</option><option value="light">Light</option></select>
      </label>
      {save.error && <ErrorLine error={save.error} />}
      <Button type="submit" disabled={disabled || unchanged}>{save.isPending ? 'Saving preferences…' : 'Save preferences'}</Button>
    </fieldset>
  </form>
}
