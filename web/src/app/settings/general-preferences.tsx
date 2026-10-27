import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useRef, useState } from 'react'
import { Monitor, Moon, Sun } from 'lucide-react'
import { personQuery, savePerson, savePersonAnswer, type PersonInput } from '@/api/person'
import { attemptFor, type Attempt } from '@/api/command-id'
import { ErrorLine } from '../error-line'

type Preferences = Pick<PersonInput, 'language' | 'theme'>

const THEMES = [
  { id: 'system' as const, label: 'System', icon: Monitor },
  { id: 'light' as const, label: 'Light', icon: Sun },
  { id: 'dark' as const, label: 'Dark', icon: Moon },
]

export function GeneralPreferences() {
  const person = useQuery(personQuery)
  const client = useQueryClient()
  const [edited, setEdited] = useState<Partial<Preferences>>({})
  const attempt = useRef<Attempt | null>(null)
  const locked = useRef(false)
  const value = { language: person.data?.language ?? null, theme: person.data?.theme ?? 'system', ...edited }

  const save = useMutation({
    retry: false,
    mutationFn: (input: PersonInput) => {
      attempt.current = attemptFor(attempt.current, input)
      return savePerson(attempt.current.commandId, input)
    },
    onSuccess: async answer => {
      await savePersonAnswer(client, answer)
      attempt.current = null
      setEdited({})
    },
    onSettled: () => {
      locked.current = false
    },
  })

  function mutateWith(preferences: Partial<Preferences>) {
    if (!person.data || locked.current) return
    locked.current = true
    setEdited(preferences)
    save.mutate(preferences)
  }

  function changeTheme(theme: 'system' | 'light' | 'dark') {
    mutateWith({ theme })
  }

  function changeLanguage(language: string | null) {
    mutateWith({ language })
  }

  if (person.error) return <ErrorLine error={person.error} />
  const disabled = !person.data || save.isPending

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
          {/* Accessible hidden select for screen readers and form automation */}
          <select
            aria-label="Theme"
            disabled={disabled}
            value={value.theme}
            className="sr-only"
            tabIndex={-1}
            aria-hidden="true"
            onChange={event => {
              const next = event.target.value
              if (next === 'dark' || next === 'light' || next === 'system') changeTheme(next)
            }}
          >
            <option value="system">System</option>
            <option value="dark">Dark</option>
            <option value="light">Light</option>
          </select>

          {/* Three separate frosted glass circles */}
          <div role="radiogroup" aria-label="Theme options" className="flex items-center justify-center gap-7 py-2">
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

        {save.error && <ErrorLine error={save.error} />}
      </fieldset>
    </form>
  )
}
