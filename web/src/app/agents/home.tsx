// One job: the main pane of the AYLUL workspace when no conversation is open —
// the next step, a provider and then an agent, until both exist.

import { useQuery } from '@tanstack/react-query'
import { Link } from '@tanstack/react-router'
import { ArrowUpRight, Bot, CheckCircle2, Code2, Cpu, Layers, Server, ShieldCheck, Sparkles, Wrench } from 'lucide-react'
import { agentsQuery, providersQuery } from '@/api/aylul-profiles'
import { buttonVariants } from '@/components/ui/button'
import { Panel } from '@/components/ui/panel'
import { Planned } from '@/components/ui/planned'
import { ErrorLine } from '../error-line'
import { useOpenSettings } from '@/app/navigation'
import { ColorDot } from './color-swatches'

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
  if (providers.data === undefined || agents.data === undefined) {
    return (
      <div className="flex flex-1 items-center justify-center p-8">
        <p className="text-sm text-faint-foreground">Looking for your agents…</p>
      </div>
    )
  }
  const list = agents.data.agents
  const providerList = providers.data

  return (
    <div className="flex flex-1 flex-col overflow-y-auto px-8 py-8">
      <div className="mx-auto flex w-full max-w-5xl flex-col gap-6">
        <Panel size="lg" className="space-y-6">
          <div className="flex flex-wrap items-center justify-between gap-4">
            <div className="flex items-start gap-4">
              <div className="flex size-12 shrink-0 items-center justify-center rounded-2xl border border-accent-line/40 bg-accent-line/15">
                <Code2 aria-hidden className="size-6 text-accent-line" />
              </div>
              <div className="space-y-1">
                <span className="inline-flex items-center gap-1 rounded-full border border-accent-line/35 bg-accent-line/15 px-2.5 py-0.5 text-[11px] font-semibold tracking-wider text-accent-line uppercase">
                  <Sparkles aria-hidden className="size-3" />
                  NYLRA · AYLUL Agent Mode
                </span>
                <h1 className="text-lg font-semibold tracking-tight text-foreground">Agent Workspace</h1>
                <p className="text-xs text-muted-foreground">Choose an agent, or start a conversation with one.</p>
              </div>
            </div>
            <div className="flex items-center gap-2 text-xs">
              <div className="rounded-xl border border-border bg-input-background px-3 py-2">
                <span className="text-muted-foreground">Providers: </span>
                <span className="font-mono font-semibold text-foreground">{providerList.length}</span>
              </div>
              <div className="rounded-xl border border-border bg-input-background px-3 py-2">
                <span className="text-muted-foreground">Agents: </span>
                <span className="font-mono font-semibold text-foreground">{list.length}</span>
              </div>
            </div>
          </div>

          <Planned title="Run stages" reason="Waits for the run stages of an agent turn.">
            <div className="grid grid-cols-2 gap-3 sm:grid-cols-4">
              {STAGES.map((stage) => (
                <div key={stage.title} className="flex items-center gap-3 rounded-xl border border-border bg-input-background px-3.5 py-2.5">
                  <div className="flex size-8 shrink-0 items-center justify-center rounded-lg border border-border">
                    <stage.icon aria-hidden className="size-4 text-accent-line" />
                  </div>
                  <div className="min-w-0">
                    <p className="truncate text-xs font-semibold text-foreground">{stage.title}</p>
                    <p className="truncate text-[11px] text-muted-foreground">{stage.subtitle}</p>
                  </div>
                </div>
              ))}
            </div>
          </Planned>
        </Panel>

        <section className="space-y-3">
          <h2 className="text-xs font-semibold tracking-wider text-muted-foreground uppercase">Active Agents ({list.length})</h2>
          <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
            {list.map((agent) => {
              const provider = providerList.find((p) => p.slug === agent.provider)
              return (
                <Link key={agent.slug} to="/aylul/agents/$slug/new" params={{ slug: agent.slug }} className="group block">
                  <Panel className="flex flex-col justify-between gap-4 p-5 transition-all group-hover:border-accent-line/45">
                    <div className="flex items-start justify-between gap-3">
                      <div className="flex min-w-0 items-center gap-3">
                        <div className="flex size-10 shrink-0 items-center justify-center rounded-xl border border-accent-line/35 bg-accent-line/15">
                          <ColorDot id={agent.character_color} />
                        </div>
                        <div className="min-w-0">
                          <span dir="auto" className="block truncate text-sm font-semibold text-foreground">{agent.name}</span>
                          {agent.description && <p dir="auto" className="truncate text-xs text-muted-foreground">{agent.description}</p>}
                        </div>
                      </div>
                      <span className="inline-flex items-center gap-1 rounded-lg border border-accent-line/30 bg-accent-line/15 px-2.5 py-1 text-xs font-medium text-foreground transition-colors group-hover:bg-accent-line/30">
                        <span>Launch</span>
                        <ArrowUpRight aria-hidden className="size-3.5" />
                      </span>
                    </div>
                    <div className="flex flex-wrap items-center gap-2 border-t border-border pt-3 text-[11px]">
                      <span className="rounded-md border border-border bg-input-background px-2 py-0.5 text-muted-foreground">
                        Provider: <strong className="text-foreground">{provider?.name ?? agent.provider}</strong>
                      </span>
                      <span className="rounded-md border border-border bg-input-background px-2 py-0.5 font-mono text-accent-line">
                        {agent.default_model}
                      </span>
                    </div>
                  </Panel>
                </Link>
              )
            })}
          </div>
        </section>

        <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
          <PlannedList icon={Wrench} title="Agent Runtime Capabilities" reason="Waits for the runtime capabilities of an agent."
            items={['File System & Diffs', 'Terminal Execution', 'Multi-Model Handoff', 'Subagent Delegation']} />
          <PlannedList icon={Cpu} title="Context & Memory Layers" reason="Waits for the context use per operation."
            items={['Working Task State', 'Session Conversation', 'Project Architecture', 'Scoped Environment']} />
        </div>
      </div>
    </div>
  )
}

