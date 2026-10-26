// Domain HTTP operations pending migration to their owners.
import { client, unwrap } from './transport'
export { client, DAEMON_URL } from './transport'
import type { components } from './schema'

type Schemas = components['schemas']
export type AgreementVersion = Schemas['AgreementVersion']
export type AgreementContent = Schemas['AgreementContent']
export type AgreementReview = Schemas['AgreementReview']
export type AgreementBinding = Schemas['AgreementBinding']
export type PlanOp = Schemas['PlanOp']
export type DirectoryListing = Schemas['DirectoryListing']
export type DirectoryEntry = Schemas['DirectoryEntry']
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
export type EffectiveStandards = Schemas['EffectiveStandards']
export type StandardsAdditions = Schemas['StandardsAdditions']
export type StandardsAdditionsVersion = Schemas['StandardsAdditionsVersion']
export type StageView = Schemas['StageView']

/** The CLIs a conversation can run on, with what each remembers and its
 * latest reported limits (spec §12.10). */
export function listHarnesses(): Promise<HarnessInfo[]> {
  return unwrap(client.GET('/api/harnesses'))
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
