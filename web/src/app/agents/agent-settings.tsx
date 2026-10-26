// One job: agent editing inside Settings, including scoped environment and deletion.

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useRef, useState } from 'react'
import {
  type AgentInput, agentsQuery, deleteAgent, providersQuery, providerPresetsQuery, saveAgent,
} from '@/api/aylul-profiles'
import { type Attempt, attemptFor } from '@/api/command-id'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ErrorLine } from '../error-line'
import { AgentEnv } from './agent-env'
import { AgentExtensions } from './agent-extensions'
import { DeleteZone } from './delete-zone'
import { Field, Select } from './form-field'
import { providerTitle } from './provider-presets'
import { colorOf } from './character-colors'
import { ColorSwatches } from './color-swatches'
import { capabilities } from '@/desktop'
import { useCompanionBinding, refreshCompanionAppearance } from '@/desktop'
import { useOpenSettings } from '@/app/navigation'

export function AgentForm({ slug, onDone }: { slug: string | null; onDone: () => void }) {
  const agent = useQuery(agentsQuery).data?.agents.find(agent => agent.slug === slug)
  return <div className="space-y-6">
    <AgentFields slug={slug} onDone={onDone} />
    {slug !== null && agent && <><AgentExtensions slug={slug} /><AgentEnv slug={slug} /><DeleteAgent slug={slug} name={agent.name} onDone={onDone} /></>}
  </div>
}

function AgentFields({ slug, onDone }: { slug: string | null; onDone: () => void }) {
  const binding = useCompanionBinding()
  const openSettings = useOpenSettings()
  const providers = useQuery(providersQuery)
  const presets = useQuery(providerPresetsQuery)
  const original = useQuery(agentsQuery).data?.agents.find((a) => a.slug === slug)
  const queryClient = useQueryClient()
  const [edited, setEdited] = useState<AgentInput | null>(null)
  const pending = useRef<Attempt | null>(null)
  const first = providers.data?.[0]
  const value: AgentInput = edited ?? (original === undefined
    ? { name: '', provider: first?.slug ?? '', default_model: first?.models[0]?.id ?? '', character_color: colorOf(null).id }
    : { name: original.name, provider: original.provider, default_model: original.default_model, character_color: colorOf(original.character_color).id })
  const provider = providers.data?.find((p) => p.slug === value.provider)

  const save = useMutation({
    mutationFn: (input: AgentInput) => {
      pending.current = attemptFor(pending.current, { slug: slug ?? null, input })
      return saveAgent(slug ?? null, pending.current.commandId, input)
    },
    onSuccess: async () => {
      pending.current = null
      setEdited(null)
      if(capabilities.companion && slug !== null && binding?.agent === slug){
        try { await refreshCompanionAppearance() } catch { /* Native polling will refresh later. */ }
      }
      await queryClient.invalidateQueries({ queryKey: agentsQuery.queryKey })
      onDone()
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
        <button type="button" onClick={() => openSettings('providers')} className="text-foreground underline underline-offset-4">
          Add a provider first
        </button>
      </p>
    )
  }
  return (
    <form onSubmit={submit} className="space-y-4">
      <fieldset disabled={save.isPending} className="grid gap-4">
        <Field label="Name" hint={slug === null ? 'Its address is made from the name and never changes.' : undefined}>
          <Input aria-label="Agent name" value={value.name} required placeholder="AYLUL"
            onChange={(e) => setEdited({ ...value, name: e.target.value })} />
        </Field>
        <Field label="Provider">
          <Select aria-label="Provider" value={value.provider}
            onChange={(e) => {
              const next = providers.data.find((p) => p.slug === e.target.value)
              // The default model must be one of the new provider's.
              if (next) setEdited({ ...value, provider: next.slug, default_model: next.models[0]?.id ?? '' })
            }}>
            {providers.data.map((p) => <option key={p.slug} value={p.slug}>{providerTitle(p, presets.data ?? [])}</option>)}
          </Select>
        </Field>
        <Field label="Default model" hint="New conversations start on it; each can switch to another of the provider's models.">
          <Select aria-label="Default model" value={value.default_model} className="font-mono"
            onChange={(e) => setEdited({ ...value, default_model: e.target.value })}>
            {provider?.models.map(({ id }) => <option key={id} value={id}>{id}</option>)}
          </Select>
        </Field>
        <fieldset className="grid gap-2">
          <legend className="mb-2 text-xs text-muted-foreground">Character colour</legend>
          <ColorSwatches value={colorOf(value.character_color).id} disabled={save.isPending} onChange={id => setEdited({ ...value, character_color: id })} />
          <p className="text-xs text-muted-foreground">{!capabilities.companion
            ? 'AYLUL wears this colour on the desktop when this agent is the companion agent.'
            : slug !== null && binding?.agent === slug ? 'AYLUL wears this colour now.'
              : 'AYLUL wears this colour when this agent is the companion agent.'}</p>
        </fieldset>
        {save.error !== null && <ErrorLine error={save.error} />}
        <div className="flex items-center gap-3">
          <Button type="submit" disabled={value.provider === '' || value.default_model === ''}>
            {save.isPending ? 'Saving…' : slug === null ? 'Create agent' : 'Save agent'}
          </Button>
          <p className="text-xs text-faint-foreground">Changes apply to new and resumed conversations.</p>
        </div>
      </fieldset>
    </form>
  )
}

export function DeleteAgent({ slug, name, onDone, compact = false }: { slug: string; name: string; onDone: () => void; compact?: boolean }) {
  const queryClient = useQueryClient()
  const deletion = useRef<Attempt | null>(null)
  const remove = useMutation({
    mutationFn: () => {
      deletion.current = attemptFor(deletion.current, { remove: slug })
      return deleteAgent(slug, deletion.current.commandId)
    },
    onSuccess: async () => {
      deletion.current = null
      await queryClient.invalidateQueries({ queryKey: agentsQuery.queryKey })
      onDone()
    },
  })
  return (
    <DeleteZone
      inline
      compact={compact}
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
