// Domain HTTP operations pending migration to their owners.
import { client, unwrap } from './transport'
export { client, DAEMON_URL } from './transport'
import type { components } from './schema'

type Schemas = components['schemas']
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
