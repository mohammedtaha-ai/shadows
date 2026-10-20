// One job: the form for one provider — its name, URL, write-only key, models
// and context window, and deleting it.

import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useRef, useState } from 'react'
import {
  type Provider, type ProviderInput, deleteProvider, providersQuery, removeProviderKey,
  saveProvider, setProviderKey,
} from '@/api/aylul-profiles'
import { type Attempt, attemptFor } from '@/api/command-id'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ErrorLine } from '../error-line'
import { DeleteZone } from './delete-zone'
import { Field, TextArea } from './form-field'
import { presetFor, type PresetGroup } from './provider-presets'

/** The models as typed, one per line, or why they cannot be saved. The daemon
 * checks again; this only spares a round trip. */
function parseModels(text: string): string[] | string {
  const ids = text.split(/[\n,]/).map((m) => m.trim()).filter((m) => m !== '')
  if (ids.length === 0 || ids.length > 20) return 'Models must be 1 to 20 ids.'
  const seen = new Set<string>()
  for (const id of ids) {
    if (id === 'default') return 'Models must not include default: that id is reserved.'
    if (seen.has(id)) return `Models must be listed once each: ${id} is listed twice.`
    // Printable, no spaces, as the daemon takes them.
    if (id.length > 128 || /[\s\p{Cc}]/u.test(id)) {
      return `Models must be at most 128 printable characters with no spaces: ${id}`
    }
    seen.add(id)
  }
  return ids
}

