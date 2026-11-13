import { QueryClient } from '@tanstack/react-query'
import { expect, it } from 'vitest'
import { codeKeys, codeServersQuery, codeStatusQuery, codeLinksQuery, saveUnlinkedCodeAnswer } from './code'
import type { ProjectLink } from './code'
it('unlink saved answers survive stale reads', async () => {
  const client = new QueryClient()
  const key = codeKeys.links('p')
  const old = [{ linked: 'other' } as ProjectLink]
  client.setQueryData(key, old)
  let release!: (value: unknown) => void
  const pending = client.fetchQuery({ queryKey: key, queryFn: () => new Promise(resolve => { release = resolve }) }).catch(() => undefined)
  await saveUnlinkedCodeAnswer(client, 'p', 'other')
  release(old); await pending
  expect(client.getQueryData(key)).toEqual([])
  client.clear()
})
it('code query identities retain their arrays', () => {
  expect(codeStatusQuery('p').queryKey).toEqual(['projects', 'p', 'code', 'status'])
  expect(codeLinksQuery('p').queryKey).toEqual(['projects', 'p', 'code', 'links'])
  expect(codeServersQuery('p').queryKey).toEqual(['projects', 'p', 'code', 'servers'])
})
