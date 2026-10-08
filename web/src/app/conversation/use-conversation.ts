// One job: binding one open conversation to the daemon — its stream, its
// entries, its turns — as the state its screen draws.

import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useEffect, useReducer } from 'react'
import { entriesQuery, operationsQuery, queuedQuery } from '@/api/queries'
import type { Limits, PlanShowFrame, UsageFrame } from '@/stream/frames'
import { tabId } from '@/stream/tab-id'
import { useThreadStream } from '@/stream/use-thread-stream'
import { isPlanEvent, refetchPlans } from '../workflows/plan-frames'
import { initialReply, replyReducer, shownReply } from './reply'
import { initialTurnState, latestTurn, runningTurn, turnReducer } from './turn-state'
import { type ContextFigures, latestContext } from './usage'
import { replaceCommands, useCommands } from './use-commands'
import { replaceChoices } from './use-session'

/** The durable events that change a thread's waiting messages (§20). */
const QUEUE_EVENTS = new Set([
  'MessageQueued',
  'QueuedMessageRemoved',
  'QueuedMessageSent',
  'QueuedMessageFailed',
])

/** Mount once per thread (key the caller by thread id): the reducers here
 * hold that one thread's turns and reply.
 *
 * The stream also keeps this thread's plans current, for its cards and side
 * panel (see `plan-frames.ts`). `onShowHere` hears a plan the Planner showed
 * for this tab (§13.9): a live `plan-show` naming this tab's id, never a
 * replayed card and never another tab's. */
export function useConversation(threadId: string, onShowHere?: (show: PlanShowFrame) => void) {
  const [turns, dispatchTurn] = useReducer(turnReducer, initialTurnState)
  const [reply, dispatchReply] = useReducer(replyReducer, initialReply)
  // The latest the harness reported live; a report that left a figure out
  // keeps the one before it.
  const [reported, dispatchUsage] = useReducer(
    (held: Reported, u: UsageFrame): Reported => ({
      context:
        u.contextUsed !== null || u.contextWindow !== null
          ? { contextUsed: u.contextUsed, contextWindow: u.contextWindow }
          : held.context,
      limits: u.limits ?? held.limits,
    }),
    { context: null, limits: null },
  )

  const queryClient = useQueryClient()
  const stream = useThreadStream(
    threadId,
    (event, live, current) => {
      dispatchTurn({ type: 'event', event, now: Date.now() })
      if (live && isPlanEvent(event.kind)) refetchPlans(queryClient)
      if (live && QUEUE_EVENTS.has(event.kind)) {
        void queryClient.invalidateQueries({ queryKey: queuedQuery(threadId).queryKey })
      }
      if (live && event.kind === 'ThreadEntryAppended') {
        const ordinal = agentOrdinal(event.payload)
        if (ordinal !== null) {
          dispatchReply({ type: 'agent-entry', ordinal, streaming: current.streaming })
        }
      }
    },
    (notice) => {
      if (notice.type === 'options') replaceChoices(queryClient, threadId, notice.choices)
      if (notice.type === 'commands') replaceCommands(queryClient, threadId, notice.commands)
      if (notice.type === 'usage') dispatchUsage(notice.usage)
      if (
        notice.type === 'plan-show' &&
        notice.show.threadId === threadId &&
        notice.show.targetTab === tabId()
      ) {
        onShowHere?.(notice.show)
      }
    },
  )

  // Each replay's end reads the plans and the waiting messages again, for any
  // change it carried.
  const live = stream.connection === 'live'
  useEffect(() => {
    if (!live) return
    refetchPlans(queryClient)
    void queryClient.invalidateQueries({ queryKey: queuedQuery(threadId).queryKey })
  }, [live, queryClient, threadId])

  const commands = useCommands(threadId)
  const entries = useQuery(entriesQuery(threadId))
  const operations = useQuery(operationsQuery(threadId))
  useEffect(() => {
    if (operations.data !== undefined) {
      dispatchTurn({ type: 'snapshot', operations: operations.data, now: Date.now() })
    }
  }, [operations.data])

  const running = runningTurn(turns)
  const runningText = running === null ? undefined : stream.streaming[running.id]
  useEffect(() => {
    dispatchReply({ type: 'stream', op: running?.id ?? null, text: runningText })
  }, [running?.id, runningText])

  const replyTurn = turns.find((turn) => turn.id === reply.op)
  const shown = shownReply(reply, {
    ordinals: new Set(entries.data?.map((entry) => entry.ordinal)),
    fetchedAt: entries.dataUpdatedAt,
    endedAt: replyTurn?.endedAt ?? null,
  })

  return {
    stream,
    entries,
    running,
    latest: latestTurn(turns),
    reply: shown,
    // Between its turn-end and its durable ending a turn still runs, but has
    // finished talking.
    thinking: running !== null && shown === null && stream.lastTurnEnd?.op !== running.id,
    label: running === null ? undefined : stream.labels[running.id],
    // Whether a turn is running is known: the snapshot answered or the
    // replay ended. Until then Send could start a second turn beside one.
    known: operations.isSuccess || stream.caughtUp,
    started: (id: string) => dispatchTurn({ type: 'started', id }),
    /** Each turn as the operations route last answered it, newest first. */
    operations: operations.data,
    /** Context as last reported: live, else as the newest turn ended with it. */
    context: reported.context ?? latestContext(operations.data ?? []),
    /** Account limits reported live on this conversation's stream, if any. */
    limits: reported.limits,
    /** The harness's `/` list for this conversation (§21), empty until sent. */
    commands,
  }
}

interface Reported {
  context: ContextFigures | null
  limits: Limits | null
}

/** The kinds a turn's agent writes: its text, its tool lines and its
 * subagent cards (§23.8). Each one moves the streamed reply on. */
const AGENT_KINDS: ReadonlySet<unknown> = new Set(['AgentMessage', 'ToolCall', 'Subagent'])

/** The ordinal of an announced entry the turn's agent wrote, else `null`. The
 * payload of `ThreadEntryAppended` is `{ordinal, kind}`. */
export function agentOrdinal(payload: unknown): number | null {
  if (typeof payload !== 'object' || payload === null) return null
  const { ordinal, kind } = payload as { ordinal?: unknown; kind?: unknown }
  return AGENT_KINDS.has(kind) && typeof ordinal === 'number' ? ordinal : null
}
