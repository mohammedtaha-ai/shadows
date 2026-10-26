import { getOutcomes } from '@/api/design'
import { outcomesQuery } from '@/api/queries'
import { useDesignPages } from '@/api/design'
export function useOutcomePages(projectId: string, parent?: string) {
  return useDesignPages(outcomesQuery(projectId, parent).queryKey.slice(0, -1), after => getOutcomes(projectId, parent, after))
}
