// Domain HTTP operations pending migration to their owners.
import { client, DAEMON_URL, unwrap, unwrapEmpty } from './transport'
export { client, DAEMON_URL } from './transport'
import type { components } from './schema'

type Schemas = components['schemas']
export type Project = Schemas['Project']
export type AgreementVersion = Schemas['AgreementVersion']
export type AgreementContent = Schemas['AgreementContent']
export type AgreementReview = Schemas['AgreementReview']
export type AgreementBinding = Schemas['AgreementBinding']
export type PlanOp = Schemas['PlanOp']
export type DirectoryListing = Schemas['DirectoryListing']
export type DirectoryEntry = Schemas['DirectoryEntry']
export type CreateProject = Schemas['CreateProject']
export type HarnessInfo = Schemas['HarnessInfo']
export type Plan = Schemas['Plan']
export type PlanTask = Schemas['PlanTask']
export type PlanLink = Schemas['Link']
export type LinkedTask = Schemas['LinkedTask']
export type PlanMap = Schemas['PlanMap']
export type MapPlan = Schemas['MapPlan']
export type PlanListing = Schemas['PlanListing']
export type PlanVersions = Schemas['PlanVersions']
export type VersionLine = Schemas['VersionLine']
export type WrittenBy = Schemas['WrittenBy']
export type Approved = Schemas['Approved']
export type InstructionsVersion = Schemas['InstructionsVersion']
export type EffectiveStandards = Schemas['EffectiveStandards']
export type StandardsAdditions = Schemas['StandardsAdditions']
export type StandardsAdditionsVersion = Schemas['StandardsAdditionsVersion']
export type StageView = Schemas['StageView']
export type Grant = Schemas['Grant']
export type IssuedGrant = Schemas['IssuedGrantBody']
export type ProjectStatus = Schemas['ProjectStatus']
export type IndexState = Schemas['IndexState']
export type ProjectLink = Schemas['ProjectLink']
export type CodeSettings = Schemas['CodeSettings']

/** The CLIs a conversation can run on, with what each remembers and its
 * latest reported limits (spec §12.10). */
export function listHarnesses(): Promise<HarnessInfo[]> {
  return unwrap(client.GET('/api/harnesses'))
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

/** Each project's plan by its latest version, or archived ones too (§16.2). */
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

export function getPlanMap(projectId: string): Promise<PlanMap> {
  return unwrap(client.GET('/api/projects/{id}/plan-map', { params: { path: { id: projectId } } }))
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

/** Compiled-in standards with this project's latest additions (§23.2). */
export function getStandards(projectId: string): Promise<EffectiveStandards> {
  return unwrap(client.GET('/api/projects/{id}/standards', { params: { path: { id: projectId } } }))
}

/** Saves project additions as a new version; retries carry the caller's command id. */
export function saveStandardsAdditions(projectId: string, commandId: string,
  content: StandardsAdditions): Promise<StandardsAdditionsVersion> {
  return unwrap(client.PUT('/api/projects/{id}/standards/additions', {
    params: { path: { id: projectId } }, body: { command_id: commandId, content },
  }))
}

/** The stage computed by the design service (§23.4). */
export function getStage(projectId: string): Promise<StageView> {
  return unwrap(client.GET('/api/projects/{id}/stage', { params: { path: { id: projectId } } }))
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

/** Plan notifications of one project, resumed by the existing journal cursor. */
export function projectEventsUrl(projectId: string, after: number): string {
  const url = new URL(`/api/projects/${encodeURIComponent(projectId)}/events`, DAEMON_URL)
  url.searchParams.set('after', String(after))
  return url.toString()
}

export function listAgreements(projectId: string): Promise<AgreementVersion[]> {
  return unwrap(client.GET('/api/projects/{id}/agreements', { params: { path: { id: projectId } } }))
}
export function getAgreement(projectId: string, agreement: string, version?: number): Promise<AgreementVersion> {
  return unwrap(client.GET('/api/projects/{id}/agreements/{agreement}', {
    params: { path: { id: projectId, agreement }, query: { version } },
  }))
}
export function startAgreement(projectId: string, command_id: string, content?: AgreementContent,
  agreement_id?: string, reason?: string): Promise<AgreementVersion> {
  return unwrap(client.POST('/api/projects/{id}/agreements', {
    params: { path: { id: projectId } }, body: { command_id, content, agreement_id, reason },
  }))
}
export function editAgreement(projectId: string, agreement: string, command_id: string,
  version: number, expected_revision: number, content: AgreementContent): Promise<AgreementVersion> {
  return unwrap(client.PUT('/api/projects/{id}/agreements/{agreement}', {
    params: { path: { id: projectId, agreement } }, body: { command_id, version, expected_revision, content },
  }))
}
export function reviewAgreement(projectId: string, agreement: string): Promise<AgreementReview> {
  return unwrap(client.GET('/api/projects/{id}/agreements/{agreement}/review', {
    params: { path: { id: projectId, agreement } },
  }))
}
export function agreeAgreement(projectId: string, agreement: string, command_id: string,
  expected_revision: number, review_id: string): Promise<AgreementVersion> {
  return unwrap(client.POST('/api/projects/{id}/agreements/{agreement}/agree', {
    params: { path: { id: projectId, agreement } }, body: { command_id, expected_revision, review_id },
  }))
}
export function editBindings(id: string, command_id: string, expected_revision: number,
  ops: PlanOp[]): Promise<Schemas['EditOutcome']> {
  return unwrap(client.POST('/api/workflows/{id}/bindings', {
    params: { path: { id } }, body: { command_id, expected_revision, ops },
  }))
}
