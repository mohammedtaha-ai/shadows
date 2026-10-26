import { QueryClient } from '@tanstack/react-query'
import { expect, it } from 'vitest'
import { threadKeys, threadsQuery, threadQuery, entriesQuery, queuedQuery, operationsQuery, saveThreadHarnessAnswer, saveRemovedThreadAnswer } from './threads'
import type { PlanningThread } from './threads'
it.each(['harness', 'remove'])('%s answers survive stale list and detail reads', async kind => {
  const client = new QueryClient()
  const old = { id: 't', project_id: 'p', harness: 'old' } as PlanningThread
  const saved = { ...old, harness: 'new', deleted_at: 'now' }
  client.setQueryData(threadKeys.list('p'), [old])
  let release!: (value: PlanningThread[]) => void
  const pending = client.fetchQuery({ ...threadsQuery('p'), queryFn: () => new Promise<PlanningThread[]>(resolve => { release = resolve }) }).catch(() => undefined)
  let releaseDetail!: (value: PlanningThread) => void
  const detail = kind === 'remove' ? client.fetchQuery({ ...threadQuery('t'), queryFn: () => new Promise<PlanningThread>(resolve => { releaseDetail = resolve }) }).catch(() => undefined) : undefined
  if (kind === 'remove') await saveRemovedThreadAnswer(client, saved)
  else await saveThreadHarnessAnswer(client, 'p', saved)
  release([old]); releaseDetail?.(old)
  await Promise.all([pending, detail])
  expect(client.getQueryData(threadKeys.list('p'))).toEqual(kind === 'remove' ? [] : [saved])
  if (kind === 'remove') expect(client.getQueryData(threadKeys.detail('t'))).toEqual(saved)
  client.clear()
})
it('thread queries preserve the stream refresh arrays', () => {
  expect(threadsQuery('p').queryKey).toEqual(['projects', 'p', 'threads'])
  expect(threadQuery('t').queryKey).toEqual(['threads', 't', 'detail'])
  expect(entriesQuery('t').queryKey).toEqual(['threads', 't', 'entries'])
  expect(queuedQuery('t').queryKey).toEqual(['threads', 't', 'queue'])
  expect(operationsQuery('t').queryKey).toEqual(['threads', 't', 'operations'])
})
