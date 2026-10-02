// One job: the only door to the daemon. Every HTTP call the app makes is a
// function here, typed by `schema.d.ts`, which is generated from the daemon's
// own OpenAPI document (`npm run gen:api`); a route the daemon changed and
// this client did not follow fails the type check.
//
// A function is added here the day a screen first calls its route, not before.

import createClient from 'openapi-fetch'
import { unwrap, unwrapEmpty } from './error'
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
export type Plan = Schemas['Plan']
export type PlanTask = Schemas['PlanTask']
export type PlanLink = Schemas['Link']
export type PlanListing = Schemas['PlanListing']
export type PlanVersions = Schemas['PlanVersions']
export type VersionLine = Schemas['VersionLine']
export type WrittenBy = Schemas['WrittenBy']
export type Approved = Schemas['Approved']
export type Focus = Schemas['Focus']
export type InstructionsVersion = Schemas['InstructionsVersion']
export type Grant = Schemas['Grant']
export type IssuedGrant = Schemas['IssuedGrantBody']
export type ProjectStatus = Schemas['ProjectStatus']
export type IndexState = Schemas['IndexState']
export type ProjectLink = Schemas['ProjectLink']
export type CodeSettings = Schemas['CodeSettings']

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

/** Sets the thread's session to `model` at once, opening it if needed;
 * answers what it offers now, the new model's efforts included (spec §12.7).
 * The change writes nothing durable, so it carries no command id; an opening
 * it causes issues the thread's MCP grant, as any opening does (spec §13.7). */
export function changeModel(threadId: string, model: string): Promise<SessionChoices> {
  return unwrap(
    client.PUT('/api/threads/{id}/session/model', {
      params: { path: { id: threadId } },
      body: { model },
    }),
  )
}

/** Sets the thread's session to `effort` at once, opening it if needed;
 * answers what it offers now (spec §12.7). Like a model change, it writes
 * nothing durable and carries no command id. */
