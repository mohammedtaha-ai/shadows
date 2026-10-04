// Accumulate hierarchy pages only while their workspace revisions agree.
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useState } from 'react'
interface Page<T> { revision: number; items: T[]; next?: string | null }
export function useDesignPages<T>(baseKey: readonly unknown[], load: (after?: string) => Promise<Page<T>>) {
  const client = useQueryClient()
  const queryKey = [...baseKey, null]
  const first = useQuery({ queryKey, queryFn: () => load() })
  const [extra, setExtra] = useState<{ revision: number; pages: Page<T>[] } | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<Error | null>(null)
  const pages = first.data === undefined ? [] : [first.data, ...(extra?.revision === first.data.revision ? extra.pages : [])]
  const next = pages.at(-1)?.next
  const more = async () => {
    if (!next || !first.data) return
    setLoading(true); setError(null)
    try {
      const page = await client.fetchQuery({ queryKey: [...baseKey, next], queryFn: () => load(next), staleTime: 0 })
      if (page.revision !== first.data.revision) {
        setExtra(null); await client.invalidateQueries({ queryKey })
      } else setExtra({ revision: page.revision, pages: [...(extra?.revision === page.revision ? extra.pages : []), page] })
    } catch (e) { setError(e instanceof Error ? e : new Error(String(e))); void client.invalidateQueries({ queryKey }) }
    finally { setLoading(false) }
  }
  return { items: pages.flatMap(page => page.items), next, more, loading, pending: first.isPending, error: first.error ?? error }
}
