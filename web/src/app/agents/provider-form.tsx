// One job: the form for one provider — its name, URL, write-only key, models
// and context window, and deleting it.

import { useMutation, useQueryClient } from '@tanstack/react-query'
import { type FormEvent, useRef, useState } from 'react'
import {
  type Provider, type ProviderInput, deleteProvider, providersQuery, removeProviderKey,
  saveProvider, setProviderKey, testProvider, testSavedProvider,
} from '@/api/aylul-profiles'
import { type Attempt, attemptFor } from '@/api/command-id'
import { ApiError } from '@/api/error'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ErrorLine } from '../error-line'
import { DeleteZone } from './delete-zone'
import { Field } from './form-field'
import type { PresetGroup } from './provider-presets'
import { ProviderModels, type ModelPick } from './provider-models'
import { testLine } from './provider-test'

// Shown only while the field holds text, so adding or clearing the id removes it.
const UNADDED = 'You have an unadded model id. Add it or clear the field before saving.'

export function ProviderForm({ provider, group, onSaved }: {
  provider: Provider | null
  group: PresetGroup | null
  onSaved: (p: Provider | null) => void
}) {
  const queryClient = useQueryClient()
  const [name, setName] = useState(provider?.name ?? '')
  const [url, setUrl] = useState(provider?.base_url ?? '')
  const [preset, setPreset] = useState(group?.presets.find(preset => preset.id === provider?.preset) ?? group?.presets[0] ?? null)
  const [picked, setPicked] = useState<ModelPick[]>(provider?.models.map(model => ({ id: model.id, contextWindow: model.context_window })) ?? [])
  const [manual, setManual] = useState('')
  const [listed, setListed] = useState<{ id: string; context_window?: number | null }[]>(provider?.models ?? [])
  const [label, setLabel] = useState(provider?.label ?? '')
  const [testedKey, setTestedKey] = useState('')
  const [line, setLine] = useState<ReturnType<typeof testLine> | null>(null)
  const [apiKey, setKey] = useState('')
  const [invalid, setInvalid] = useState<string | null>(null)
  const [refusal, setRefusal] = useState<{ field: string; message: string } | null>(null)
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
      const key = apiKey || testedKey
      if (key === '') return saved
      keyPending.current = attemptFor(keyPending.current, { slug: saved.slug, value: key })
      await setProviderKey(saved.slug, keyPending.current.commandId, key)
      return { ...saved, key_set: true }
    },
    onSuccess: async (saved) => {
      // The key is never shown again, so the field empties once it is stored.
      setKey(''); setTestedKey('')
      pending.current = null
      keyPending.current = null
      await refresh()
      onSaved(saved)
    },
    onError: (error) => {
      if (error instanceof ApiError && error.problem.kind === 'daemon' && error.problem.code === 'PROFILE_INVALID') pending.current = null
      if (error instanceof ApiError && error.problem.kind === 'daemon' && error.problem.field) {
        setRefusal({ field: error.problem.field, message: error.message })
      }
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

  const test = useMutation({
    mutationFn: (key: string) => {
      const slug = provider?.slug ?? created.current
      return slug ? testSavedProvider(slug, key || null) : testProvider({ api_key: key, ...(preset ? { preset: preset.id } : { base_url: url }) })
    },
    onSuccess: (outcome, key) => {
      setLine(testLine(outcome, group?.name ?? name))
      setListed(outcome.ok && outcome.listed ? outcome.models : [])
      if (outcome.ok) { setTestedKey(key); setKey('') }
    },
  })
  const submit = (event: FormEvent) => {
    event.preventDefault()
    if (manual !== '') return setInvalid(UNADDED)
    setInvalid(null)
    setRefusal(null)
    save.mutate({ ...(preset ? { preset: preset.id } : { preset: null, name, base_url: url }), label: label.trim() || null,
      models: picked.map(model => ({ id: model.id, ...(model.contextWindow != null && { context_window: model.contextWindow }) })) })
  }
  const busy = save.isPending || removeKey.isPending || remove.isPending || test.isPending
  const credentialsChanged = () => { setTestedKey(''); setLine(null); setListed([]) }
  const clearRefusal = (field: string) => setRefusal(current => current?.field === field ? null : current)
  const changeModels = (models: ModelPick[]) => {
    setRefusal(current => {
      if (!current) return null
      if (current.field === 'models') return null
      const match = /^models\[(\d+)\]\.(id|context_window)$/.exec(current.field)
      if (!match) return current
      const index = Number(match[1])
      const previous = picked[index]
      if (!previous) return null
      const nextIndex = models.indexOf(previous)
      if (nextIndex >= 0) return { ...current, field: `models[${nextIndex}].${match[2]}` }
      const next = models[index]
      return models.length === picked.length && next && previous.id === next.id && match[2] === 'id' ? current : null
    })
    setPicked(models)
  }
  const modelRefusal = refusal && /^models\[(\d+)\]\.(id|context_window)$/.exec(refusal.field)
  const knownRefusal = refusal && (['models', 'label', ...(!group ? ['name', 'base_url'] : [])].includes(refusal.field) || (modelRefusal && Number(modelRefusal[1]) < picked.length))

  return (
    <div className="space-y-8">
      <form onSubmit={submit} noValidate className="space-y-4">
        <fieldset disabled={busy} className="grid gap-4">
          {group ? <h3 className="font-medium">{group.name}</h3> : <Field label="Name">
            <Input aria-label="Provider name" value={name} required placeholder="MiniMax"
              onChange={(e) => { setName(e.target.value); clearRefusal('name') }} />
            {refusal?.field === 'name' && <p role="alert">{refusal.message}</p>}
          </Field>}
          {group && group.presets.length > 1 && <fieldset aria-label="Region" role="radiogroup" className="flex gap-4">
            <legend className="mb-2 text-sm">Region</legend>
            {group.presets.map(item => <label key={item.id} className="flex gap-2 text-sm">
              <input type="radio" name="provider-region" aria-label={item.region === 'china' ? 'China' : 'International'} checked={preset?.id === item.id} onChange={() => { setPreset(item); credentialsChanged() }} />
              {item.region === 'china' ? 'China' : 'International'}
            </label>)}
          </fieldset>}
          {!group && <Field label="URL" hint="Anthropic-compatible. http:// only for this machine.">
            <Input aria-label="Provider URL" type="url" value={url} required
              placeholder="https://api.example.com/anthropic" className="font-mono"
              onChange={(e) => { setUrl(e.target.value); clearRefusal('base_url'); credentialsChanged() }} />
            {refusal?.field === 'base_url' && <p role="alert">{refusal.message}</p>}
          </Field>}
          <Field label="Label" hint="Shown in parentheses, to tell two keys apart.">
            <Input aria-label="Label" value={label} onChange={event => { setLabel(event.target.value); clearRefusal('label') }} />
            {refusal?.field === 'label' && <p role="alert">{refusal.message}</p>}
          </Field>
          <Field
            label={provider?.key_set === true ? 'API key (saved, enter a new one to replace it)' : 'API key'}
            hint="Stored encrypted and never shown again."
          >
            <div className="flex gap-2">
              <Input aria-label="API key" type="password" autoComplete="new-password" value={apiKey}
                placeholder={provider?.key_set === true ? '••••••••' : ''}
                onChange={(e) => { setKey(e.target.value); credentialsChanged() }} />
              <Button type="button" variant="outline" disabled={!apiKey && !testedKey && !provider?.key_set} onClick={() => test.mutate(apiKey || testedKey)}>Test connection</Button>
              {provider?.key_set === true && (
                <Button type="button" variant="outline" size="sm" className="h-8"
                  onClick={() => removeKey.mutate(provider.slug)}>
                  Remove key
                </Button>
              )}
            </div>
          </Field>
          {line && <p role={line.kind === 'error' ? 'alert' : 'status'}>{line.text}</p>}
          {test.error && <ErrorLine error={test.error} />}
          <ProviderModels listed={listed} picked={picked} onChange={changeModels} manual={manual} refusal={refusal}
            onManualChange={value => { setManual(value); if (invalid === UNADDED) setInvalid(null) }} />
          {invalid !== null && <p role="alert" className="text-xs text-destructive-foreground">{invalid}</p>}
          {refusal && !knownRefusal && <p role="alert">{refusal.message}</p>}
          {save.error !== null && !(save.error instanceof ApiError && save.error.problem.kind === 'daemon' && save.error.problem.field) && <ErrorLine error={save.error} />}
          {removeKey.error !== null && <ErrorLine error={removeKey.error} />}
          <div className="flex items-center gap-3">
            <Button type="submit">{save.isPending ? 'Saving…' : provider ? 'Save provider' : `Add ${picked.length} ${picked.length === 1 ? 'model' : 'models'}`}</Button>
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
