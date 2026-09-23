// One job: the only door to the daemon. Every HTTP call the app makes is a
// function here, typed by `schema.d.ts`, which is generated from the daemon's
// own OpenAPI document (`npm run gen:api`); a route the daemon changed and
// this client did not follow fails the type check.
//
// A function is added here the day a screen first calls its route, not before.

import createClient from 'openapi-fetch'
import { unwrap } from './error'
import type { components, paths } from './schema'

export type Project = components['schemas']['Project']

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

/** The URL of a thread's event stream after `after` (spec §2.10). The stream
 * itself is not an openapi-fetch call — `EventSource` owns the connection —
 * but its address comes from here like every other. */
export function subscribeUrl(threadId: string, after: number): string {
  const url = new URL('/api/subscribe', DAEMON_URL)
  url.searchParams.set('thread_id', threadId)
  url.searchParams.set('after', String(after))
  return url.toString()
}
