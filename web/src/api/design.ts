// The project design workspace's server state, from HTTP call to cache.
import { queryOptions, useQuery, useQueryClient, type QueryClient } from '@tanstack/react-query'
import { client, getStage, getStandards, type EffectiveStandards } from './client'
import { unwrap } from './error'
import { writeSaved } from './saved-answer'
import type { components } from './schema'
import { useState } from 'react'

export type VisionContent = components['schemas']['VisionContent']
export type VisionView = components['schemas']['VisionView']
export type VisionItem = components['schemas']['VisionItem']
export type VisionField = components['schemas']['VisionField']
export type VisionProposal = components['schemas']['VisionProposal']
export type MapProposal = components['schemas']['MapProposal']
export type RoadmapProposal = components['schemas']['RoadmapProposal']
export type ContractHead = components['schemas']['ContractHead']
export type DesignChange = components['schemas']['DesignChange']
export type DesignEdit = components['schemas']['DesignEdit']
export type Part = components['schemas']['Part']
export type PartContent = components['schemas']['PartContent']
export type PartView = components['schemas']['PartView']
export type PartPage = components['schemas']['PartPage']
export type StructureWritten = components['schemas']['StructureWritten']
export function createStructure(projectId: string, commandId: string): Promise<StructureWritten> {
  return unwrap(client.POST('/api/projects/{id}/structure', {
    params: {path:{id:projectId}}, body: {command_id:commandId},
  }))
}
export type Outcome = components['schemas']['Outcome']
export type OutcomeContent = components['schemas']['OutcomeContent']
export type OutcomeView = components['schemas']['OutcomeView']
export type OutcomePage = components['schemas']['OutcomePage']
export function getOutcome(projectId: string, outcomeId: string): Promise<OutcomeView> {
  return unwrap(client.GET('/api/projects/{id}/design/outcomes/{outcome}', { params: { path: { id: projectId, outcome: outcomeId } } }))
}
export function getOutcomes(projectId: string, parent?: string, after?: string): Promise<OutcomePage> {
  return unwrap(client.GET('/api/projects/{id}/design/outcomes', { params: { path: { id: projectId }, query: { parent, after } } }))
}

export function getPart(projectId: string, partId: string): Promise<PartView> {
  return unwrap(client.GET('/api/projects/{id}/design/parts/{part}', { params: { path: { id: projectId, part: partId } } }))
}
export function getParts(projectId: string, parent?: string, after?: string): Promise<PartPage> {
  return unwrap(client.GET('/api/projects/{id}/design/parts', { params: { path: { id: projectId }, query: { parent, after } } }))
}

export function getVision(projectId: string): Promise<VisionView> {
  return unwrap(client.GET('/api/projects/{id}/design/vision', { params: { path: { id: projectId } } }))
}

export function editDesign(projectId: string, body: DesignEdit): Promise<DesignChange> {
  return unwrap(client.POST('/api/projects/{id}/design/edits', { params: { path: { id: projectId } }, body }))
}

export function listVisionProposals(projectId: string): Promise<VisionProposal[]> {
  return unwrap(client.GET('/api/projects/{id}/vision-proposals', { params: { path: { id: projectId } } }))
}

export function listMapProposals(projectId: string): Promise<MapProposal[]> {
  return unwrap(client.GET('/api/projects/{id}/map-proposals', {params:{path:{id:projectId}}}))
}
export function approveMapProposal(projectId: string, proposalId: string, commandId: string): Promise<DesignChange> {
  return unwrap(client.POST('/api/projects/{id}/map-proposals/{proposal_id}/approve', {
    params:{path:{id:projectId,proposal_id:proposalId}},body:{command_id:commandId},
  }))
}

export function listRoadmapProposals(projectId: string): Promise<RoadmapProposal[]> {
  return unwrap(client.GET('/api/projects/{id}/roadmap-proposals', {params:{path:{id:projectId}}}))
}
export function approveRoadmapProposal(projectId: string, proposalId: string, commandId: string): Promise<DesignChange> {
  return unwrap(client.POST('/api/projects/{id}/roadmap-proposals/{proposal_id}/approve', {
    params:{path:{id:projectId,proposal_id:proposalId}},body:{command_id:commandId},
  }))
}

export function approveVisionProposal(projectId: string, proposalId: string, commandId: string): Promise<DesignChange> {
  return unwrap(client.POST('/api/projects/{id}/vision-proposals/{proposal_id}/approve', {
    params: { path: { id: projectId, proposal_id: proposalId } }, body: { command_id: commandId },
  }))
}

