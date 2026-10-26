// One job: the main pane of the AYLUL workspace when no conversation is open —
// the next step, a provider and then an agent, until both exist.

import { useQuery } from '@tanstack/react-query'
import { Bot, Server } from 'lucide-react'
import { agentsQuery, providersQuery } from '@/api/aylul-profiles'
import { buttonVariants } from '@/components/ui/button'
import { ErrorLine } from '../error-line'
import { useOpenSettings } from '@/app/navigation'

export function AylulHome() {
  const providers = useQuery(providersQuery)
  const agents = useQuery(agentsQuery)
  const error = providers.error ?? agents.error

  if (error !== null) {
    return <div className="flex flex-1 items-center justify-center p-8"><ErrorLine error={error} /></div>
  }
  if (providers.data?.length === 0) {
    return <Step icon={Server} title="Add a provider first" to="providers" action="Add a provider"
      detail="A provider is an Anthropic-compatible URL, its key and its models. Enter it once; every agent on it shares the key." />
  }
  if (agents.data?.agents.length === 0) {
    return <Step icon={Bot} title="Add an agent to start" to="agents" action="New agent"
      detail="An agent runs on one provider and keeps its own conversations, model and variables." />
  }
  return (
    <div className="flex flex-1 items-center justify-center p-8">
      <p className="text-sm text-faint-foreground">
        {providers.data === undefined || agents.data === undefined
          ? 'Looking for your agents…'
          : 'Choose an agent, or start a conversation with one.'}
      </p>
    </div>
  )
}

function Step({ icon: Icon, title, detail, to, action }: {
  icon: typeof Bot
  title: string
  detail: string
  to: 'providers' | 'agents'
  action: string
}) {
  const openSettings = useOpenSettings()
  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-4 p-8 text-center">
      <Icon aria-hidden className="size-8 text-accent-line" />
      <div className="space-y-1">
        <h1 className="text-base font-medium">{title}</h1>
        <p className="max-w-sm text-sm text-muted-foreground">{detail}</p>
      </div>
      <button type="button" className={buttonVariants()} onClick={() => openSettings(to, to === 'agents' ? 'new' : undefined)}>{action}</button>
    </div>
  )
}
