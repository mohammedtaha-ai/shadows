// One job: the AYLUL workspace's agents as a tree — each folds open to its
// conversations and settings, and opens a new conversation from its row.

import { useQuery } from '@tanstack/react-query'
import { Link, useSearch, useParams } from '@tanstack/react-router'
import { Bot, ChevronRight, Plus, Settings } from 'lucide-react'
import { useState } from 'react'
import { type Agent, agentsQuery, providersQuery } from '@/api/aylul-profiles'
import { threadQuery } from '@/api/threads'
import { ErrorLine } from '../error-line'
import { ThreadList } from './thread-list'
import { colorOf } from '@/app/agents'
import { useOpenSettings } from '@/app/navigation'

export function AgentList({ filter = '' }: { filter?: string }) {
  const agents = useQuery(agentsQuery)
  const providers = useQuery(providersQuery)
  const { threadId } = useParams({ strict: false })
  // The open conversation's agent starts unfolded, so the page shows where it is.
  const thread = useQuery({ ...threadQuery(threadId ?? ''), enabled: threadId !== undefined })
  const currentAgent = thread.data?.profile ?? undefined
  const [toggled, setToggled] = useState<Record<string, boolean>>({})
  const allAgents = agents.data?.agents
  const q = filter.trim().toLowerCase()
  const list = allAgents === undefined || q === ''
    ? allAgents
    : allAgents.filter((a) => a.name.toLowerCase().includes(q) || (a.description?.toLowerCase().includes(q) ?? false))

  return (
    <div className="flex min-h-full flex-col gap-2">
      {agents.error !== null && <ErrorLine error={agents.error} />}
      {providers.error !== null && <ErrorLine error={providers.error} />}
      {providers.data?.length === 0 ? (
        <EmptyHint to="providers" action="Add a provider first"
          detail="A provider is the URL, key and models your agents run on." />
      ) : allAgents?.length === 0 ? (
        <EmptyHint to="agents" action="Add an agent to start"
          detail="Each agent picks a provider and keeps its own conversations." />
      ) : list?.length === 0 ? (
        <div className="rounded-xl border border-border/60 bg-card/40 px-3 py-2.5">
          <p className="text-xs text-faint-foreground">No matching agents.</p>
        </div>
      ) : null}
      {list !== undefined && list.length > 0 && agents.data !== undefined && (
        <ul className="space-y-1">
          {list.map((agent) => {
            const open = toggled[agent.slug] ?? (agent.slug === currentAgent || q !== '')
            return (
              <AgentRow
                key={agent.slug}
                agent={agent}
                projectId={agents.data.project_id}
                open={open}
                current={agent.slug === currentAgent}
                selected={threadId}
                onToggle={() => setToggled((t) => ({ ...t, [agent.slug]: !open }))}
              />
            )
          })}
        </ul>
      )}
    </div>
  )
}

function EmptyHint({ to, action, detail }: {
  to: 'providers' | 'agents'
  action: string
  detail: string
}) {
  const openSettings = useOpenSettings()
  return (
    <div className="space-y-1.5 rounded-xl border border-accent-line/25 bg-card/80 px-3 py-2.5 shadow-xs">
      <button type="button" className="text-sm font-medium text-accent-line underline-offset-4 hover:underline" onClick={() => openSettings(to, to === 'agents' ? 'new' : undefined)}>{action}</button>
      <p className="text-xs leading-relaxed text-muted-foreground">{detail}</p>
    </div>
  )
}

/** Laid out like a project row: the row folds, the plus opens a draft. */
function AgentRow({ agent, projectId, open, current, selected, onToggle }: {
  agent: Agent
  /** The hidden assistant project every AYLUL conversation lives in. */
  projectId: string
  open: boolean
  current: boolean
  selected?: string
  onToggle: () => void
}) {
  const sectionId = `agent-${agent.slug}-section`
  return (
    <li>
      <div
        className={`group relative flex items-start rounded-xl border transition-all ${
          current
            ? 'border-accent-line/45 bg-gradient-to-r from-accent/25 via-accent-softer/90 to-transparent'
            : open
              ? 'border-border/70 bg-sidebar-accent/70'
              : 'border-transparent hover:border-border/50 hover:bg-sidebar-accent/50'
        }`}
      >
        <button
          type="button"
          onClick={onToggle}
          aria-expanded={open}
          aria-controls={sectionId}
          className={`flex min-w-0 flex-1 items-center gap-2 rounded-xl py-2 ps-2 pe-8 text-left text-sm ${
            open || current ? 'font-medium text-sidebar-foreground' : 'text-muted-foreground'
          }`}
        >
          <ChevronRight
            aria-hidden
            className={`size-4 shrink-0 text-faint-foreground transition-transform duration-150 ${open ? 'rotate-90 text-accent-line' : ''}`}
          />
          <Bot aria-hidden className="size-4 shrink-0" data-color={colorOf(agent.character_color).id} style={{ color: colorOf(agent.character_color).swatch }} />
          <span className="min-w-0 flex-1 text-start">
            <span dir="auto" className="block truncate">{agent.name}</span>
            {agent.description && <span dir="auto" className="block truncate text-xs font-normal text-faint-foreground">{agent.description}</span>}
          </span>
        </button>
        <Link
          to="/aylul/agents/$slug/new"
          params={{ slug: agent.slug }}
          aria-label={`New conversation with ${agent.name}`}
          title="New conversation"
          className="absolute top-1.5 right-1.5 rounded-lg p-1 text-faint-foreground opacity-0 transition-all group-hover:opacity-100 hover:bg-accent/30 hover:text-sidebar-foreground focus-visible:opacity-100 pointer-coarse:opacity-100"
        >
          <Plus aria-hidden className="size-3.5" />
        </Link>
      </div>
      <div id={sectionId}>
        {open && (
          <>
            <ThreadList projectId={projectId} profile={agent.slug} selected={selected} />
            <AgentSettingsLink slug={agent.slug} />
          </>
        )}
      </div>
    </li>
  )
}

function AgentSettingsLink({ slug }: { slug: string }) {
  const search = useSearch({ from: '__root__' })
  const isSelected = search.settings === 'agents' && search.agent === slug
  const openSettings = useOpenSettings()
  return (
    <div className="mb-1.5 ml-4 border-l border-sidebar-border/80 pl-2">
      <button
        type="button"
        onClick={() => openSettings('agents', slug)}
        aria-current={isSelected ? 'page' : undefined}
        className={`flex w-full items-center gap-2 rounded-r-lg border-l-2 px-2.5 py-1.5 text-xs transition-all ${
          isSelected
            ? 'border-accent-line bg-secondary/90 font-medium text-secondary-foreground'
            : 'border-transparent text-muted-foreground hover:bg-sidebar-accent/70 hover:text-sidebar-foreground'
        }`}
      >
        <Settings className="size-3.5 shrink-0" aria-hidden />
        Agent settings
      </button>
    </div>
  )
}
