import type { TurnSettings } from './client'
import { client, DAEMON_URL, unwrap, unwrapEmpty } from './transport'
import type { components } from './schema'
import { queryOptions, type QueryClient } from '@tanstack/react-query'

import { writeSaved } from './saved-answer'
export const threadKeys = {
  list: (projectId: string) => ['projects', projectId, 'threads'],
  detail: (threadId: string) => ['threads', threadId, 'detail'],
  entries: (threadId: string) => ['threads', threadId, 'entries'] as const,
  queue: (threadId: string) => ['threads', threadId, 'queue'] as const,
  operations: (threadId: string) => ['threads', threadId, 'operations'] as const,
}


/** A project's planning threads, oldest first. A thread's own stream
 * refreshes the list on its `ThreadRetitled`, but the harness titles a thread
 * after its turn has ended, often once the person has left it, so the list is
 * also polled every 10 s while shown (spec §12.10). */
export function threadsQuery(projectId: string) {
  return queryOptions({
    queryKey: threadKeys.list(projectId),
    queryFn: () => listThreads(projectId),
    refetchInterval: 10_000,
  })
}


/** Historical conversation links still read a thread omitted from the live list. */
export function threadQuery(threadId: string) {
  return queryOptions({
    queryKey: threadKeys.detail(threadId),
    queryFn: () => getThread(threadId),
  })
}


/** A thread's entries (`GET /api/threads/{id}/entries`). A durable
 * `ThreadEntryAppended` carries only `{ordinal, kind}`, so the stream hook
 * invalidates this key and the entries are fetched again (ruling 51). */
export function threadEntriesKey(threadId: string) {
  return threadKeys.entries(threadId)
}


export function entriesQuery(threadId: string) {
  return queryOptions({ queryKey: threadEntriesKey(threadId), queryFn: () => listEntries(threadId) })
}


/** A thread's waiting messages (spec §20); the stream's queue events invalidate it. */
export function queuedQuery(threadId: string) {
  return queryOptions({
    queryKey: threadKeys.queue(threadId),
    queryFn: () => listQueued(threadId),
  })
}


/** A thread's operations, newest first: on opening a thread, whether a turn is
 * running and which. The stream's durable operation events carry it on from
 * there; the stream hook invalidates this key at each `caught-up`. */
export function threadOperationsKey(threadId: string) {
  return threadKeys.operations(threadId)
}


export function operationsQuery(threadId: string) {
  return queryOptions({
    queryKey: threadOperationsKey(threadId),
    queryFn: () => listOperations(threadId),
  })
}
export function saveThreadHarnessAnswer(client: QueryClient, projectId: string, thread: PlanningThread) {
  return writeSaved<PlanningThread[]>(client, threadKeys.list(projectId), threads =>
    threads?.map(value => value.id === thread.id ? thread : value))
}
export async function saveRemovedThreadAnswer(client: QueryClient, thread: PlanningThread) {
  await writeSaved(client, threadKeys.detail(thread.id), thread)
  await writeSaved<PlanningThread[]>(client, threadKeys.list(thread.project_id), threads =>
    threads?.filter(value => value.id !== thread.id))
}

export function invalidateEntries(client: QueryClient, threadId: string): void {
  void client.invalidateQueries({ queryKey: threadKeys.entries(threadId) })
}

export function invalidateOperations(client: QueryClient, threadId: string): void {
  void client.invalidateQueries({ queryKey: threadKeys.operations(threadId) })
}

export function invalidateQueue(client: QueryClient, threadId: string): void {
  void client.invalidateQueries({ queryKey: threadKeys.queue(threadId) })
}

export function invalidateThreadLists(client: QueryClient): void {
  void client.invalidateQueries({ predicate: query => {
    const key = query.queryKey
    return (key[0] === 'projects' || key[0] === 'aylul-agent') && key[2] === 'threads'
  } })
}


export type PlanningThread = components['schemas']['PlanningThread']

export type ThreadEntry = components['schemas']['ThreadEntry']

export type Attachment = components['schemas']['Attachment']

export type Operation = components['schemas']['Operation']

export type InvocationView = components['schemas']['InvocationView']

export type Focus = components['schemas']['Focus']

/** Uploads one immutable image; Core validates its bytes and media type. */
export function uploadAttachment(threadId: string, commandId: string, file: File): Promise<Attachment> {
  const body = new FormData()
  body.append('command_id', commandId)
  body.append('file', file)
  return unwrap(client.POST('/api/threads/{id}/attachments', {
    params: { path: { id: threadId } },
    body: { command_id: commandId, file: '' },
    bodySerializer: () => body,
  }))
}

/** The byte route used by thumbnails and durable messages. */
export function attachmentUrl(threadId: string, attachmentId: string): string {
  return `${DAEMON_URL}/api/threads/${encodeURIComponent(threadId)}/attachments/${encodeURIComponent(attachmentId)}`
}

export function listThreads(projectId: string): Promise<PlanningThread[]> {
  return unwrap(client.GET('/api/projects/{id}/threads', { params: { path: { id: projectId } } }))
}

/** One conversation, including a removed one kept for history. */
export function getThread(threadId: string): Promise<PlanningThread> {
  return unwrap(client.GET('/api/threads/{id}', { params: { path: { id: threadId } } }))
}

