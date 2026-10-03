// Accumulate one branch's pages only while their workspace revisions agree.
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
import { type PartPage } from '@/api/design'
import { partsQuery } from '@/api/queries'

export function usePartPages(projectId: string, parent?: string) {
  const client = useQueryClient()
  const first = useQuery(partsQuery(projectId, parent))
  const [extra, setExtra] = useState<{ revision: number; pages: PartPage[] } | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<Error | null>(null)
  const pages = first.data === undefined ? [] : [first.data, ...(extra?.revision === first.data.revision ? extra.pages : [])]
  const next = pages.at(-1)?.next
  const more = async () => {
    if (!next || !first.data) return
    setLoading(true); setError(null)
    try {
      const page = await client.fetchQuery({ ...partsQuery(projectId, parent, next), staleTime: 0 })
      if (page.revision !== first.data.revision) {
        setExtra(null)
        await client.invalidateQueries({ queryKey: partsQuery(projectId, parent).queryKey })
      } else setExtra({ revision: page.revision, pages: [...(extra?.revision === page.revision ? extra.pages : []), page] })
    } catch (e) { setError(e instanceof Error ? e : new Error(String(e))); void client.invalidateQueries({ queryKey: partsQuery(projectId, parent).queryKey }) }
    finally { setLoading(false) }
  }
  return { items: pages.flatMap(page => page.items), next, more, loading, pending: first.isPending, error: first.error ?? error }
}
