import { client, unwrap } from './transport'
import type { components } from './schema'
import { queryOptions, type QueryClient } from '@tanstack/react-query'

import { writeSaved } from './saved-answer'

export const agreementKeys = {
  list: (projectId: string) => ['projects', projectId, 'agreements'],
  versions: (projectId: string) => ['projects', projectId, 'agreement'],
  version: (projectId: string, id: string | undefined, version?: number) => ['projects', projectId, 'agreement', id, version],
}
export function agreementsQuery(projectId: string) {
  return queryOptions({ queryKey: agreementKeys.list(projectId), queryFn: () => listAgreements(projectId) })
}
export function agreementQuery(projectId: string, id: string | undefined, version?: number) {
  return queryOptions({ queryKey: agreementKeys.version(projectId, id, version),
    queryFn: () => getAgreement(projectId, id!, version), enabled: id !== undefined })
}
export function saveAgreementAnswer(client: QueryClient, projectId: string, id: string | undefined, version: number | undefined, saved: AgreementVersion) {
  return writeSaved(client, agreementKeys.version(projectId, id, version), saved)
}

export function invalidateAgreementVersions(client: QueryClient, projectId: string): void {
  void client.invalidateQueries({ queryKey: agreementKeys.versions(projectId) })
}

export function invalidateAgreements(client: QueryClient, projectId: string): void {
  void client.invalidateQueries({ queryKey: agreementKeys.list(projectId) })
  invalidateAgreementVersions(client, projectId)
}


export type AgreementVersion = components['schemas']['AgreementVersion']

export type AgreementContent = components['schemas']['AgreementContent']

export type AgreementReview = components['schemas']['AgreementReview']

export type AgreementBinding = components['schemas']['AgreementBinding']

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
