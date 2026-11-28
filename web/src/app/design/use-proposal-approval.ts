import { useMutation, useQuery, useQueryClient, type UseQueryOptions } from '@tanstack/react-query'
import { useRef } from 'react'
import { type Attempt, attemptFor } from '@/api/command-id'
import { designKeys, type MapProposal, type RoadmapProposal, type VisionProposal } from '@/api/design'

type Proposal = VisionProposal | MapProposal | RoadmapProposal

export function useProposalApproval<T extends Proposal>({
  projectId, proposalId, proposalsQuery, approveProposal, approvedSuffix,
}: {
  projectId: string
  proposalId: string | null
  proposalsQuery: UseQueryOptions<T[], Error, T[], string[]>
  approveProposal: (projectId: string, proposalId: string, commandId: string) => Promise<unknown>
  approvedSuffix?: (live: T) => string
}) {
  const client = useQueryClient()
  const query = useQuery({ ...proposalsQuery, enabled: proposalId !== null })
  const attempt = useRef<Attempt | null>(null)
  const approve = useMutation({
    mutationFn: (sent: { proposalId: string; commandId: string }) =>
      approveProposal(projectId, sent.proposalId, sent.commandId),
    onSuccess: () => {
      attempt.current = null
      void client.invalidateQueries({ queryKey: designKeys.all(projectId) })
    },
    onError: () => {
      void client.invalidateQueries({ queryKey: proposalsQuery.queryKey })
    },
  })
  const live = query.data?.find(p => p.id === proposalId)
  const status = live?.status === 'Approved' ? `Approved${approvedSuffix?.(live) ?? ''}`
    : live?.status === 'Superseded' ? 'Superseded'
    : live?.outdated ? 'Outdated — ask the Planner to propose again'
    : live?.status === 'Pending' ? 'Pending'
    : query.isPending ? 'Loading status…' : 'Status unavailable'
  const submit = () => {
    if (proposalId === null) return
    attempt.current = attemptFor(attempt.current, { projectId, proposalId })
    approve.mutate({ proposalId, commandId: attempt.current.commandId })
  }
  return {
    projectId, status, approved: live?.status === 'Approved', approvedBy: live?.approved_by,
    canApprove: live?.status === 'Pending' && !live.outdated && query.error === null,
    isPending: approve.isPending, queryError: query.error, approvalError: approve.error, submit,
  }
}

export type ProposalApprovalState = ReturnType<typeof useProposalApproval>