const STAGES = [
  { title: 'Plan', subtitle: 'Analyze & create steps', icon: Layers },
  { title: 'Execute', subtitle: 'Run tasks & tools', icon: Code2 },
  { title: 'Verify', subtitle: 'Test & validate diffs', icon: ShieldCheck },
  { title: 'Deliver', subtitle: 'Results & summary', icon: CheckCircle2 },
]

function PlannedList({ icon: Icon, title, reason, items }: {
  icon: typeof Bot
  title: string
  reason: string
  items: string[]
}) {
  return (
    <Panel>
      <div className="mb-3 flex items-center gap-2">
        <Icon aria-hidden className="size-4 text-accent-line" />
        <h3 className="text-xs font-semibold tracking-wider text-foreground uppercase">{title}</h3>
      </div>
      <Planned title={`${title} list`} reason={reason}>
        <div className="grid grid-cols-2 gap-2 text-xs">
          {items.map((item) => (
            <div key={item} className="truncate rounded-xl border border-border bg-input-background px-3 py-2 text-muted-foreground">{item}</div>
          ))}
        </div>
      </Planned>
    </Panel>
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
      <Panel size="lg" className="flex max-w-md flex-col items-center gap-4">
        <div className="flex size-14 items-center justify-center rounded-2xl border border-accent-line/40 bg-accent-line/15">
          <Icon aria-hidden className="size-7 text-accent-line" />
        </div>
        <div className="space-y-1.5">
          <h1 className="text-base font-semibold text-foreground">{title}</h1>
          <p className="max-w-sm text-sm text-muted-foreground">{detail}</p>
        </div>
        <button type="button" className={buttonVariants({ className: 'pill-active' })} onClick={() => openSettings(to, to === 'agents' ? 'new' : undefined)}>{action}</button>
      </Panel>
    </div>
  )
}