export const designKeys = {
  all: (projectId: string) => ['projects', projectId, 'design'],
  standards: (projectId: string) => ['projects', projectId, 'design', 'standards'],
  stage: (projectId: string) => ['projects', projectId, 'design', 'stage'],
  vision: (projectId: string) => ['projects', projectId, 'design', 'vision'],
  visionProposals: (projectId: string) => ['projects', projectId, 'design', 'vision-proposals'],
  mapProposals: (projectId: string) => ['projects', projectId, 'design', 'map-proposals'],
  roadmapProposals: (projectId: string) => ['projects', projectId, 'design', 'roadmap-proposals'],
  part: (projectId: string, partId: string) => ['projects', projectId, 'design', 'part', partId],
  outcome: (projectId: string, outcomeId: string) => ['projects', projectId, 'design', 'outcome', outcomeId],
  collection: (projectId: string, kind: 'parts' | 'outcomes') => ['projects', projectId, 'design', kind],
  page: (projectId: string, kind: 'parts' | 'outcomes', parent?: string, after?: string) => ['projects', projectId, 'design', kind, parent ?? null, after ?? null],
  preview: (projectId: string) => ['projects', projectId, 'design', 'structure-preview'],
}


export function standardsQuery(projectId: string) {
  return queryOptions({ queryKey: designKeys.standards(projectId), queryFn: () => getStandards(projectId) })
}


export function stageQuery(projectId: string) {
  return queryOptions({ queryKey: designKeys.stage(projectId), queryFn: () => getStage(projectId) })
}


export function visionQuery(projectId: string) {
  return queryOptions({
    queryKey: designKeys.vision(projectId),
    queryFn: () => getVision(projectId),
  })
}


export function visionProposalsQuery(projectId: string) {
  return queryOptions({ queryKey: designKeys.visionProposals(projectId),
    queryFn: () => listVisionProposals(projectId) })
}


export function mapProposalsQuery(projectId: string) {
  return queryOptions({queryKey:designKeys.mapProposals(projectId),
    queryFn:() => listMapProposals(projectId)})
}


export function roadmapProposalsQuery(projectId: string) {
  return queryOptions({queryKey:designKeys.roadmapProposals(projectId),
    queryFn:() => listRoadmapProposals(projectId)})
}


export function partQuery(projectId: string, partId: string) {
  return queryOptions({ queryKey: designKeys.part(projectId, partId), queryFn: () => getPart(projectId, partId) })
}

export function outcomeQuery(projectId: string, outcomeId: string) {
  return queryOptions({ queryKey: designKeys.outcome(projectId, outcomeId), queryFn: () => getOutcome(projectId, outcomeId) })
}

export function outcomesQuery(projectId: string, parent?: string, after?: string) {
  return queryOptions({ queryKey: designKeys.page(projectId, 'outcomes', parent, after), queryFn: () => getOutcomes(projectId, parent, after) })
}

export function partsQuery(projectId: string, parent?: string, after?: string) {
  return queryOptions({ queryKey: designKeys.page(projectId, 'parts', parent, after), queryFn: () => getParts(projectId, parent, after) })
}
export function designPageQuery<T>(baseKey: readonly unknown[], load: (after?: string) => Promise<T>, after?: string) {
  return queryOptions({ queryKey: [...baseKey, after ?? null], queryFn: () => load(after) })
}
export function saveStandardsAnswer(client: QueryClient, projectId: string, saved: EffectiveStandards, version: NonNullable<EffectiveStandards['additions']>) {
  // A journal refresh can already hold a newer saved version.
  return writeSaved<EffectiveStandards>(client, designKeys.standards(projectId), current =>
    (current?.additions?.number ?? 0) > version.number ? current : { ...(current ?? saved), additions: version })
}

// Accumulate hierarchy pages only while their workspace revisions agree.
interface Page<T> { revision: number; items: T[]; next?: string | null }
export function useDesignPages<T>(baseKey: readonly unknown[], load: (after?: string) => Promise<Page<T>>) {
  const client = useQueryClient()
  const queryKey = designPageQuery(baseKey, load).queryKey
  const first = useQuery(designPageQuery(baseKey, load))
  const [extra, setExtra] = useState<{ revision: number; pages: Page<T>[] } | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<Error | null>(null)
  const pages = first.data === undefined ? [] : [first.data, ...(extra?.revision === first.data.revision ? extra.pages : [])]
  const next = pages.at(-1)?.next
  const more = async () => {
    if (!next || !first.data) return
    setLoading(true); setError(null)
    try {
      const page = await client.fetchQuery({ ...designPageQuery(baseKey, load, next), staleTime: 0 })
      if (page.revision !== first.data.revision) {
        setExtra(null); await client.invalidateQueries({ queryKey })
      } else setExtra({ revision: page.revision, pages: [...(extra?.revision === page.revision ? extra.pages : []), page] })
    } catch (e) { setError(e instanceof Error ? e : new Error(String(e))); void client.invalidateQueries({ queryKey }) }
    finally { setLoading(false) }
  }
  return { items: pages.flatMap(page => page.items), next, more, loading, pending: first.isPending, error: first.error ?? error }
}

export function usePartPages(projectId: string, parent?: string) {
  return useDesignPages(partsQuery(projectId, parent).queryKey.slice(0, -1), after => getParts(projectId, parent, after))
}
