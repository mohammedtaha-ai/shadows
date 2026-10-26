import { QueryClient } from '@tanstack/react-query'
import { expect, it } from 'vitest'
import { personKeys, personQuery, savePersonAnswer } from './person'
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
