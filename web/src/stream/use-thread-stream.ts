// One job: binding a thread's `ThreadStream` to a React component's lifetime.

import { useQueryClient } from '@tanstack/react-query'
import { useEffect, useMemo, useSyncExternalStore } from 'react'
import { subscribeUrl } from '@/api/client'
import { threadEntriesKey } from '@/api/queries'
import { type StreamState, ThreadStream } from './thread-stream'

/** A thread's live stream while the calling component is mounted. Switching
 * `threadId` or unmounting closes the connection; it never stops a turn.
 *
 * Entries are not carried by the stream (a `ThreadEntryAppended` is only
 * `{ordinal, kind}`), so the thread's entries query is invalidated at every
 * `caught-up` and at every entry appended while live, and fetched again
 * (ruling 51). */
export function useThreadStream(threadId: string): StreamState & { retry: () => void } {
  const queryClient = useQueryClient()

  // Constructing a ThreadStream opens nothing, so creating it during render is
  // safe; the effect below owns the connection.
  const stream = useMemo(() => {
    const refetchEntries = () =>
      void queryClient.invalidateQueries({ queryKey: threadEntriesKey(threadId) })
    const created: ThreadStream = new ThreadStream({
      url: (after) => subscribeUrl(threadId, after),
      onCaughtUp: refetchEntries,
      // Only while live. During any replay — the first, or the one after a
      // reconnect, when `caughtUp` is already true — the `caught-up` that
      // ends it refetches once for the lot.
      onDurable: (event) => {
        if (event.kind === 'ThreadEntryAppended' && created.getState().connection === 'live') {
          refetchEntries()
        }
      },
    })
    return created
  }, [threadId, queryClient])

  useEffect(() => {
    stream.start()
    return () => stream.close()
  }, [stream])

  const state = useSyncExternalStore(stream.subscribe, stream.getState)
  return { ...state, retry: stream.retry }
}
