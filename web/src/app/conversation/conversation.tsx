// One job: the open conversation's pane — header, messages, composer, and
// the plan the Planner put beside them.

import { useQuery } from '@tanstack/react-query'
import { getRouteApi, useNavigate } from '@tanstack/react-router'
import { useState } from 'react'
import type { Plan, PlanTask, PlanningThread, Project } from '@/api/client'
import { harnessesQuery, projectsQuery, threadsQuery } from '@/api/queries'
import { type PlanShowFrame, toLimits } from '@/stream/frames'
import { ErrorLine } from '../error-line'
import { HeldThreadStream } from '../workflows/plan-frames'
import { CliPicker } from './cli-picker'
import { Composer } from './composer'
import { ContextRing } from './context-ring'
import type { PointedTask } from './focus-chip'
import { Messages } from './messages'
import { PlanSidePanel, type SideShown } from './plan-side-panel'
import { StatusBadge } from './status-badge'
import { StreamBanner } from './stream-banner'
import { useConversation } from './use-conversation'
import { useSession } from './use-session'

const route = getRouteApi('/projects/$projectId/threads/$threadId')

/** A thread made before threads named their CLI runs on Claude Code (spec §12.6). */
const DEFAULT_HARNESS = 'claude-code'

/** The route's component. Keyed by thread, so switching conversations starts
 * the next one's state from nothing rather than from the last one's. */
export function ConversationRoute() {
  const { projectId, threadId } = route.useParams()
  const project = useQuery(projectsQuery).data?.find((p) => p.id === projectId)
  const thread = useQuery(threadsQuery(projectId)).data?.find((t) => t.id === threadId)
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
  const showHere = (show: PlanShowFrame) => {
    if (show.place === 'side') {
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
  const harness = thread?.harness ?? DEFAULT_HARNESS
  const info = useQuery(harnessesQuery).data?.find((h) => h.kind === harness)
  // The kind itself until the list has answered.
  const label = info?.label ?? harness
  // Limits are account-wide: reported live on this stream, else as the
  // daemon last kept them for the harness.
  const limits = c.limits ?? (info?.limits == null ? null : toLimits(info.limits))

  return (
    // This conversation's stream carries its plans' changes: its cards and
    // panel open no stream of their own.
    <HeldThreadStream value={threadId}>
      <div className="flex h-full min-h-0">
        <div className="flex min-w-0 flex-1 flex-col">
          <header className="flex items-center justify-between gap-4 border-b border-border px-6 py-3">
            <div className="min-w-0">
              <h1 className="truncate text-sm font-medium">{thread?.title ?? 'Conversation'}</h1>
              <p className="truncate text-xs text-faint-foreground">
                {project?.name ?? '…'} · Planner
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
          />
          <Composer
            threadId={threadId}
            harness={harness}
            harnessLabel={label}
            session={session}
            running={c.running}
            directory={project?.directory}
            known={c.known}
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
      </div>
    </HeldThreadStream>
  )
}
