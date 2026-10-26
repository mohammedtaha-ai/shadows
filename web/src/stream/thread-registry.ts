import type { QueryClient } from '@tanstack/react-query'
import { projectEventsUrl, subscribeUrl } from '@/api/client'
import type { DurableEvent } from './frames'
import { ThreadStream, type Notice, type StreamState } from './thread-stream'

export interface StreamSubscriber {
  onDurable?: (event: DurableEvent, live: boolean, state: StreamState) => void
  onNotice?: (notice: Notice) => void
  onCaughtUp?: () => void
  onState?: (state: StreamState) => void
}

interface Channel {
  stream: ThreadStream
  subscribers: Set<StreamSubscriber>
}

// Cache scopes also isolate independent clients representing separate tabs.
const channels = new WeakMap<QueryClient, Map<string, Channel>>()

function subscribe(
  client: QueryClient, key: string, url: (after: number) => string, subscriber: StreamSubscriber,
): () => void {
  let scope = channels.get(client)
  if (!scope) { scope = new Map(); channels.set(client, scope) }
  let channel = scope.get(key)
  const first = channel === undefined
  if (!channel) {
    const subscribers = new Set<StreamSubscriber>()
    const stream = new ThreadStream({
      url,
      onDurable: event => {
        const state = stream.getState()
        for (const listener of subscribers) listener.onDurable?.(event, state.connection === 'live', state)
      },
      onNotice: notice => { for (const listener of subscribers) listener.onNotice?.(notice) },
      onCaughtUp: () => { for (const listener of subscribers) listener.onCaughtUp?.() },
    })
    stream.subscribe(() => {
      for (const listener of subscribers) listener.onState?.(stream.getState())
    })
    channel = { stream, subscribers }
    scope.set(key, channel)
  }
  const held = channel
  held.subscribers.add(subscriber)
  subscriber.onState?.(held.stream.getState())
  if (first) held.stream.start()
  // A late watcher still needs to reconcile its initial read with the live feed.
  else if (held.stream.getState().connection === 'live') subscriber.onCaughtUp?.()
  return () => {
    if (!held.subscribers.delete(subscriber)) return
    if (held.subscribers.size === 0) {
      held.stream.close()
      // A later subscriber may already hold a fresh channel under this key.
      if (scope.get(key) === held) scope.delete(key)
    }
  }
}

export function subscribeThreadStream(client: QueryClient, threadId: string, subscriber: StreamSubscriber): () => void {
  return subscribe(client, `thread:${threadId}`, after => subscribeUrl(threadId, after), subscriber)
}

export function subscribeProjectStream(client: QueryClient, projectId: string, subscriber: StreamSubscriber): () => void {
  return subscribe(client, `project:${projectId}`, after => projectEventsUrl(projectId, after), subscriber)
}

/** A render can be abandoned: only subscribing acquires a connection. */
export function threadBinding(client: QueryClient, threadId: string) {
  let state: StreamState = {
    connection: 'connecting', caughtUp: false, lastSeq: 0,
    streaming: {}, labels: {}, subagents: {}, lastTurnEnd: null, problem: null,
  }
  const durable = new Set<NonNullable<StreamSubscriber['onDurable']>>()
  const notices = new Set<NonNullable<StreamSubscriber['onNotice']>>()
  return {
    durable, notices,
    getState: () => state,
    retry: () => channels.get(client)?.get(`thread:${threadId}`)?.stream.retry(),
    subscribe: (changed: () => void) => subscribeThreadStream(client, threadId, {
      onState: next => { state = next; changed() },
      onDurable: (event, live, current) => { for (const listener of durable) listener(event, live, current) },
      onNotice: notice => { for (const listener of notices) listener(notice) },
    }),
  }
}
