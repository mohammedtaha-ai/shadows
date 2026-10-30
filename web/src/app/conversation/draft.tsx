// Defer thread creation until Send so an abandoned draft leaves nothing on the daemon.

import { useQuery } from '@tanstack/react-query'
import { getRouteApi, useNavigate } from '@tanstack/react-router'
import { MessageSquarePlus, X } from 'lucide-react'
import { useMemo, useState } from 'react'
import { harnessesQuery, policyOf } from '@/api/harnesses'
import { planVersionsQuery, plansQuery } from '@/api/workflows'
import { projectsQuery } from '@/api/projects'
import { agentsQuery, providersQuery, type Provider } from '@/api/aylul-profiles'
import { ErrorLine } from '../error-line'
import { draftSession } from './draft-session'
import { useFirstSend } from './first-send'
import { AYLUL_HARNESS, CliMenu } from './cli-picker'
import { Composer } from './composer'
import { DEFAULT_HARNESS } from './conversation'
import { StageChip } from './stage-chip'
import type { SessionView } from './use-session'

const route = getRouteApi('/projects/$projectId/new')

/** Keyed by project, so a draft never carries its text to another project. */
export function DraftRoute() {
  const { projectId } = route.useParams()
  const { plan } = route.useSearch()
  return <Draft key={projectId} projectId={projectId} planId={plan} />
}

export function AgentDraftRoute() {
  const { slug } = getRouteApi('/aylul/agents/$slug/new').useParams()
  const workspace = useQuery(agentsQuery)
  const providers = useQuery(providersQuery)
  const error = workspace.error ?? providers.error
  if (error) return <ErrorLine error={error} />
  if (workspace.data === undefined || providers.data === undefined) return <p className="p-8">Loading agent…</p>
  const agent = workspace.data.agents.find(a => a.slug === slug)
  const provider = providers.data.find(p => p.slug === agent?.provider)
  if (agent === undefined || provider === undefined) return <p className="p-8">Agent not found.</p>
  return <Draft key={slug} projectId={workspace.data.project_id} profile={slug}
    provider={provider} defaultModel={agent.default_model} agentName={agent.name} />
}

function Draft({ projectId, planId, profile, provider, defaultModel, agentName }: {
  projectId: string; planId?: string; profile?: string; provider?: Provider; defaultModel?: string; agentName?: string
}) {
  const navigate = useNavigate()
  const project = useQuery(projectsQuery).data?.find((p) => p.id === projectId)
  const [picked, setHarness] = useState(DEFAULT_HARNESS)
  // AYLUL's project runs on `aylul` alone.
  const harness = profile !== undefined || project?.assistant === true ? AYLUL_HARNESS : picked
  const harnesses = useQuery(harnessesQuery)
  const label = harnesses.data?.find((h) => h.kind === harness)?.label ?? harness
  const allowed = project?.allowed_modes[harness]
  const policy = harnesses.data === undefined ? undefined : policyOf(harnesses.data, harness)
  const ready = useMemo(() => draftSession(policy, allowed, provider, defaultModel), [policy, allowed, provider, defaultModel])
  // Without the harness list no mode is offered; the composer shows why. A failed
  // refetch over a list already read keeps that list's policy.
  const session: SessionView =
    harnesses.data === undefined && harnesses.isError && !harnesses.isFetching
      ? { state: 'failed', error: harnesses.error, retry: () => void harnesses.refetch() }
      : ready
  const draft = useFirstSend(projectId, harness, planId, profile)

  const planVersions = useQuery({
    ...planVersionsQuery(planId ?? ''),
    enabled: planId !== undefined,
  }).data
  const plansList = useQuery(plansQuery(projectId, true)).data
  const planTitle =
    plansList?.find((p) => p.plan_id === planId)?.title ??
    planVersions?.versions.at(-1)?.title ??
    planId

  const clearPlan = () => {
    void navigate({
      to: '/projects/$projectId/new',
      params: { projectId },
      search: {},
      replace: true,
    })
  }

  return (
    <div className="flex h-full min-h-0 flex-col">
      <header className="flex items-center justify-between gap-4 border-b border-border bg-background/80 px-6 py-3 backdrop-blur-md">
        <div className="min-w-0">
          <h1 className="truncate text-sm font-medium">New conversation</h1>
          {harness !== AYLUL_HARNESS && <StageChip projectId={projectId} />}
          <p className="truncate text-xs text-faint-foreground">
            <bdi>{agentName ?? project?.name ?? '…'}</bdi>
            {project?.assistant === true ? null : ' · Planner'}
          </p>
        </div>
        <CliMenu
          harness={harness}
          label={label}
          onPick={setHarness}
          assistant={project?.assistant === true}
        />
      </header>
      <div className="flex flex-1 flex-col items-center justify-center gap-3 p-8 text-center">
        <div className="relative flex items-center justify-center">
          <div className="absolute size-14 rounded-full bg-accent-line/10 blur-xl" aria-hidden />
          <div className="relative flex size-12 items-center justify-center rounded-full border border-border/60 bg-card/60 shadow-xs">
            <MessageSquarePlus aria-hidden className="size-6 text-accent-line" />
          </div>
        </div>
        <p className="text-sm text-muted-foreground">
          The conversation starts with your first message.
        </p>
      </div>
      {planId && (
        <div className="px-6 pb-2">
          <div className="mx-auto max-w-3xl">
            <div className="inline-flex max-w-full items-center gap-1.5 rounded-full border border-accent-line/40 bg-accent-softer/90 py-1 pr-1.5 pl-3 text-xs text-secondary-foreground shadow-xs transition-all">
              <span dir="auto" className="truncate font-medium">
                Continuing: {planTitle}
              </span>
              <button
                type="button"
                onClick={clearPlan}
                aria-label="×"
                className="rounded-full p-0.5 text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
              >
                <X className="size-3" aria-hidden />
                <span className="sr-only">×</span>
              </button>
            </div>
          </div>
        </div>
      )}
      <Composer
        to={{ draft, planId, selectableModels: profile !== undefined }}
        harness={harness}
        harnessLabel={label}
        session={session}
        running={null}
        directory={project?.directory}
        known
        onStarted={() => {}}
        pointed={null}
        onPointed={() => {}}
      />
    </div>
  )
}
