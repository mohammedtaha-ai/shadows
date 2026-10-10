// One job: the open conversation's pane — header, messages, composer, and
// the plan or subagent shown beside them.

import { useQuery } from '@tanstack/react-query'
import { getRouteApi, useNavigate } from '@tanstack/react-router'
import { useState } from 'react'
import type { Plan, PlanTask, PlanningThread, Project, ThreadEntry } from '@/api/client'
import { harnessesQuery, projectsQuery, threadQuery, threadsQuery } from '@/api/queries'
import { type PlanShowFrame, type SubagentCard, toLimits } from '@/stream/frames'
import { ErrorLine } from '../error-line'
import { HeldThreadStream } from '../workflows/plan-frames'
import { useCarriedSend } from './carried-send'
import { CliPicker } from './cli-picker'
import { Composer } from './composer'
import { ContextRing } from './context-ring'
import { DeletedConversation } from './deleted-conversation'
import type { PointedTask } from './focus-chip'
import { Messages } from './messages'
import { PlanSidePanel, type SideShown } from './plan-side-panel'
import { StatusBadge } from './status-badge'
import { StageChip } from './stage-chip'
import { StreamBanner } from './stream-banner'
import { SubagentPanel } from './subagent-panel'
import { subagentOf } from './tool-text'
import { useConversation } from './use-conversation'
import { useSession } from './use-session'
import { WaitingMessages } from './waiting-messages'

const route = getRouteApi('/projects/$projectId/threads/$threadId')

/** A thread made before threads named their CLI runs on Claude Code (spec §12.6). */
export const DEFAULT_HARNESS = 'claude-code'

/** The route's component. Keyed by thread, so switching conversations starts
 * the next one's state from nothing rather than from the last one's. */
export function ConversationRoute() {
  const { projectId, threadId } = route.useParams()
  const project = useQuery(projectsQuery).data?.find((p) => p.id === projectId)
  const list = useQuery(threadsQuery(projectId))
  const listed = list.data?.find((t) => t.id === threadId)
  const historical = useQuery({
    ...threadQuery(threadId),
    enabled: list.isSuccess && listed === undefined,
    retry: false,
  })
  const thread = listed ?? historical.data
  // A deleted thread must never mount the component that opens its session.
  if (thread === undefined) {
    const error = list.error ?? historical.error
    return <div className="p-6">{error !== null ? <ErrorLine error={error} /> : 'Loading conversation…'}</div>
  }
  if (thread.removed_at != null) {
    return <DeletedConversation key={threadId} thread={thread} projectId={projectId} />
  }
  return (
    <Conversation
      key={threadId}
      projectId={projectId}
      threadId={threadId}
      thread={thread}
      project={project}
    />
  )
}

