import { useQuery } from '@tanstack/react-query'
import { useSearch } from '@tanstack/react-router'
import { agentsQuery, providersQuery, providerPresetsQuery } from '@/api/aylul-profiles'
import { Button } from '@/components/ui/button'
import { providerTitle } from '@/app/agents'
import { ColorDot } from '@/app/agents'
import { isDesktop } from '@/desktop'
import { useCompanionBinding } from '@/desktop'
import { AgentForm, DeleteAgent } from '@/app/agents'
import { ErrorLine } from '../../error-line'
import { useOpenSettings } from '@/app/navigation'

export function AgentsSection() {
  const binding = useCompanionBinding()
  const agents = useQuery(agentsQuery)
  const providers = useQuery(providersQuery)
  const presets = useQuery(providerPresetsQuery)
  const { agent: slug } = useSearch({ from: '__root__' })
  const open = useOpenSettings()
  const onDone = () => open('agents')
  const live = agents.data?.agents.filter(agent => !agent.deleted)
  const selected = live?.find(agent => agent.slug === slug)
  if (agents.error || providers.error) return <ErrorLine error={agents.error ?? providers.error!} />
  if (slug === 'new' || selected) return <div className="space-y-4">
    <Button type="button" variant="ghost" aria-label="Back" onClick={onDone}>←</Button>
    <AgentForm key={slug} slug={slug === 'new' ? null : slug!} onDone={onDone} />
  </div>
  return <div className="space-y-4">
    {slug && agents.data && <p role="status" className="text-sm text-muted-foreground">This agent no longer exists.</p>}
    <Button type="button" onClick={() => open('agents', 'new')}>New agent</Button>
    {live && <ul aria-label="Saved agents" className="divide-y divide-border">
      {live.map(agent => <li key={agent.slug} className="flex flex-wrap items-start gap-3 py-3">
        <span className="flex flex-1 items-center gap-2"><ColorDot id={agent.character_color} />{agent.name}{isDesktop() && binding?.agent === agent.slug && <span className="rounded bg-secondary px-1 text-xs">Companion</span>}</span>
        <span className="text-sm text-muted-foreground">{(() => { const provider = providers.data?.find(provider => provider.slug === agent.provider); return provider ? providerTitle(provider, presets.data ?? []) : agent.provider })()}</span>
        <span className="text-xs text-muted-foreground">{agent.default_model}</span>
        <Button type="button" variant="outline" size="sm" onClick={() => open('agents', agent.slug)}>Edit</Button>
        <DeleteAgent compact slug={agent.slug} name={agent.name} onDone={onDone} />
      </li>)}
    </ul>}
  </div>
}
