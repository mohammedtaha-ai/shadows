import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useRef, useState } from 'react'
import { Monitor, Moon, Sun } from 'lucide-react'
import { personQuery, savePerson, savePersonAnswer, type PersonPatch } from '@/api/person'
import { attemptFor, type Attempt } from '@/api/command-id'
import { ErrorLine } from '../error-line'

type Preferences = Pick<PersonPatch, 'language' | 'theme'>

const THEMES = [
  { id: 'system' as const, label: 'System', icon: Monitor },
  { id: 'light' as const, label: 'Light', icon: Sun },
  { id: 'dark' as const, label: 'Dark', icon: Moon },
]

export function GeneralPreferences() {
  const person = useQuery(personQuery)
  const client = useQueryClient()
  const [edited, setEdited] = useState<Partial<Preferences>>({})
  const attempts = useRef<Partial<Record<keyof Preferences, Attempt>>>({})
  const [failures, setFailures] = useState<Partial<Record<keyof Preferences, Error>>>({})
  const queued = useRef<Preferences[]>([])
  const value = { language: person.data?.language ?? null, theme: person.data?.theme ?? 'system', ...edited }

  const save = useMutation({
    retry: false,
    mutationFn: (input: Preferences) => {
      const key = 'theme' in input ? 'theme' : 'language'
      const attempt = attemptFor(attempts.current[key] ?? null, input)
      attempts.current[key] = attempt
      return savePerson(attempt.commandId, input)
    },
    onSuccess: async (answer, input) => {
      await savePersonAnswer(client, answer)
      const key = 'theme' in input ? 'theme' : 'language'
      delete attempts.current[key]
      setFailures(current => { const next = { ...current }; delete next[key]; return next })
      const later = queued.current.slice(1)
      setEdited(current => {
        const next = { ...current }
        // Keep later picks visible while an earlier answer updates the saved record.
        for (const key of ['theme', 'language'] as const) {
          if (key in input && !later.some(pick => key in pick)) delete next[key]
        }
        return next
      })
    },
    onError: (error, input) => {
      // A queued success must not hide another field's failure or discard its retry key.
      const key = 'theme' in input ? 'theme' : 'language'
      setFailures(current => ({ ...current, [key]: error }))
    },
    onSettled: () => {
      queued.current.shift()
      const next = queued.current[0]
      if (next) save.mutate(next)
    },
  })

  function mutateWith(preferences: Partial<Preferences>) {
    if (!person.data) return
    const last = queued.current.at(-1)
    if (last && last.theme === preferences.theme && last.language === preferences.language) return
    const waiting = queued.current.length > 0
    setEdited(current => waiting ? { ...current, ...preferences } : preferences)
    // Serialize one-field patches so the last pick also wins for the same field.
    queued.current.push(preferences)
    if (queued.current.length === 1) save.mutate(preferences)
  }

  function changeTheme(theme: 'system' | 'light' | 'dark') {
    mutateWith({ theme })
  }

  function changeLanguage(language: string | null) {
    mutateWith({ language })
  }

  if (person.error) return <ErrorLine error={person.error} />
  const disabled = !person.data

  return (
    <form aria-label="General preferences" onSubmit={e => e.preventDefault()} className="space-y-4 rounded-lg border border-border p-4">
      <div className="flex items-center justify-between">
        <h2 className="font-medium">Language and theme</h2>
        {save.isPending && (
          <span className="text-xs text-muted-foreground animate-pulse">Saving…</span>
        )}
      </div>
      {!person.data && <p role="status">Loading preferences…</p>}
      <fieldset disabled={disabled} className="grid gap-5">
        <label className="grid gap-2 text-sm">
          Language
          <select
            aria-label="Language"
            disabled={disabled}
            value={value.language ?? ''}
            className="rounded border border-input bg-background p-2"
            onChange={event => {
              const next = event.target.value
              if (next === '' || next === 'en') changeLanguage(next || null)
            }}
          >
            <option value="">Default (English)</option>
            <option value="en">English</option>
            {value.language && value.language !== 'en' && (
              <option value={value.language} disabled>Saved ({value.language})</option>
            )}
          </select>
        </label>

        <div className="grid gap-2">
          <span className="text-sm font-medium">Theme</span>
          <div role="radiogroup" aria-label="Theme" className="flex items-center justify-center gap-7 py-2">
            {THEMES.map(({ id, label, icon: Icon }) => {
              const active = value.theme === id
              return (
                <div key={id} className="flex flex-col items-center gap-2">
                  <button
                    type="button"
                    role="radio"
                    aria-checked={active}
                    aria-label={label}
                    disabled={disabled}
                    onClick={() => changeTheme(id)}
                    className={`group relative flex h-12 w-12 items-center justify-center rounded-full transition-all duration-200 cursor-pointer disabled:pointer-events-none disabled:opacity-50 ${
                      active
                        ? 'bg-primary/20 text-accent-text border border-primary/70 shadow-[0_0_20px_rgba(127,119,221,0.4)] scale-105'
                        : 'bg-foreground/5 hover:bg-foreground/10 border border-foreground/10 hover:border-foreground/25 text-muted-foreground hover:text-foreground backdrop-blur-md shadow-[0_4px_20px_rgba(0,0,0,0.15)] hover:-translate-y-0.5 hover:scale-105'
                    }`}
                  >
                    <Icon className="h-5 w-5 pointer-events-none" />
                  </button>
                  <span className={`text-xs ${active ? 'font-medium text-foreground' : 'text-muted-foreground'}`}>
                    {label}
                  </span>
                </div>
              )
            })}
          </div>
        </div>

        {(['theme', 'language'] as const).map(key => failures[key] && <ErrorLine key={key} error={failures[key]} />)}
      </fieldset>
    </form>
  )
}
