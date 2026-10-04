import { getParts } from '@/api/design'
import { partsQuery } from '@/api/queries'
import { useDesignPages } from './use-design-pages'
export function usePartPages(projectId: string, parent?: string) {
  return useDesignPages(partsQuery(projectId, parent).queryKey.slice(0, -1), after => getParts(projectId, parent, after))
}