function Conversation({
  projectId,
  threadId,
  thread,
  project,
}: {
  projectId: string
  threadId: string
  thread: PlanningThread | undefined
  project: Project | undefined
}) {
  const navigate = useNavigate()
  const [side, setSide] = useState<SideShown | null>(null)
  const [pointed, setPointed] = useState<PointedTask | null>(null)
  // Only a live `plan-show` for this tab arrives here (see `useConversation`).
  // The subagent whose panel is open (§22.4), by card id: one panel at a time.
  const [agent, setAgent] = useState<string | null>(null)
  const showHere = (show: PlanShowFrame) => {
    if (show.place === 'side') {
      setAgent(null)
      setSide({ workflowId: show.workflowId, taskNumber: show.taskNumber })
    } else if (show.place === 'page') {
      void navigate({
        to: '/projects/$projectId/workflows/$workflowId',
        params: { projectId, workflowId: show.workflowId },
      })
    }
  }
  // In the version and at the revision the person was looking at.
  const onPointAt = (plan: Plan, task: PlanTask) =>
    setPointed({ workflowId: plan.id, revision: plan.revision, task })
  const c = useConversation(threadId, showHere)
  // Opened as soon as the conversation shows, so the menus are ready before
  // the first message (spec §12.2).
  const session = useSession(threadId)
  const carried = useCarriedSend(threadId)
  const harness = thread?.harness ?? DEFAULT_HARNESS
  const info = useQuery(harnessesQuery).data?.find((h) => h.kind === harness)
  // The kind itself until the list has answered.
  const label = info?.label ?? harness
  // Limits are account-wide: reported live on this stream, else as the
  // daemon last kept them for the harness.
  const limits = c.limits ?? (info?.limits == null ? null : toLimits(info.limits))
  const live = Object.values(c.stream.subagents)
  const agentCard = agent === null ? null : cardOf(agent, c.entries.data ?? [], live)
  const openAgent = (id: string) => {
    setSide(null)
    setAgent(id)
  }

  return (
    // This conversation's stream carries its plans' changes: its cards and
    // panel open no stream of their own.
    <HeldThreadStream value={threadId}>
      <div className="flex h-full min-h-0">
        <div className="flex min-w-0 flex-1 flex-col">
          <header className="flex items-center justify-between gap-4 border-b border-border bg-background/80 px-6 py-3 backdrop-blur-md">
            <div className="min-w-0">
              <h1 dir="auto" className="truncate text-start text-sm font-medium">
                {thread?.title ?? 'Conversation'}
              </h1>
              <StageChip projectId={projectId} />
              <p className="truncate text-xs text-faint-foreground">
                <bdi>{project?.name ?? '…'}</bdi> · Planner
              </p>
            </div>
            <div className="flex items-center gap-3">
              <CliPicker
                projectId={projectId}
                threadId={threadId}
                harness={harness}
                label={label}
                // A fork is locked from birth (spec §12.9): its session is a fork
                // of its source's, which no other harness could continue.
                locked={c.latest !== null || thread?.forked_from_thread != null}
              />
              <StatusBadge running={c.running} latest={c.latest} />
            </div>
          </header>
          <StreamBanner
            connection={c.stream.connection}
            problem={c.stream.problem}
            retry={c.stream.retry}
          />
          {c.entries.isError && (
            <div className="px-6 pt-3">
              <ErrorLine error={c.entries.error} />
            </div>
          )}
          <Messages
            entries={c.entries.data}
            loading={!c.stream.caughtUp}
            reply={c.reply}
            running={c.running !== null}
            thinking={c.thinking}
            label={c.label}
            operations={c.operations}
            models={session.state === 'ready' ? session.choices.models : []}
            harness={harness}
            forkFrom={c.known && c.running === null ? { projectId, threadId } : null}
            projectId={projectId}
            onPointAt={onPointAt}
            subagents={live}
            onOpenSubagent={openAgent}
          />
          <WaitingMessages threadId={threadId} running={c.running !== null} />
          <Composer
            to={{ threadId }}
            carried={carried}
            harness={harness}
            harnessLabel={label}
            session={session}
            running={c.running}
            directory={project?.directory}
            known={c.known}
            commands={c.commands}
            ring={<ContextRing threadId={threadId} usage={c.context} limits={limits} />}
            onStarted={c.started}
            pointed={pointed}
            // Only the chip that was sent or dismissed: a task clicked meanwhile stays.
            onPointed={(done) => setPointed((now) => (now === done ? null : now))}
          />
        </div>
        {side !== null && (
          <PlanSidePanel
            // Another version is another graph, fitted afresh.
            key={side.workflowId}
            shown={side}
            projectId={projectId}
            onClose={() => setSide(null)}
            onPointAt={onPointAt}
          />
        )}
        {agentCard !== null && (
          <SubagentPanel key={agentCard.id} card={agentCard} onClose={() => setAgent(null)} />
        )}
      </div>
    </HeldThreadStream>
  )
}

/** Subagent `id`'s card: its entry once written, else as the stream last
 * sent it. */
function cardOf(
  id: string,
  entries: readonly ThreadEntry[],
  live: readonly SubagentCard[],
): SubagentCard | null {
  for (const entry of entries) {
    const card = subagentOf(entry)
    if (card?.id === id) return card
  }
  return live.find((card) => card.id === id) ?? null
}
