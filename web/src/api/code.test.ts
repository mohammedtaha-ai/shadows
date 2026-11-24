import { QueryClient } from '@tanstack/react-query'
import { expect, it } from 'vitest'
import { codeKeys, codeServersQuery, codeLinksQuery, saveUnlinkedCodeAnswer } from './code'
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
  expect(codeLinksQuery('p').queryKey).toEqual(['projects', 'p', 'code', 'links'])
  expect(codeServersQuery('p').queryKey).toEqual(['projects', 'p', 'code', 'servers'])
})
it('code servers are polled only while one is starting', () => {
  const interval = (states: string[]) => {
    const fn = codeServersQuery('p').refetchInterval as unknown as (q: { state: { data: unknown } }) => unknown
    return fn({ state: { data: states.map(state => ({ state: { state } })) } })
  }
  expect(interval(['running', 'idle'])).toBe(false)
  expect(interval(['running', 'starting'])).toBe(5_000)
})