export function ProviderForm({ provider, group, onSaved }: {
  provider: Provider | null
  group: PresetGroup | null
  onSaved: (p: Provider | null) => void
}) {
  const queryClient = useQueryClient()
  const [name, setName] = useState(provider?.name ?? '')
  const [url, setUrl] = useState(provider?.base_url ?? '')
  const [preset, setPreset] = useState(presetFor(provider?.base_url ?? '') ?? group?.presets[0] ?? null)
  const [models, setModels] = useState(provider?.models.join('\n') ?? '')
  const [context, setContext] = useState(String(provider?.context_window ?? 200000))
  const [apiKey, setKey] = useState('')
  const [invalid, setInvalid] = useState<string | null>(null)
  const pending = useRef<Attempt | null>(null)
  const keyPending = useRef<Attempt | null>(null)
  const keyRemoval = useRef<Attempt | null>(null)
  const deletion = useRef<Attempt | null>(null)
  // A provider created by an answer that the key save then failed after: the
  // retry edits it rather than creating a second one.
  const created = useRef<string | null>(null)
  const refresh = () => queryClient.invalidateQueries({ queryKey: providersQuery.queryKey })

  const save = useMutation({
    mutationFn: async (input: ProviderInput) => {
      const slug = provider?.slug ?? created.current
      pending.current = attemptFor(pending.current, { slug, input })
      const saved = await saveProvider(slug, pending.current.commandId, input)
      created.current = saved.slug
      if (apiKey === '') return saved
      keyPending.current = attemptFor(keyPending.current, { slug: saved.slug, value: apiKey })
      await setProviderKey(saved.slug, keyPending.current.commandId, apiKey)
      return { ...saved, key_set: true }
    },
    onSuccess: async (saved) => {
      // The key is never shown again, so the field empties once it is stored.
      setKey('')
      pending.current = null
      keyPending.current = null
      await refresh()
      onSaved(saved)
    },
  })
  const removeKey = useMutation({
    mutationFn: (slug: string) => {
      keyRemoval.current = attemptFor(keyRemoval.current, { removeKey: slug })
      return removeProviderKey(slug, keyRemoval.current.commandId)
    },
    onSuccess: async () => {
      keyRemoval.current = null
      await refresh()
    },
  })
  const remove = useMutation({
    mutationFn: (slug: string) => {
      deletion.current = attemptFor(deletion.current, { remove: slug })
      return deleteProvider(slug, deletion.current.commandId)
    },
    onSuccess: async () => {
      deletion.current = null
      await refresh()
      onSaved(null)
    },
  })

  const submit = (e: FormEvent) => {
    e.preventDefault()
    const ids = parseModels(models)
    const size = Number(context)
    if (typeof ids === 'string') return setInvalid(ids)
    if (!Number.isInteger(size) || size < 1000 || size > 10_000_000) {
      return setInvalid('Context window must be a whole number from 1000 to 10000000.')
    }
    setInvalid(null)
    save.mutate({ name: group?.name ?? name, base_url: preset?.baseUrl ?? url, models: ids, context_window: size })
  }
  const busy = save.isPending || removeKey.isPending || remove.isPending

  return (
    <div className="space-y-8">
      <form onSubmit={submit} className="space-y-4">
        <fieldset disabled={busy} className="grid gap-4">
          {group ? <h3 className="font-medium">{group.name}</h3> : <Field label="Name">
            <Input aria-label="Provider name" value={name} required placeholder="MiniMax"
              onChange={(e) => setName(e.target.value)} />
          </Field>}
          {group && group.presets.length > 1 && <fieldset aria-label="Region" role="radiogroup" className="flex gap-4">
            <legend className="mb-2 text-sm">Region</legend>
            {group.presets.map(item => <label key={item.id} className="flex gap-2 text-sm">
              <input type="radio" name="provider-region" aria-label={item.region === 'china' ? 'China' : 'International'} checked={preset?.id === item.id} onChange={() => setPreset(item)} />
              {item.region === 'china' ? 'China' : 'International'}
            </label>)}
          </fieldset>}
          {!group && <Field label="URL" hint="Anthropic-compatible. http:// only for this machine.">
            <Input aria-label="Provider URL" type="url" value={url} required
              placeholder="https://api.example.com/anthropic" className="font-mono"
              onChange={(e) => setUrl(e.target.value)} />
          </Field>}
          <Field
            label={provider?.key_set === true ? 'API key (saved, enter a new one to replace it)' : 'API key'}
            hint="Stored encrypted and never shown again."
          >
            <div className="flex gap-2">
              <Input aria-label="API key" type="password" autoComplete="new-password" value={apiKey}
                placeholder={provider?.key_set === true ? '••••••••' : ''}
                onChange={(e) => setKey(e.target.value)} />
              {provider?.key_set === true && (
                <Button type="button" variant="outline" size="sm" className="h-8"
                  onClick={() => removeKey.mutate(provider.slug)}>
                  Remove key
                </Button>
              )}
            </div>
          </Field>
          <Field label="Models" hint="One model id per line, as the provider names it. The first is the usual default.">
            <TextArea aria-label="Models" value={models} required spellCheck={false}
              placeholder={'model-large\nmodel-small'} onChange={(e) => setModels(e.target.value)} />
          </Field>
          <Field label="Context window" hint="Tokens the provider's models accept.">
            <Input aria-label="Context window" type="number" inputMode="numeric" value={context}
              className="w-40" onChange={(e) => setContext(e.target.value)} />
          </Field>
          {invalid !== null && <p role="alert" className="text-xs text-destructive-foreground">{invalid}</p>}
          {save.error !== null && <ErrorLine error={save.error} />}
          {removeKey.error !== null && <ErrorLine error={removeKey.error} />}
          <div className="flex items-center gap-3">
            <Button type="submit">{save.isPending ? 'Saving…' : 'Save provider'}</Button>
            <p className="text-xs text-faint-foreground">Changes apply to new and resumed conversations.</p>
          </div>
        </fieldset>
      </form>
      {provider !== null && (
        <DeleteZone
          inline
          title="Delete provider"
          summary="Only a provider no agent uses can be deleted."
          action={`Delete ${provider.name}?`}
          consequence="Its URL, models and key are removed. An agent on it must move to another provider first."
          pending={remove.isPending}
          error={remove.error}
          onConfirm={() => remove.mutate(provider.slug)}
        />
      )}
    </div>
  )
}