/** Remove a conversation; retry an unanswered request with the same command id. */
export function removeThread(threadId: string, commandId: string): Promise<PlanningThread> {
  return unwrap(client.DELETE('/api/threads/{id}', {
    params: { path: { id: threadId }, query: { command_id: commandId } },
  }))
}

export function createThread(projectId: string, body: CreateThread): Promise<PlanningThread> {
  return unwrap(
    client.POST('/api/projects/{id}/threads', { params: { path: { id: projectId } }, body }),
  )
}

/** A thread's entries in ordinal order. */
export function listEntries(threadId: string): Promise<ThreadEntry[]> {
  return unwrap(client.GET('/api/threads/{id}/entries', { params: { path: { id: threadId } } }))
}

/** A thread's operations (its turns), newest first. */
export function listOperations(threadId: string): Promise<Operation[]> {
  return unwrap(client.GET('/api/threads/{id}/operations', { params: { path: { id: threadId } } }))
}

/** Starts a Planner turn with its settings, as one command;
 * answers the operation it runs as. The command id is the caller's: a retry of
 * the same send passes the same one, and this function never makes one.
 * `focus` is the task the person points at, part of the command; `clientTab`
 * is the sending tab, transport state that is not. */
export async function startTurn(
  threadId: string,
  commandId: string,
  prompt: string,
  settings: TurnSettings,
  {
    focus = null,
    plan,
    clientTab,
    attachments,
  }: { focus?: Focus | null; plan?: string | null; clientTab: string; attachments?: string[] },
): Promise<string> {
  const started = await unwrap(
    client.POST('/api/threads/{id}/turns', {
      params: { path: { id: threadId } },
      body: {
        command_id: commandId,
        prompt,
        ...settings,
        focus,
        ...(plan != null ? { plan } : {}),
        client_tab: clientTab,
        ...(attachments === undefined ? {} : { attachments }),
      },
    }),
  )
  return started.operation_id
}

export type Queued = components['schemas']['Queued']

export type QueuedMessage = components['schemas']['QueuedMessage']

export type SentNow = components['schemas']['SentNow']

/** Writes while a turn runs: the message waits. */
export function queueMessage(
  threadId: string,
  commandId: string,
  prompt: string,
  settings: TurnSettings,
  {
    focus = null,
    plan,
    clientTab,
    attachments,
  }: { focus?: Focus | null; plan?: string | null; clientTab: string; attachments?: string[] },
): Promise<Queued> {
  return unwrap(
    client.POST('/api/threads/{id}/queue', {
      params: { path: { id: threadId } },
      body: {
        command_id: commandId,
        prompt,
        ...settings,
        focus,
        ...(plan != null ? { plan } : {}),
        client_tab: clientTab,
        ...(attachments === undefined ? {} : { attachments }),
      },
    }),
  )
}

/** A thread's waiting messages, in send order. */
export function listQueued(threadId: string): Promise<QueuedMessage[]> {
  return unwrap(client.GET('/api/threads/{id}/queue', { params: { path: { id: threadId } } }))
}

/** Removes a waiting message. */
export function unqueueMessage(threadId: string, id: string, commandId: string): Promise<void> {
  return unwrapEmpty(
    client.DELETE('/api/threads/{id}/queue/{qid}', {
      params: { path: { id: threadId, qid: id }, query: { command_id: commandId } },
    }),
  )
}

/** Sends a waiting message now: steers the running turn, or starts one. */
export function sendQueuedNow(threadId: string, id: string, commandId: string): Promise<SentNow> {
  return unwrap(
    client.POST('/api/threads/{id}/queue/{qid}/send-now', {
      params: { path: { id: threadId, qid: id } },
      body: { command_id: commandId },
    }),
  )
}

/** Changes the thread's CLI; refused once it has run a turn. */
export function setThreadHarness(
  threadId: string,
  commandId: string,
  harness: string,
): Promise<PlanningThread> {
  return unwrap(
    client.PATCH('/api/threads/{id}', {
      params: { path: { id: threadId } },
      body: { command_id: commandId, harness },
    }),
  )
}

/** Forks the thread at `atEntryId` into a new thread. */
export function forkThread(
  threadId: string,
  commandId: string,
  atEntryId: string,
): Promise<PlanningThread> {
  return unwrap(
    client.POST('/api/threads/{id}/fork', {
      params: { path: { id: threadId } },
      body: { command_id: commandId, at_entry_id: atEntryId },
    }),
  )
}

/** Stops a turn; answers the operation as it now stands. */
export function stopTurn(operationId: string): Promise<Operation> {
  return unwrap(
    client.POST('/api/operations/{id}/stop', { params: { path: { id: operationId } } }),
  )
}

/** The URL of a thread's event stream after `after`. The stream
 * itself is not an openapi-fetch call — `EventSource` owns the connection —
 * but its address comes from here like every other. */
export function subscribeUrl(threadId: string, after: number): string {
  const url = new URL('/api/subscribe', DAEMON_URL)
  url.searchParams.set('thread_id', threadId)
  url.searchParams.set('after', String(after))
  return url.toString()
}

export type CreateThread = components['schemas']['CreateThread']
