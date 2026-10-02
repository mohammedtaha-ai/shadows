// One job: a new conversation before its first message (spec §13.11, §16.8) — the
// empty conversation with its composer, where nothing exists on the daemon
// until Send makes the thread and starts its first turn.

import { useQuery, useQueryClient } from '@tanstack/react-query'
import { getRouteApi, useNavigate } from '@tanstack/react-router'
import { MessageSquarePlus, X } from 'lucide-react'
import { useEffect, useMemo, useRef, useState } from 'react'
import {
  type Choice,
  type SessionChoices,
  type TurnSettings,
  createThread,
  openSession,
  startTurn,
} from '@/api/client'
import { type Attempt, attemptFor } from '@/api/command-id'
import { harnessesQuery, planVersionsQuery, plansQuery, projectsQuery, threadsQuery } from '@/api/queries'
import { tabId } from '@/stream/tab-id'
import { policyOf } from '../mode-policy'
import { carrySend } from './carried-send'
import { CliMenu } from './cli-picker'
import { Composer } from './composer'
import { DEFAULT_HARNESS } from './conversation'
import { type SessionView, sessionKey } from './use-session'

const route = getRouteApi('/projects/$projectId/new')

/** What a draft's model menu reads: no model is chosen before a session
 * exists, and Send takes the one the session opens at. */
const UNOPENED: Choice = {
  id: 'unopened',
  label: 'Set on send',
  description: null,
  enabled: true,
  reason: null,
}

/** Required by the API; the first message retitles the thread at once, and
 * the harness after it (spec §4). */
const DRAFT_TITLE = 'New conversation'

/** Keyed by project, so a draft never carries its text to another project. */
export function DraftRoute() {
  const { projectId } = route.useParams()
  return <Draft key={projectId} projectId={projectId} />
}

function Draft({ projectId }: { projectId: string }) {
  const navigate = useNavigate()
  const { plan: planId } = route.useSearch()
  const project = useQuery(projectsQuery).data?.find((p) => p.id === projectId)
  const [harness, setHarness] = useState(DEFAULT_HARNESS)
  const label = useQuery(harnessesQuery).data?.find((h) => h.kind === harness)?.label ?? harness
  const allowed = project?.allowed_modes[harness]
  const session = useMemo(() => draftSession(harness, allowed), [harness, allowed])
  const draft = useFirstSend(projectId, harness, planId)

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
      <header className="flex items-center justify-between gap-4 border-b border-border px-6 py-3">
        <div className="min-w-0">
          <h1 className="truncate text-sm font-medium">New conversation</h1>
          <p className="truncate text-xs text-faint-foreground">
            <bdi>{project?.name ?? '…'}</bdi> · Planner
          </p>
        </div>
        <CliMenu harness={harness} label={label} onPick={setHarness} />
      </header>
      <div className="flex flex-1 flex-col items-center justify-center gap-2 p-8 text-center">
        <MessageSquarePlus aria-hidden className="size-8 text-accent-line" />
        <p className="text-sm text-muted-foreground">
          The conversation starts with your first message.
        </p>
      </div>
      {planId && (
        <div className="px-6 pb-2">
          <div className="mx-auto max-w-3xl">
            <div className="inline-flex max-w-full items-center gap-1.5 rounded-full border border-accent-line/50 bg-accent-softer py-0.5 pr-1 pl-2.5 text-xs text-secondary-foreground">
              <span dir="auto" className="truncate">
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
        to={{ draft }}
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

/** What a draft's composer offers before a session exists: the harness's
 * modes as the project allows them, no model chosen and no effort. */
function draftSession(harness: string, allowed: string[] | undefined): SessionView {
  if (allowed === undefined) return { state: 'connecting' }
  const policy = policyOf(harness)
  const choices: SessionChoices = {
    current: {
      mode: policy.initial ?? policy.modes[0]?.id ?? '',
      model: UNOPENED.id,
      effort: null,
    },
    modes: policy.modes.map((m) => {
      const enabled = allowed.includes(m.id)
      return {
        id: m.id,
        label: m.label,
        description: null,
        enabled,
        reason: enabled ? null : 'Not allowed in this project',
      }
    }),
    models: [UNOPENED],
    efforts: [],
  }
  return { state: 'ready', choices }
}

/** Send from a draft: create the thread, open its session for the model it
 * holds, start the turn with the chosen mode, then replace the draft's URL
 * with the thread's. A failed create stays in the draft for a retry with the
 * same command id; a turn that fails once the thread exists goes to the
 * thread with its text and error, so a retry never makes a second thread.
 * A person who left the draft while it sent stays where they went: the
 * thread shows in the sidebar, and nothing pulls them back to it. */
function useFirstSend(projectId: string, harness: string, planId?: string) {
  const queryClient = useQueryClient()
  const navigate = useNavigate()
  const create = useRef<Attempt | null>(null)
  const shown = useRef(true)
  useEffect(() => {
    shown.current = true
    return () => {
      shown.current = false
    }
  }, [])

  return async (commandId: string, text: string, chosen: TurnSettings): Promise<string> => {
    const request = { title: DRAFT_TITLE, harness }
    create.current = attemptFor(create.current, request)
    const thread = await createThread(projectId, { ...request, command_id: create.current.commandId })
    void queryClient.invalidateQueries({ queryKey: threadsQuery(projectId).queryKey })
    const open = async () => {
      if (!shown.current) return
      await navigate({
        to: '/projects/$projectId/threads/$threadId',
        params: { projectId, threadId: thread.id },
        replace: true,
      })
    }

    let settings: TurnSettings | null = null
    try {
      const choices = await openSession(thread.id)
      queryClient.setQueryData(sessionKey(thread.id), choices)
      settings = { mode: chosen.mode, model: choices.current.model, effort: choices.current.effort }
      const operationId = await startTurn(thread.id, commandId, text, settings, {
        plan: planId ?? null,
        clientTab: tabId(),
      })
      void open()
      return operationId
    } catch (error) {
      // The thread page's composer fingerprints what it sends the same way,
      // so sending the same text and settings there replays this command.
      const request = { text, settings: settings ?? chosen, focus: null }
      carrySend(thread.id, {
        text,
        attempt: attemptFor(null, request, () => commandId),
        error: error instanceof Error ? error : new Error(String(error)),
      })
      void open()
      throw error
    }
  }
}
