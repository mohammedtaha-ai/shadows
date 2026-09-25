// One job: how a plan version's state reads to a person.

import type { Plan } from '@/api/client'

/** "Draft v2", or "Approved v1": the client shows `Frozen` as Approved. */
export function stateLabel(plan: Pick<Plan, 'state' | 'version'>): string {
  return `${plan.state === 'Frozen' ? 'Approved' : 'Draft'} v${plan.version}`
}
