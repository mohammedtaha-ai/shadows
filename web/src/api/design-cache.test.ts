import { QueryClient } from '@tanstack/react-query'
import { expect, it } from 'vitest'
import { designKeys, partsQuery, outcomesQuery, visionQuery, designPageQuery, standardsQuery, saveStandardsAnswer } from './design'
import type { EffectiveStandards } from './client'
it('design queries retain null cursors and detail identities', () => {
  expect(visionQuery('p').queryKey).toEqual(['projects', 'p', 'design', 'vision'])
  expect(partsQuery('p').queryKey).toEqual(['projects', 'p', 'design', 'parts', null, null])
  expect(outcomesQuery('p', 'root', 'next').queryKey).toEqual(['projects', 'p', 'design', 'outcomes', 'root', 'next'])
  const branch = partsQuery('p').queryKey.slice(0, -1)
  expect(designPageQuery(branch, async () => null).queryKey).toEqual(partsQuery('p').queryKey)
  expect(designPageQuery(branch, async () => null, 'next').queryKey).toEqual(partsQuery('p', undefined, 'next').queryKey)
})
it('a stale standards read cannot overwrite a saved additions answer', async () => {
  const client = new QueryClient()
  const saved = { additions: null } as EffectiveStandards
  const version = { number: 2 } as NonNullable<EffectiveStandards['additions']>
  let release!: (value: EffectiveStandards) => void
  const pending = client.fetchQuery({ ...standardsQuery('p'), queryFn: () => new Promise<EffectiveStandards>(resolve => { release = resolve }) }).catch(() => undefined)
  await saveStandardsAnswer(client, 'p', saved, version)
  release(saved)
  await pending
  expect(client.getQueryData(designKeys.standards('p'))).toEqual({ ...saved, additions: version })
  client.clear()
})
