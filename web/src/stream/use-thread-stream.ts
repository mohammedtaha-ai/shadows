// One job: binding a thread's `ThreadStream` to a React component's lifetime.

import { useQueryClient } from '@tanstack/react-query'
import { useEffect, useMemo, useSyncExternalStore } from 'react'
import { threadEntriesKey, threadOperationsKey } from '@/api/queries'
import type { DurableEvent } from './frames'
import { type Notice, type StreamState } from './thread-stream'
import { threadBinding } from './thread-registry'

/** Every project's thread list: a title lives there, and the stream names
 * the thread, not its project. */
const isThreadList = (key: readonly unknown[]) => (key[0] === 'projects' || key[0] === 'aylul-agent') && key[2] === 'threads'

/** Operation events after which the operation is over. */
const TERMINAL_KINDS: ReadonlySet<string> = new Set([
  'OperationCompleted',
  'OperationFailed',
  'OperationCancelled',
  'OperationInterrupted',
])

export type DurableListener = (event: DurableEvent, live: boolean, state: StreamState) => void
export type NoticeListener = (notice: Notice) => void

/** A thread's live stream while the calling component is mounted. Switching
 * `threadId` or unmounting closes the connection; it never stops a turn.
 *
 * Entries are not carried by the stream (a `ThreadEntryAppended` is only
 * `{ordinal, kind}`), so the thread's entries query is invalidated at every
 * `caught-up`, at every entry appended while live, and when a turn ends while
 * live — a turn's last word is settled only then — and fetched again (ruling
 * 51). The thread's operations query is invalidated at every `caught-up`, and
 * when a turn ends while live, for the observed values of its invocation.
 * A `ThreadRetitled` (spec §4.2) reads the thread lists again: at once while
 * live, at the `caught-up` after a replay that held one.
 *
 * `onDurable` sees every durable event once, replayed or live, in `seq` order;
 * `live` says which, and `state` is the stream as it stood when the event
 * arrived. `onNotice` sees each `usage` and `options` frame. Either may change
 * between renders. */
export function useThreadStream(
  threadId: string,
  onDurable?: DurableListener,
  onNotice?: NoticeListener,
): StreamState & { retry: () => void } {
  const queryClient = useQueryClient()

  const { stream, listeners, noticeListeners } = useMemo(() => {
    const stream = threadBinding(queryClient, threadId)
    const listeners = stream.durable
    const noticeListeners = stream.notices
    const refetch = (key: readonly unknown[]) => void queryClient.invalidateQueries({ queryKey: key })
    const refetchTitles = () =>
      void queryClient.invalidateQueries({ predicate: (query) => isThreadList(query.queryKey) })
    // Whether the replay under way held a `ThreadRetitled`.
    const replay = { retitled: false }
    stream.caughtUp.add(() => {
      refetch(threadEntriesKey(threadId))
      refetch(threadOperationsKey(threadId))
      if (replay.retitled) refetchTitles()
      replay.retitled = false
    })
    // Only while live. During any replay — the first, or the one after a
    // reconnect, when `caughtUp` is already true — the `caught-up` that
    // ends it refetches once for the lot.
    stream.durable.add((event, live) => {
      if (live && (event.kind === 'ThreadEntryAppended' || TERMINAL_KINDS.has(event.kind))) {
        refetch(threadEntriesKey(threadId))
      }
      // A turn's ending settles its invocation's observed values (spec §12.8).
      if (live && TERMINAL_KINDS.has(event.kind)) refetch(threadOperationsKey(threadId))
      if (event.kind === 'ThreadRetitled') {
        if (live) refetchTitles()
        else replay.retitled = true
      }
    })
    return { stream, listeners, noticeListeners }
  }, [threadId, queryClient])

  useEffect(() => {
    if (onNotice === undefined) return
    noticeListeners.add(onNotice)
    return () => {
      noticeListeners.delete(onNotice)
    }
  }, [noticeListeners, onNotice])

  // Declared before the effect that connects, so the listener is in place
  // before the first frame can arrive.
  useEffect(() => {
    if (onDurable === undefined) return
    listeners.add(onDurable)
    return () => {
      listeners.delete(onDurable)
    }
  }, [listeners, onDurable])

  const state = useSyncExternalStore(stream.subscribe, stream.getState)
  return { ...state, retry: stream.retry }
}
