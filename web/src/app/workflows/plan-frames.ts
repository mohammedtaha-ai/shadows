import { usePlanEvents } from '@/stream'

/** The stream owner refreshes the writer's plan views while mounted. */
export function usePlanFrames(threadId: string | undefined, projectId?: string): void {
  usePlanEvents(threadId, projectId)
}
