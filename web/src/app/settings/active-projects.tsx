// One job: how many projects the code index keeps active (§15.6), as a
// number the person edits and saves.

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useRef, useState } from 'react'
import { type CodeSettings, setActiveLimit } from '@/api/client'
import { type Attempt, attemptFor } from '@/api/command-id'
import { codeSettingsQuery } from '@/api/queries'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ErrorLine } from '../error-line'

/** The bounds the daemon holds the setting to; another value is refused. */
const MIN = 1
const MAX = 20

export function ActiveProjects() {
  const saved = useQuery(codeSettingsQuery)

  return (
    <section className="space-y-3">
      <div className="space-y-1">
        <h2 className="text-sm font-medium">Active projects</h2>
        <p className="text-xs text-muted-foreground">
          This many recently used projects are watched and their code index kept fresh. The others
          keep their index and answer as inactive.
        </p>
      </div>
      {saved.error !== null && <ErrorLine error={saved.error} />}
      {saved.data !== undefined && <LimitForm saved={saved.data} />}
    </section>
  )
}

function LimitForm({ saved }: { saved: CodeSettings }) {
  const queryClient = useQueryClient()
  // `null` until the person edits the field: it then follows the saved value,
  // changed elsewhere or refetched. Once edited, their number stands until
  // it is saved.
  const [edited, setEdited] = useState<string | null>(null)
  const draft = edited ?? String(saved.active_limit)
  const pending = useRef<Attempt | null>(null)

  const save = useMutation({
    mutationFn: ({ commandId, limit }: { commandId: string; limit: number }) =>
      setActiveLimit(commandId, limit),
    onSuccess: (settings) => {
      pending.current = null
      setEdited(null)
      queryClient.setQueryData(codeSettingsQuery.queryKey, settings)
    },
  })

  const limit = Number(draft)
  const valid = draft.trim() !== '' && Number.isInteger(limit) && limit >= MIN && limit <= MAX
  const unchanged = valid && limit === saved.active_limit

  const submit = (event: FormEvent) => {
    event.preventDefault()
    if (!valid || unchanged) return
    // A retry of the same number reuses its command id; another number is new.
    pending.current = attemptFor(pending.current, { active_limit: limit })
    save.mutate({ commandId: pending.current.commandId, limit })
  }

  return (
    <form onSubmit={submit} className="space-y-2">
      <div className="flex items-center gap-3">
        <Input
          type="number"
          inputMode="numeric"
          min={MIN}
          max={MAX}
          step={1}
          aria-label="Active projects"
          aria-invalid={!valid}
          value={draft}
          onChange={(e) => setEdited(e.target.value)}
          className="w-20"
        />
        <Button type="submit" size="sm" disabled={!valid || unchanged || save.isPending}>
          Save
        </Button>
      </div>
      {!valid && (
        <p className="text-xs text-destructive-foreground">
          A whole number from {MIN} to {MAX}.
        </p>
      )}
      {save.error !== null && <ErrorLine error={save.error} />}
    </form>
  )
}