export function changeEffort(threadId: string, effort: string): Promise<SessionChoices> {
  return unwrap(
    client.PUT('/api/threads/{id}/session/effort', {
      params: { path: { id: threadId } },
      body: { effort },
    }),
  )
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

/** One conversation, including a removed one kept for history (§16.5). */
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

/** Starts a Planner turn with its settings, as one command (spec §12.7);
 * answers the operation it runs as. The command id is the caller's: a retry of
 * the same send passes the same one, and this function never makes one.
 * `focus` is the task the person points at, part of the command; `clientTab`
 * is the sending tab (§13.9), transport state that is not. */
export async function startTurn(
  threadId: string,
  commandId: string,
  prompt: string,
  settings: TurnSettings,
  {
    focus = null,
    plan,
    clientTab,
  }: { focus?: Focus | null; plan?: string | null; clientTab: string },
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
      },
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

/** Removes a project that holds no conversation (spec §4.2): it is listed
 * nowhere again, its code index, links and grants go, its folder is not
 * touched and its slug stays taken. Refused `PROJECT_HAS_THREADS` otherwise. */
export function removeProject(projectId: string, commandId: string): Promise<Project> {
  return unwrap(
    client.DELETE('/api/projects/{id}', {
      params: { path: { id: projectId }, query: { command_id: commandId } },
    }),
  )
}

/** How a project's code index stands (§15.5). Asking does not make it active. */
export function getCodeStatus(projectId: string): Promise<ProjectStatus> {
  return unwrap(
    client.GET('/api/projects/{id}/code/status', { params: { path: { id: projectId } } }),
  )
}

/** The projects whose index this one reads, by slug (§15.6). */
export function listCodeLinks(projectId: string): Promise<ProjectLink[]> {
  return unwrap(
    client.GET('/api/projects/{id}/code/links', { params: { path: { id: projectId } } }),
  )
}

/** Lets the project read `linkedId`'s index, one way (§15.6). */
export function putCodeLink(
  projectId: string,
  linkedId: string,
  commandId: string,
): Promise<ProjectLink> {
  return unwrap(
    client.PUT('/api/projects/{id}/code/links/{linked}', {
      params: { path: { id: projectId, linked: linkedId } },
      body: { command_id: commandId },
    }),
  )
}

/** Removes the link: the project no longer reads `linkedId`'s index. */
export function removeCodeLink(
  projectId: string,
  linkedId: string,
  commandId: string,
): Promise<void> {
  return unwrapEmpty(
    client.DELETE('/api/projects/{id}/code/links/{linked}', {
      params: { path: { id: projectId, linked: linkedId }, query: { command_id: commandId } },
    }),
  )
}

/** The code index's settings: how many projects are active at once (§15.6). */
export function getCodeSettings(): Promise<CodeSettings> {
  return unwrap(client.GET('/api/code/settings'))
}

/** Sets how many projects are active at once, 1 to 20 (§15.6). */
export function setActiveLimit(commandId: string, activeLimit: number): Promise<CodeSettings> {
  return unwrap(
    client.PUT('/api/code/settings', {
      body: { command_id: commandId, active_limit: activeLimit },
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

/** Each conversation's latest plan version in a project (spec §13.10), or archived ones too (§16.2). */
export function listPlans(projectId: string, archived = false): Promise<PlanListing[]> {
  return unwrap(
    client.GET('/api/projects/{id}/workflows', {
      params: { path: { id: projectId }, query: { archived } },
    }),
  )
}

/** One plan with every version, oldest first (§16.10). */
export function getPlanVersions(planId: string): Promise<PlanVersions> {
  return unwrap(client.GET('/api/plans/{id}', { params: { path: { id: planId } } }))
}

/** Archives a plan (§16.2). */
export function archivePlan(planId: string, commandId: string): Promise<PlanVersions> {
  return unwrap(
    client.POST('/api/plans/{id}/archive', {
      params: { path: { id: planId } },
      body: { command_id: commandId },
    }),
  )
}

/** Unarchives a plan (§16.2). */
export function unarchivePlan(planId: string, commandId: string): Promise<PlanVersions> {
  return unwrap(
    client.POST('/api/plans/{id}/unarchive', {
      params: { path: { id: planId } },
      body: { command_id: commandId },
    }),
  )
}

/** One plan version: tasks, links, revision, its neighbours, what blocks its
 * approval and what the last edit changed (spec §13.10). */
export function getPlan(workflowId: string): Promise<Plan> {
  return unwrap(client.GET('/api/workflows/{id}', { params: { path: { id: workflowId } } }))
}

/** Approves a Draft at the revision the person saw (spec §13.2). Answers what
 * the approval did, not the plan: the caller reads the plan again. */
export function approvePlan(
  workflowId: string,
  commandId: string,
  expectedRevision: number,
): Promise<Approved> {
  return unwrap(
    client.POST('/api/workflows/{id}/approve', {
      params: { path: { id: workflowId } },
      body: { command_id: commandId, expected_revision: expectedRevision },
    }),
  )
}

/** The project's current instructions, or `null` before the first save
 * (spec §13.8). */
export function getInstructions(projectId: string): Promise<InstructionsVersion | null> {
  return unwrap(
    client.GET('/api/projects/{id}/planner-instructions', { params: { path: { id: projectId } } }),
  )
}

/** Saves the project's instructions as its next version (spec §13.8). */
export function saveInstructions(
  projectId: string,
  commandId: string,
  body: string,
): Promise<InstructionsVersion> {
  return unwrap(
    client.PUT('/api/projects/{id}/planner-instructions', {
      params: { path: { id: projectId } },
      body: { command_id: commandId, body },
    }),
  )
}

/** A project's grants for external agents, revoked ones included, newest
 * first (spec §13.7). */
export function listGrants(projectId: string): Promise<Grant[]> {
  return unwrap(
    client.GET('/api/projects/{id}/mcp-grants', { params: { path: { id: projectId } } }),
  )
}

/** Connect: issues a grant bound to the project. Its `command` and `token`
 * are in this answer only, and `null` when the command is a replay. */
export function issueGrant(projectId: string, commandId: string): Promise<IssuedGrant> {
  return unwrap(
    client.POST('/api/projects/{id}/mcp-grants', {
      params: { path: { id: projectId } },
      body: { command_id: commandId },
    }),
  )
}

/** Revokes a project grant; Shadows refuses its token from then on. */
export function revokeGrant(grantId: string, commandId: string): Promise<Grant> {
  return unwrap(
    client.DELETE('/api/mcp-grants/{id}', {
      params: { path: { id: grantId }, query: { command_id: commandId } },
    }),
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

/** Plan notifications of one project, resumed by the existing journal cursor. */
export function projectEventsUrl(projectId: string, after: number): string {
  const url = new URL(`/api/projects/${encodeURIComponent(projectId)}/events`, DAEMON_URL)
  url.searchParams.set('after', String(after))
  return url.toString()
}
