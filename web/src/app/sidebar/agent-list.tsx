// One job: the AYLUL workspace's agents as a tree — each folds open to its
// conversations and settings, and opens a new conversation from its row.

import { useQuery } from '@tanstack/react-query'
import { Link, useSearch, useParams } from '@tanstack/react-router'
import { Bot, ChevronRight, Plus, Settings } from 'lucide-react'
import { useState } from 'react'
import { type Agent, agentsQuery, providersQuery } from '@/api/aylul-profiles'
import { threadQuery } from '@/api/queries'
import { ErrorLine } from '../error-line'
import { ThreadList } from './thread-list'
import { useOpenSettings } from '../settings/use-settings'

export function AgentList() {
  const agents = useQuery(agentsQuery)
  const providers = useQuery(providersQuery)
  const { threadId } = useParams({ strict: false })
  // The open conversation's agent starts unfolded, so the page shows where it is.
  const thread = useQuery({ ...threadQuery(threadId ?? ''), enabled: threadId !== undefined })
  const currentAgent = thread.data?.profile ?? undefined
  const [toggled, setToggled] = useState<Record<string, boolean>>({})
  const list = agents.data?.agents

  return (
    <div className="flex min-h-full flex-col gap-2">
      {agents.error !== null && <ErrorLine error={agents.error} />}
      {providers.error !== null && <ErrorLine error={providers.error} />}
      {providers.data?.length === 0 ? (
        <EmptyHint to="providers" action="Add a provider first"
          detail="A provider is the URL, key and models your agents run on." />
      ) : list?.length === 0 ? (
        <EmptyHint to="agents" action="Add an agent to start"
          detail="Each agent picks a provider and keeps its own conversations." />
      ) : null}
      {list !== undefined && list.length > 0 && agents.data !== undefined && (
        <ul className="space-y-0.5">
          {list.map((agent) => {
            const open = toggled[agent.slug] ?? agent.slug === currentAgent
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
    <div className="space-y-0.5 px-2 py-1">
      <button type="button" className="text-sm text-sidebar-foreground underline-offset-4 hover:underline" onClick={() => openSettings(to, to === 'agents' ? 'new' : undefined)}>{action}</button>
      <p className="text-xs text-faint-foreground">{detail}</p>
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
      <div className="group relative flex items-start rounded-md transition-colors hover:bg-sidebar-accent">
        <button
          type="button"
          onClick={onToggle}
          aria-expanded={open}
          aria-controls={sectionId}
          className={`flex min-w-0 flex-1 items-center gap-1.5 rounded-md py-1.5 ps-1 pe-8 text-left text-sm ${
            open || current ? 'text-sidebar-foreground' : 'text-muted-foreground'
          }`}
        >
          <ChevronRight
            aria-hidden
            className={`size-4 shrink-0 text-faint-foreground transition-transform ${open ? 'rotate-90' : ''}`}
          />
          <Bot aria-hidden className={`size-4 shrink-0 ${current ? 'text-accent-line' : ''}`} />
          <span dir="auto" className="min-w-0 flex-1 truncate text-start">{agent.name}</span>
        </button>
        <Link
          to="/aylul/agents/$slug/new"
          params={{ slug: agent.slug }}
          aria-label={`New conversation with ${agent.name}`}
          title="New conversation"
          className="absolute top-1 right-1 rounded-md p-1 text-faint-foreground opacity-0 transition-opacity group-hover:opacity-100 hover:bg-secondary hover:text-sidebar-foreground focus-visible:opacity-100 pointer-coarse:opacity-100"
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
    <div className="mb-1.5 ml-4 border-l border-sidebar-border pl-2">
      <button
        type="button"
        onClick={() => openSettings('agents', slug)}
        aria-current={isSelected ? 'page' : undefined}
        className={`flex items-center gap-2 rounded-r-md border-l-2 px-2 py-1 text-sm transition-colors ${
          isSelected
            ? 'border-accent-line bg-secondary text-secondary-foreground'
            : 'border-transparent text-muted-foreground hover:bg-sidebar-accent hover:text-sidebar-foreground'
        }`}
      >
        <Settings className="size-3.5 shrink-0" aria-hidden />
        Agent settings
      </button>
    </div>
  )
}
