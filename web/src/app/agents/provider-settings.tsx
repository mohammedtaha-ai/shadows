// One job: the Providers page — the saved providers to pick from, and the one
// form that adds or edits the picked one.

import { useQuery } from '@tanstack/react-query'
import { KeyRound, Plus } from 'lucide-react'
import { useState } from 'react'
import { type Provider, providersQuery } from '@/api/aylul-profiles'
import { ErrorLine } from '../error-line'
import { ProviderForm } from './provider-form'

export function ProviderSettings() {
  const providers = useQuery(providersQuery)
  // The picked provider's slug; null is the new-provider form.
  const [picked, setPicked] = useState<string | null>(null)
  const provider = providers.data?.find((p) => p.slug === picked) ?? null

  return (
    <div className="h-full overflow-y-auto">
      <div className="mx-auto max-w-2xl space-y-8 px-6 py-8">
        <header className="space-y-1">
          <h1 className="text-base font-medium">Providers</h1>
          <p className="text-xs text-faint-foreground">
            A provider is entered once, and every agent on it shares its key.
          </p>
        </header>
        {providers.error !== null && <ErrorLine error={providers.error} />}
        {providers.data !== undefined && providers.data.length > 0 && (
          <ul aria-label="Saved providers" className="divide-y divide-border rounded-md border border-border">
            {providers.data.map((p) => (
              <ProviderRow key={p.slug} provider={p} picked={p.slug === picked}
                onPick={() => setPicked(p.slug)} />
            ))}
            <li>
              <button type="button" onClick={() => setPicked(null)}
                aria-current={picked === null ? 'true' : undefined}
                className={`flex w-full items-center gap-2 px-3 py-2 text-left text-sm transition-colors hover:bg-secondary/60 ${
                  picked === null ? 'text-foreground' : 'text-muted-foreground'
                }`}>
                <Plus aria-hidden className="size-3.5" />
                New provider
              </button>
            </li>
          </ul>
        )}
        <section className="space-y-4">
          <h2 className="text-sm font-medium">
            {provider === null ? 'New provider' : <span dir="auto">{provider.name}</span>}
          </h2>
          {/* Keyed, so one provider's unsaved edits never show under another. */}
          <ProviderForm key={provider?.slug ?? 'new'} provider={provider}
            onSaved={(p) => setPicked(p?.slug ?? null)} />
        </section>
      </div>
    </div>
  )
}

function ProviderRow({ provider, picked, onPick }: {
  provider: Provider
  picked: boolean
  onPick: () => void
}) {
  return (
    <li>
      <button type="button" onClick={onPick} aria-current={picked ? 'true' : undefined}
        className={`flex w-full items-center gap-3 border-l-2 px-3 py-2 text-left transition-colors hover:bg-secondary/60 ${
          picked ? 'border-accent-line bg-secondary/60' : 'border-transparent'
        }`}>
        <span className="min-w-0 flex-1">
          <span dir="auto" className="block truncate text-sm">{provider.name}</span>
          <span className="block truncate font-mono text-[11px] text-faint-foreground">
            {provider.base_url}
          </span>
        </span>
        <span className="shrink-0 text-xs text-muted-foreground">
          {provider.models.length} {provider.models.length === 1 ? 'model' : 'models'}
        </span>
        <span title={provider.key_set ? 'Key saved' : 'No key yet'}
          className={`flex shrink-0 items-center gap-1 text-xs ${
            provider.key_set ? 'text-muted-foreground' : 'text-destructive-foreground'
          }`}>
          <KeyRound aria-hidden className="size-3.5" />
          {provider.key_set ? 'Key saved' : 'No key'}
        </span>
      </button>
    </li>
  )
}
