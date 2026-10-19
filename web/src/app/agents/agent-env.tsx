// One job: one agent's environment variables (AYLUL spec section 10, per agent):
// their names, and adding, replacing or removing a value never shown again.

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useRef, useState } from 'react'
import { agentEnvQuery, removeAgentEnv, setAgentEnv, type EnvEntry } from '@/api/aylul-profiles'
import { type Attempt, attemptFor } from '@/api/command-id'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ErrorLine } from '../error-line'

export function AgentEnv({ slug }: { slug: string }) {
  const saved = useQuery(agentEnvQuery(slug))

  return (
    <section className="space-y-3">
      <div className="space-y-1">
        <h2 className="text-sm font-medium">Environment variables</h2>
        <p className="text-xs text-muted-foreground">
          Values are encrypted and never shown again. Changes apply to new and resumed conversations.
        </p>
      </div>
      {saved.error !== null && <ErrorLine error={saved.error} />}
      {saved.data !== undefined && <>
        <ul className="space-y-3">
          {saved.data.map((entry) => <EnvRow key={entry.name} entry={entry} slug={slug} />)}
        </ul>
        <EnvEditor slug={slug} />
      </>}
    </section>
  )
}

function EnvRow({ entry, slug }: { entry: EnvEntry; slug: string }) {
  const [replacing, setReplacing] = useState(false)
  const queryClient = useQueryClient()
  const commandId = useRef<string | null>(null)
  const remove = useMutation({
    mutationFn: () => {
      commandId.current ??= crypto.randomUUID()
      return removeAgentEnv(slug, entry.name, commandId.current)
    },
    onSuccess: () => {
      commandId.current = null
      void queryClient.invalidateQueries({ queryKey: agentEnvQuery(slug).queryKey })
    },
  })
  return (
    <li className="space-y-2 rounded-md border p-3">
      <div className="flex flex-wrap items-center gap-3 text-xs">
        <code className="font-medium">{entry.name}</code>
        <time className="text-muted-foreground" dateTime={entry.updated_at}>{entry.updated_at}</time>
        <Button size="sm" variant="outline" aria-label={`Replace ${entry.name}`}
          disabled={remove.isPending || replacing} onClick={() => setReplacing(true)}>Replace</Button>
        <Button size="sm" variant="outline" aria-label={`Remove ${entry.name}`}
          disabled={remove.isPending || replacing} onClick={() => remove.mutate()}>Remove</Button>
      </div>
      {remove.error !== null && <ErrorLine error={remove.error} />}
      {replacing && <EnvEditor slug={slug} name={entry.name} onDone={() => setReplacing(false)} />}
    </li>
  )
}

function EnvEditor({ slug, name, onDone }: { slug: string; name?: string; onDone?: () => void }) {
  const [draftName, setDraftName] = useState('')
  const [value, setValue] = useState('')
  const pending = useRef<Attempt | null>(null)
  const queryClient = useQueryClient()
  const save = useMutation({
    mutationFn: (request: { name: string; commandId: string; value: string }) =>
      setAgentEnv(slug, request.name, request.commandId, request.value),
    onSuccess: () => {
      pending.current = null
      setValue('')
      setDraftName('')
      save.reset()
      void queryClient.invalidateQueries({ queryKey: agentEnvQuery(slug).queryKey })
      onDone?.()
    },
  })
  const submit = (event: FormEvent) => {
    event.preventDefault()
    const variable = name ?? draftName
    if (variable === '' || value === '' || save.isPending) return
    pending.current = attemptFor(pending.current, { name: variable, value })
    save.mutate({ name: variable, value, commandId: pending.current.commandId })
  }
  return (
    <form onSubmit={submit} className="space-y-2">
      <div className="flex flex-wrap items-center gap-2">
        {name === undefined && <Input aria-label="Environment name" value={draftName}
          placeholder="NAME" autoComplete="off" disabled={save.isPending}
          onChange={(e) => setDraftName(e.target.value.toUpperCase())} className="w-44" />}
        <Input aria-label={name === undefined ? 'Environment value' : `Replace value for ${name}`}
          type="password" autoComplete="new-password" value={value} disabled={save.isPending}
          onChange={(e) => setValue(e.target.value)} className="w-56" />
        <Button type="submit" size="sm" disabled={value === '' || (name ?? draftName) === '' || save.isPending}>
          {name === undefined ? 'Add variable' : `Save ${name}`}
        </Button>
        {name !== undefined && <Button type="button" size="sm" variant="outline" disabled={save.isPending}
          onClick={onDone}>Cancel</Button>}
      </div>
      {save.error !== null && <ErrorLine error={save.error} />}
    </form>
  )
}
