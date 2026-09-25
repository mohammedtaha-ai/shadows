// One job: refetching plans when their thread's stream says a plan changed —
// one connection per thread, however many plans on screen watch it.
//
// A plan's changes reach a client only on its thread's stream (§13.10). The
// Workflows page has no conversation open, and a conversation may show many
// plan cards (§13.9); a stream per card would spend the browser's handful of
// connections to one origin. So watchers of one thread share one stream, which
// closes when the last of them unmounts — and a screen that already holds its
// thread's stream (the conversation) says so through `HeldThreadStream`, feeds
// that stream's plan events to `refetchPlans`, and no second one is opened.

import { type QueryClient, useQueryClient } from '@tanstack/react-query'
import { createContext, useContext, useEffect } from 'react'
import { subscribeUrl } from '@/api/client'
import { workflowsKey } from '@/api/queries'
import { ThreadStream } from '@/stream/thread-stream'

/** The durable events after which a plan, its neighbours or its list read differently. */
const PLAN_KINDS: ReadonlySet<string> = new Set([
  'WorkflowEdited',
  'WorkflowFrozen',
  'WorkflowDraftStarted',
])

/** Whether a durable event of `kind` changes how a plan, its neighbours or
 * its list read. */
export function isPlanEvent(kind: string): boolean {
  return PLAN_KINDS.has(kind)
}

/** Reads every plan on screen again. */
export function refetchPlans(queryClient: QueryClient): void {
  void queryClient.invalidateQueries({ queryKey: workflowsKey })
}

/** The thread whose stream an enclosing screen holds and reads plan events
 * from; `usePlanFrames` opens no stream of its own for it. */
export const HeldThreadStream = createContext<string | null>(null)

interface Watch {
  readonly stream: ThreadStream
  holders: number
}

const watches = new WeakMap<QueryClient, Map<string, Watch>>()

/** Keeps every plan query fresh from `threadId`'s stream while mounted;
 * `undefined` (the plan is not read yet) watches nothing, and neither does a
 * thread whose stream an enclosing screen holds. */
export function usePlanFrames(threadId: string | undefined): void {
  const queryClient = useQueryClient()
  const held = useContext(HeldThreadStream)
  useEffect(() => {
    if (threadId === undefined || threadId === held) return
    return watch(queryClient, threadId)
  }, [queryClient, threadId, held])
}

function watch(queryClient: QueryClient, threadId: string): () => void {
  let threads = watches.get(queryClient)
  if (threads === undefined) {
    threads = new Map()
    watches.set(queryClient, threads)
  }
  let held = threads.get(threadId)
  if (held === undefined) {
    const refetch = () => refetchPlans(queryClient)
    const stream: ThreadStream = new ThreadStream({
      url: (after) => subscribeUrl(threadId, after),
      // The replay ends in one refetch for the lot, which also covers any
      // change between the plan's first read and the stream opening.
      onCaughtUp: refetch,
      onDurable: (event) => {
        if (stream.getState().connection === 'live' && isPlanEvent(event.kind)) refetch()
      },
    })
    held = { stream, holders: 0 }
    threads.set(threadId, held)
    stream.start()
  }
  held.holders += 1

  const watching = held
  const map = threads
  return () => {
    watching.holders -= 1
    if (watching.holders === 0) {
      watching.stream.close()
      map.delete(threadId)
    }
  }
}
