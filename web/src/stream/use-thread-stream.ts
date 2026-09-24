// One job: binding a thread's `ThreadStream` to a React component's lifetime.

import { useQueryClient } from '@tanstack/react-query'
import { useEffect, useMemo, useSyncExternalStore } from 'react'
import { subscribeUrl } from '@/api/client'
import { threadEntriesKey, threadOperationsKey } from '@/api/queries'
import type { DurableEvent } from './frames'
import { type Notice, type StreamState, ThreadStream } from './thread-stream'

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
 * 51). The thread's operations query is invalidated at every `caught-up`.
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

  // Constructing a ThreadStream opens nothing, so creating it during render is
  // safe; the effect below owns the connection.
  const { stream, listeners, noticeListeners } = useMemo(() => {
    const listeners = new Set<DurableListener>()
    const noticeListeners = new Set<NoticeListener>()
    const refetch = (key: readonly unknown[]) => void queryClient.invalidateQueries({ queryKey: key })
    const created: ThreadStream = new ThreadStream({
      url: (after) => subscribeUrl(threadId, after),
      onCaughtUp: () => {
        refetch(threadEntriesKey(threadId))
        refetch(threadOperationsKey(threadId))
      },
      // Only while live. During any replay — the first, or the one after a
      // reconnect, when `caughtUp` is already true — the `caught-up` that
      // ends it refetches once for the lot.
      onDurable: (event) => {
        const live = created.getState().connection === 'live'
        if (live && (event.kind === 'ThreadEntryAppended' || TERMINAL_KINDS.has(event.kind))) {
          refetch(threadEntriesKey(threadId))
        }
        for (const listener of listeners) listener(event, live, created.getState())
      },
      onNotice: (notice) => {
        for (const listener of noticeListeners) listener(notice)
      },
    })
    return { stream: created, listeners, noticeListeners }
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

  useEffect(() => {
    stream.start()
    return () => stream.close()
  }, [stream])

  const state = useSyncExternalStore(stream.subscribe, stream.getState)
  return { ...state, retry: stream.retry }
}
