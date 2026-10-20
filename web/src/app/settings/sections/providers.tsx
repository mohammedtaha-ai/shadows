import { useQuery } from '@tanstack/react-query'
import { useState } from 'react'
import { providersQuery } from '@/api/aylul-profiles'
import { Button } from '@/components/ui/button'
import { ErrorLine } from '../../error-line'
import { ProviderForm } from '../../agents/provider-form'
import { ProviderPicker } from '../../agents/provider-picker'
import { PRESETS, presetFor, presetGroups, type PresetGroup } from '../../agents/provider-presets'

export function ProvidersSection() {
  const providers = useQuery(providersQuery)
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
          const preset = presetFor(item.base_url)
          const name = preset ? `${preset.name}${preset.region === 'china' ? ' China' : ''}` : item.name
          return <li key={item.slug} className="flex flex-wrap items-center gap-3 py-3">
            <span className="flex-1">{name}</span>
            <span className="text-xs text-muted-foreground">{item.key_set ? `${item.models.length} ${item.models.length === 1 ? 'model' : 'models'}` : 'No key'}</span>
            <span className="text-xs text-muted-foreground">{item.key_set ? 'connected' : 'not set'}</span>
            <Button variant="outline" size="sm" onClick={() => {
              setSlug(item.slug)
              setGroup(presetGroups(PRESETS).find(group => group.name === preset?.name) ?? null)
              setStep('setup')
            }}>{item.key_set ? 'Edit' : 'Set up'}</Button>
          </li>
        })}
      </ul>}
    </>}
    {step === 'choose' && <ProviderPicker onChoose={group => { setGroup(group); setStep('setup') }} />}
    {step === 'setup' && <ProviderForm key={slug ?? group?.name ?? 'custom'} provider={provider} group={group} onSaved={() => setStep('list')} />}
  </div>
}
