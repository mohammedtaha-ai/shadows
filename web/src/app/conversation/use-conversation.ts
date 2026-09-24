// One job: binding one open conversation to the daemon — its stream, its
// entries, its turns — as the state its screen draws.

import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useEffect, useReducer } from 'react'
import { entriesQuery, operationsQuery } from '@/api/queries'
import { useThreadStream } from '@/stream/use-thread-stream'
import { initialReply, replyReducer, shownReply } from './reply'
import { initialTurnState, latestTurn, runningTurn, turnReducer } from './turn-state'
import { replaceChoices } from './use-session'

/** Mount once per thread (key the caller by thread id): the reducers here
 * hold that one thread's turns and reply. */
export function useConversation(threadId: string) {
  const [turns, dispatchTurn] = useReducer(turnReducer, initialTurnState)
  const [reply, dispatchReply] = useReducer(replyReducer, initialReply)

  const queryClient = useQueryClient()
  const stream = useThreadStream(
    threadId,
    (event, live, current) => {
      dispatchTurn({ type: 'event', event, now: Date.now() })
      if (live && event.kind === 'ThreadEntryAppended') {
        const ordinal = agentOrdinal(event.payload)
        if (ordinal !== null) {
          dispatchReply({ type: 'agent-entry', ordinal, streaming: current.streaming })
        }
      }
    },
    (notice) => {
      if (notice.type === 'options') replaceChoices(queryClient, threadId, notice.choices)
    },
  )

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
  }
}

/** The ordinal of an announced AgentMessage entry, else `null`. The payload
 * of `ThreadEntryAppended` is `{ordinal, kind}`. */
function agentOrdinal(payload: unknown): number | null {
  if (typeof payload !== 'object' || payload === null) return null
  const { ordinal, kind } = payload as { ordinal?: unknown; kind?: unknown }
  return kind === 'AgentMessage' && typeof ordinal === 'number' ? ordinal : null
}
