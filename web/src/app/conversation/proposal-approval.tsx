import { Button } from '@/components/ui/button'
import { ErrorLine } from '../error-line'
import { ApprovedBy } from './approved-by'
import type { ProposalApprovalState } from './use-proposal-approval'

export function ProposalApproval({ approval, buttonLabel, textClass }: {
  approval: ProposalApprovalState
  buttonLabel: string
  textClass: string
}) {
  return <>
    <p role="status" className={textClass}>{approval.status}</p>
    {approval.approved && <p className={textClass}>
      <ApprovedBy by={approval.approvedBy} projectId={approval.projectId} /></p>}
    {approval.canApprove && <Button size="sm" onClick={approval.submit}
      disabled={approval.isPending}>{buttonLabel}</Button>}
    {approval.queryError !== null && <ErrorLine error={approval.queryError} />}
    {approval.approvalError !== null && <ErrorLine error={approval.approvalError} />}
  </>
}
