// One job: refetching plans when their thread's stream says a plan changed —
// one connection per thread, however many plans on screen watch it.
//
// A plan's changes reach a client only on its thread's stream (§13.10). The
// Workflows page has no conversation open, and a conversation may show many
// plan cards (§13.9); a stream per card would spend the browser's handful of
// connections to one origin. So watchers of one thread share one stream, which
// closes when the last of them unmounts.

import { type QueryClient, useQueryClient } from '@tanstack/react-query'
import { useEffect } from 'react'
import { subscribeUrl } from '@/api/client'
import { workflowsKey } from '@/api/queries'
import { ThreadStream } from '@/stream/thread-stream'

/** The durable events after which a plan, its neighbours or its list read differently. */
const PLAN_KINDS: ReadonlySet<string> = new Set([
  'WorkflowEdited',
  'WorkflowFrozen',
  'WorkflowDraftStarted',
])

interface Watch {
  readonly stream: ThreadStream
  holders: number
}

const watches = new WeakMap<QueryClient, Map<string, Watch>>()

/** Keeps every plan query fresh from `threadId`'s stream while mounted;
 * `undefined` (the plan is not read yet) watches nothing. */
export function usePlanFrames(threadId: string | undefined): void {
  const queryClient = useQueryClient()
  useEffect(() => {
    if (threadId === undefined) return
    return watch(queryClient, threadId)
  }, [queryClient, threadId])
}

function watch(queryClient: QueryClient, threadId: string): () => void {
  let threads = watches.get(queryClient)
  if (threads === undefined) {
    threads = new Map()
    watches.set(queryClient, threads)
  }
  let held = threads.get(threadId)
  if (held === undefined) {
    const refetch = () => void queryClient.invalidateQueries({ queryKey: workflowsKey })
    const stream: ThreadStream = new ThreadStream({
      url: (after) => subscribeUrl(threadId, after),
      // The replay ends in one refetch for the lot, which also covers any
      // change between the plan's first read and the stream opening.
      onCaughtUp: refetch,
      onDurable: (event) => {
        if (stream.getState().connection === 'live' && PLAN_KINDS.has(event.kind)) refetch()
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
