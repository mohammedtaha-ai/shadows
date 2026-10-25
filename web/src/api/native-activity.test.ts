import { QueryClient } from '@tanstack/react-query'
import { expect, it, vi } from 'vitest'
import { nativeActivityKeys, nativeActivityQuery } from './native-activity'
it('native activity retains its key and visible-panel polling without owning transport', async () => {
  const client = new QueryClient()
  const read = vi.fn(async () => ({ sessions: [], pending: [], hooksInstalled: false }))
  const query = nativeActivityQuery(read, 1234)
  expect(query.queryKey).toEqual(['desktop-local-activity'])
  expect(query.refetchInterval).toBe(1234)
  expect(query.refetchIntervalInBackground).toBe(false)
  expect(query.retry).toBe(false)
  await client.fetchQuery(query)
  expect(read).toHaveBeenCalledOnce()
  expect(client.getQueryData(nativeActivityKeys.state())).toEqual({ sessions: [], pending: [], hooksInstalled: false })
  client.clear()
})
