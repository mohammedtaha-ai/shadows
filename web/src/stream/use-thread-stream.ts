// One job: binding a thread's `ThreadStream` to a React component's lifetime.

import { useQueryClient } from '@tanstack/react-query'
import { useEffect, useMemo, useSyncExternalStore } from 'react'
import { holdThreadRefresh } from './refresh'
import type { DurableEvent } from './frames'
import { type Notice, type StreamState } from './thread-stream'
import { threadBinding } from './thread-registry'

export type DurableListener = (event: DurableEvent, live: boolean, state: StreamState) => void
export type NoticeListener = (notice: Notice) => void

/** A thread's live stream while the calling component is mounted. Switching
 * `threadId` or unmounting closes the connection; it never stops a turn.
 *
 * Entries are not carried by the stream (a `ThreadEntryAppended` is only
 * `{ordinal, kind}`), so the thread's entries query is invalidated at every
 * `caught-up`, at every entry appended while live, and when a turn ends while
 * live — a turn's last word is settled only then — and fetched again.
 * The thread's operations query is invalidated at every `caught-up`, and
 * when a turn ends while live, for the observed values of its invocation.
 * A `ThreadRetitled` reads the thread lists again: at once while
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
  projectId?: string,
): StreamState & { retry: () => void } {
  const queryClient = useQueryClient()

  const { stream, listeners, noticeListeners } = useMemo(() => {
    const stream = threadBinding(queryClient, threadId)
    return { stream, listeners: stream.durable, noticeListeners: stream.notices }
  }, [threadId, queryClient])

  // Held before the store subscribes, so queries are refreshed before any
  // listener below hears the same event.
  useEffect(() => holdThreadRefresh(queryClient, threadId, { projectId }), [queryClient, threadId, projectId])

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
