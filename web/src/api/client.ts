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

/** The daemon's origin, without a trailing slash. */
export const DAEMON_URL = (import.meta.env.VITE_SHADOWS_URL ?? 'http://127.0.0.1:4318').replace(
  /\/+$/,
  '',
)

const client = createClient<paths>({ baseUrl: DAEMON_URL })

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

/** Starts a Planner turn; answers the operation it runs as. */
export async function startTurn(threadId: string, prompt: string): Promise<string> {
  const started = await unwrap(
    client.POST('/api/threads/{id}/turns', {
      params: { path: { id: threadId } },
      body: { prompt },
    }),
  )
  return started.operation_id
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
