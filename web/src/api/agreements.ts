import { queryOptions, type QueryClient } from '@tanstack/react-query'
import { getAgreement, listAgreements, type AgreementVersion } from './client'
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
