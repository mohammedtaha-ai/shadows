// Domain HTTP operations pending migration to their owners.
import { client, unwrap } from './transport'
export { client, DAEMON_URL } from './transport'
import type { components } from './schema'

type Schemas = components['schemas']
export type AgreementVersion = Schemas['AgreementVersion']
export type AgreementContent = Schemas['AgreementContent']
export type AgreementReview = Schemas['AgreementReview']
export type AgreementBinding = Schemas['AgreementBinding']
export type DirectoryListing = Schemas['DirectoryListing']
export type DirectoryEntry = Schemas['DirectoryEntry']
export type HarnessInfo = Schemas['HarnessInfo']
export type EffectiveStandards = Schemas['EffectiveStandards']
export type StandardsAdditions = Schemas['StandardsAdditions']
export type StandardsAdditionsVersion = Schemas['StandardsAdditionsVersion']
export type StageView = Schemas['StageView']

/** The CLIs a conversation can run on, with what each remembers and its
 * latest reported limits (spec §12.10). */
export function listHarnesses(): Promise<HarnessInfo[]> {
  return unwrap(client.GET('/api/harnesses'))
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
