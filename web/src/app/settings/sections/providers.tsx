import { useQuery } from '@tanstack/react-query'
import { useState } from 'react'
import { providersQuery, providerPresetsQuery } from '@/api/aylul-profiles'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../../error-line'
import { ProviderForm } from '@/app/agents'
import { ProviderPicker } from '@/app/agents'
import { presetOf, providerTitle, presetGroups, type PresetGroup } from '@/app/agents'

export function ProvidersSection() {
  const providers = useQuery(providersQuery)
  const presets = useQuery(providerPresetsQuery)
  const [step, setStep] = useState<'list' | 'choose' | 'setup'>('list')
  const [slug, setSlug] = useState<string | null>(null)
  const [group, setGroup] = useState<PresetGroup | null>(null)
  const provider = providers.data?.find(item => item.slug === slug) ?? null
  return <div className="space-y-4">
    {providers.error && <ErrorLine error={providers.error} />}
    {step !== 'list' && <Button type="button" variant="ghost" aria-label="Back" onClick={() => setStep(step === 'setup' && slug === null ? 'choose' : 'list')}>←</Button>}
    {step === 'list' && <>
      <Button type="button" onClick={() => { setSlug(null); setStep('choose') }}>Add provider</Button>
      {providers.data && <ul aria-label="Saved providers" className="divide-y divide-border">
        {providers.data.map(item => {
          const preset = presetOf(presets.data ?? [], item)
          const name = providerTitle(item, presets.data ?? [])
          return <li key={item.slug} className="flex flex-wrap items-center gap-3 py-3">
            <span className="flex-1">{name}</span>
            <span className="text-xs text-muted-foreground">{item.key_set ? `${item.models.length} ${item.models.length === 1 ? 'model' : 'models'}` : 'No key'}</span>
            <span className="text-xs text-muted-foreground">{item.key_set ? 'connected' : 'not set'}</span>
            <Button variant="outline" size="sm" disabled={!presets.data} onClick={() => {
              setSlug(item.slug)
              setGroup(presetGroups(presets.data ?? []).find(group => group.name === preset?.name) ?? null)
              setStep('setup')
            }}>{item.key_set ? 'Edit' : 'Set up'}</Button>
          </li>
        })}
      </ul>}
    </>}
    {step === 'choose' && <ProviderPicker onChoose={group => { setGroup(group); setStep('setup') }} />}
    {presets.error && <ErrorLine error={presets.error} />}
    {step === 'setup' && <ProviderForm key={slug ?? group?.name ?? 'custom'} provider={provider} group={group} onSaved={() => setStep('list')} />}
  </div>
}
