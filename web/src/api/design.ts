// The HTTP calls for the project design workspace.
import { client } from './client'
import { unwrap } from './error'
import type { components } from './schema'

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
