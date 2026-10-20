// One job: the agent page — a new agent's form, or one agent's name, provider,
// default model, environment and deletion.

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link, getRouteApi, useNavigate } from '@tanstack/react-router'
import { type FormEvent, useRef, useState } from 'react'
import {
  type AgentInput, agentsQuery, deleteAgent, providersQuery, saveAgent,
} from '@/api/aylul-profiles'
import { type Attempt, attemptFor } from '@/api/command-id'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ErrorLine } from '../error-line'
import { AgentEnv } from './agent-env'
import { DeleteZone } from './delete-zone'
import { Field, Select } from './form-field'

export function NewAgent() {
  return <AgentPage />
}

export function AgentSettings() {
  const { slug } = getRouteApi('/aylul/agents/$slug/settings').useParams()
  // Keyed, so one agent's unsaved edits never show under another.
  return <AgentPage key={slug} slug={slug} />
}

function AgentPage({ slug }: { slug?: string }) {
  const agents = useQuery(agentsQuery)
  const agent = agents.data?.agents.find((a) => a.slug === slug)
  const missing = slug !== undefined && agents.data !== undefined && agent === undefined
  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto max-w-2xl space-y-8 px-6 py-8">
        <header className="space-y-1">
          <h1 className="text-base font-medium">{slug === undefined ? 'New agent' : 'Agent settings'}</h1>
          <p className="text-xs text-faint-foreground">
            {agent !== undefined ? <>
              <span dir="auto">{agent.name}</span>
              <span className="ml-2 font-mono">{agent.slug}</span>
            </> : 'An agent runs on one provider and keeps its own conversations.'}
          </p>
        </header>
        {agents.error !== null && <ErrorLine error={agents.error} />}
        {missing ? (
          <p className="text-sm text-muted-foreground">
            This agent was deleted. Its conversations stay readable from their links.
          </p>
        ) : (slug === undefined || agent !== undefined) && <AgentForm slug={slug} />}
        {slug !== undefined && agent !== undefined && <AgentEnv slug={slug} />}
        {slug !== undefined && agent !== undefined && <DeleteAgent slug={slug} name={agent.name} />}
      </div>
    </div>
  )
}

function AgentForm({ slug }: { slug?: string }) {
  const providers = useQuery(providersQuery)
  const original = useQuery(agentsQuery).data?.agents.find((a) => a.slug === slug)
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const [edited, setEdited] = useState<AgentInput | null>(null)
  const pending = useRef<Attempt | null>(null)
  const first = providers.data?.[0]
  const value: AgentInput = edited ?? (original === undefined
    ? { name: '', provider: first?.slug ?? '', default_model: first?.models[0] ?? '' }
    : { name: original.name, provider: original.provider, default_model: original.default_model })
  const provider = providers.data?.find((p) => p.slug === value.provider)

  const save = useMutation({
    mutationFn: (input: AgentInput) => {
      pending.current = attemptFor(pending.current, { slug: slug ?? null, input })
      return saveAgent(slug ?? null, pending.current.commandId, input)
    },
    onSuccess: async (saved) => {
      pending.current = null
      setEdited(null)
      await queryClient.invalidateQueries({ queryKey: agentsQuery.queryKey })
      // A new agent opens its settings, where its variables can be added.
      if (slug === undefined) {
        await navigate({ to: '/aylul/agents/$slug/settings', params: { slug: saved.slug } })
      }
    },
  })
  const submit = (e: FormEvent) => {
    e.preventDefault()
    save.mutate(value)
  }

  if (providers.error !== null) return <ErrorLine error={providers.error} />
  if (providers.data === undefined) return null
  if (providers.data.length === 0) {
    return (
      <p className="text-sm text-muted-foreground">
        An agent needs a provider.{' '}
        <Link to="/aylul/providers" className="text-foreground underline underline-offset-4">
          Add a provider first
        </Link>
      </p>
    )
  }
  return (
    <form onSubmit={submit} className="space-y-4">
      <fieldset disabled={save.isPending} className="grid gap-4">
        <Field label="Name" hint={slug === undefined ? 'Its address is made from the name and never changes.' : undefined}>
          <Input aria-label="Agent name" value={value.name} required placeholder="AYLUL"
            onChange={(e) => setEdited({ ...value, name: e.target.value })} />
        </Field>
        <Field label="Provider">
          <Select aria-label="Provider" value={value.provider}
            onChange={(e) => {
              const next = providers.data.find((p) => p.slug === e.target.value)
              // The default model must be one of the new provider's.
              if (next) setEdited({ ...value, provider: next.slug, default_model: next.models[0] ?? '' })
            }}>
            {providers.data.map((p) => <option key={p.slug} value={p.slug}>{p.name}</option>)}
          </Select>
        </Field>
        <Field label="Default model" hint="New conversations start on it; each can switch to another of the provider's models.">
          <Select aria-label="Default model" value={value.default_model} className="font-mono"
            onChange={(e) => setEdited({ ...value, default_model: e.target.value })}>
            {provider?.models.map((id) => <option key={id} value={id}>{id}</option>)}
          </Select>
        </Field>
        {save.error !== null && <ErrorLine error={save.error} />}
        <div className="flex items-center gap-3">
          <Button type="submit" disabled={value.provider === '' || value.default_model === ''}>
            {save.isPending ? 'Saving…' : slug === undefined ? 'Create agent' : 'Save agent'}
          </Button>
          <p className="text-xs text-faint-foreground">Changes apply to new and resumed conversations.</p>
        </div>
      </fieldset>
    </form>
  )
}

function DeleteAgent({ slug, name }: { slug: string; name: string }) {
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  const deletion = useRef<Attempt | null>(null)
  const remove = useMutation({
    mutationFn: () => {
      deletion.current = attemptFor(deletion.current, { remove: slug })
      return deleteAgent(slug, deletion.current.commandId)
    },
    onSuccess: async () => {
      deletion.current = null
      await navigate({ to: '/aylul' })
      void queryClient.invalidateQueries({ queryKey: agentsQuery.queryKey })
    },
  })
  return (
    <DeleteZone
      title="Delete agent"
      summary="Its conversations stay readable, and its home folder stays on disk."
      action={`Delete ${name}?`}
      consequence={<>Its variables are removed and its running conversations stop. Sending in one of
        its conversations is refused from then on. Its slug, <code className="font-mono">{slug}</code>,
        stays taken.</>}
      pending={remove.isPending}
      error={remove.error}
      onConfirm={() => remove.mutate()}
    />
  )
}
