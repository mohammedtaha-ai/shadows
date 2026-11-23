// Where a topic is in the project, as CodeRecord answers it.
import { queryOptions } from '@tanstack/react-query'
import { retryLost } from './error'
import type { components } from './schema'
import { client, unwrap } from './transport'

export type WhereAnswer = components['schemas']['WhereAnswer']
export type WhereEntry = components['schemas']['WhereEntry']
export type FileEntry = components['schemas']['FileEntry']
export type ElementEntry = components['schemas']['ElementEntry']
export type SymbolEntry = components['schemas']['SymbolEntry']
export type UsesLive = components['schemas']['UsesLive']
export type Live = components['schemas']['Live']
export type UseLine = components['schemas']['UseLine']
export type UnrecordedFile = components['schemas']['UnrecordedFile']
export type Owner = components['schemas']['Owner']
export type FileTask = components['schemas']['FileTask']
export type Rank = components['schemas']['Rank']

export const codeRecordKeys = {
  where: (projectId: string, topic: string) => ['projects', projectId, 'code-record', 'where', topic],
}

/** The topic goes exactly as typed. Only the person's "Ask again" reads it again, never the stream. */
export function codeWhereQuery(projectId: string, topic: string) {
  return queryOptions({
    queryKey: codeRecordKeys.where(projectId, topic),
    queryFn: () => unwrap(client.GET('/api/projects/{id}/code-where', {
      params: { path: { id: projectId }, query: { topic } },
    })),
    retry: retryLost,
    staleTime: Infinity,
  })
}
