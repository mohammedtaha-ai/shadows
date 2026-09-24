// One job: the only door to the daemon. Every HTTP call the app makes is a
// function here, typed by `schema.d.ts`, which is generated from the daemon's
// own OpenAPI document (`npm run gen:api`); a route the daemon changed and
// this client did not follow fails the type check.
//
// A function is added here the day a screen first calls its route, not before.

import createClient from 'openapi-fetch'
import { unwrap } from './error'
import type { components, paths } from './schema'

type Schemas = components['schemas']
export type Project = Schemas['Project']
export type PlanningThread = Schemas['PlanningThread']
export type ThreadEntry = Schemas['ThreadEntry']
export type Operation = Schemas['Operation']
export type DirectoryListing = Schemas['DirectoryListing']
export type DirectoryEntry = Schemas['DirectoryEntry']
export type CreateProject = Schemas['CreateProject']
export type CreateThread = Schemas['CreateThread']
export type HarnessInfo = Schemas['HarnessInfo']
export type Choice = Schemas['Choice']
export type SessionChoices = Schemas['SessionChoices']
export type AccountLimits = Schemas['AccountLimits']
export type LimitWindow = Schemas['LimitWindow']
export type InvocationView = Schemas['InvocationView']
export type TurnSettings = Schemas['TurnSettings']
export type ContextBreakdown = Schemas['ContextBreakdown']

/** The daemon's origin, without a trailing slash. */
export const DAEMON_URL = (import.meta.env.VITE_SHADOWS_URL ?? 'http://127.0.0.1:4318').replace(
  /\/+$/,
  '',
)

// `fetch` is looked up on each call rather than captured once here (what
// openapi-fetch does by default), so a test that stubs it is heard however
// early this module was imported.
const client = createClient<paths>({ baseUrl: DAEMON_URL, fetch: (request) => fetch(request) })

/** The CLIs a conversation can run on, with what each remembers and its
 * latest reported limits (spec §12.10). */
export function listHarnesses(): Promise<HarnessInfo[]> {
  return unwrap(client.GET('/api/harnesses'))
}

/** Opens the thread's harness session if it is not open; answers what it
 * offers now. Idempotent: an open session answers what it holds (spec §12.2). */
export function openSession(threadId: string): Promise<SessionChoices> {
  return unwrap(client.POST('/api/threads/{id}/session', { params: { path: { id: threadId } } }))
}

/** The session's context breakdown, read on demand, or none with the reason
 * (spec §12.8). */
export function readContext(threadId: string): Promise<ContextBreakdown> {
  return unwrap(client.GET('/api/threads/{id}/context', { params: { path: { id: threadId } } }))
}

/** Every project, oldest first. Also the reachability probe: it is the
 * cheapest call that proves the daemon answers this origin. */
export function listProjects(): Promise<Project[]> {
  return unwrap(client.GET('/api/projects'))
}

/** Creates a project owning an existing directory. Replaying the same
 * `command_id` with the same body answers the first result. */
export function createProject(body: CreateProject): Promise<Project> {
  return unwrap(client.POST('/api/projects', { body }))
}

export function listThreads(projectId: string): Promise<PlanningThread[]> {
  return unwrap(client.GET('/api/projects/{id}/threads', { params: { path: { id: projectId } } }))
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

/** Starts a Planner turn with its settings, as one command (spec §12.7);
 * answers the operation it runs as. The command id is the caller's: a retry of
 * the same send passes the same one, and this function never makes one. */
export async function startTurn(
  threadId: string,
  commandId: string,
  prompt: string,
  settings: TurnSettings,
): Promise<string> {
  const started = await unwrap(
    client.POST('/api/threads/{id}/turns', {
      params: { path: { id: threadId } },
      body: { command_id: commandId, prompt, ...settings },
    }),
  )
  return started.operation_id
}

/** Changes the thread's CLI; refused once it has run a turn (spec §12.6). */
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

/** Sets the modes a project allows, per harness kind (spec §12.5). */
export function setProjectModes(
  projectId: string,
  commandId: string,
  allowedModes: Record<string, string[]>,
): Promise<Project> {
  return unwrap(
    client.PATCH('/api/projects/{id}', {
      params: { path: { id: projectId } },
      body: { command_id: commandId, allowed_modes: allowedModes },
    }),
  )
}

/** Forks the thread at `atEntryId` into a new thread (spec §12.9). */
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

/** One directory's subdirectories, or with no path the roots (drives). */
export function listDirs(path: string | null): Promise<DirectoryListing> {
  return unwrap(
    client.GET('/api/fs/dirs', { params: { query: path === null ? {} : { path } } }),
  )
}

/** Creates one directory named `name` inside `parent`. */
export function createDir(parent: string, name: string): Promise<DirectoryEntry> {
  return unwrap(client.POST('/api/fs/dirs', { body: { parent, name } }))
}

/** The URL of a thread's event stream after `after` (spec §2.10). The stream
 * itself is not an openapi-fetch call — `EventSource` owns the connection —
 * but its address comes from here like every other. */
export function subscribeUrl(threadId: string, after: number): string {
  const url = new URL('/api/subscribe', DAEMON_URL)
  url.searchParams.set('thread_id', threadId)
  url.searchParams.set('after', String(after))
  return url.toString()
}
