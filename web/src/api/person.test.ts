import { QueryClient } from '@tanstack/react-query'
import { afterEach, expect, it, vi } from 'vitest'
import { personKeys, personQuery, savePerson, savePersonAnswer } from './person'
afterEach(() => vi.unstubAllGlobals())
it('patches only supplied person fields with the caller command id', async () => {
  let sent: Request | undefined
  const saved = { name: 'Mo', aylul_calls_me: null, language: 'en', theme: 'dark', updated_at: null }
  vi.stubGlobal('fetch', (request: Request) => { sent = request; return Promise.resolve(Response.json(saved)) })
  expect(await savePerson('command-1', { name: 'Mo', aylul_calls_me: null })).toEqual(saved)
  expect(sent?.method).toBe('PATCH')
  expect(new URL(sent!.url).pathname).toBe('/api/me')
  expect(await sent!.json()).toEqual({ command_id: 'command-1', name: 'Mo', aylul_calls_me: null })
})
it('a stale person read cannot overwrite the owner saved answer', async () => {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  let release!: (value: never) => void
  const pending = client.fetchQuery({ ...personQuery, queryFn: () => new Promise<never>(resolve => { release = resolve }) }).catch(() => undefined)
  const saved = { name: 'Saved', aylul_calls_me: null, language: null, theme: 'dark' as const, updated_at: null }
  await savePersonAnswer(client, saved)
  release({ ...saved, name: 'Old' } as never)
  await pending
  expect(client.getQueryData(personKeys.record())).toEqual(saved)
  expect(personQuery.queryKey).toEqual(['person'])
  client.clear()
})
