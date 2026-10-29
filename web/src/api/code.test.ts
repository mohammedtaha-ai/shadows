import { QueryClient } from '@tanstack/react-query'
import { expect, it } from 'vitest'
import { codeKeys, codeSettingsQuery, codeStatusQuery, codeLinksQuery, saveCodeSettingsAnswer, saveUnlinkedCodeAnswer } from './code'
import type { ProjectLink } from './code'
it.each(['settings', 'unlink'])('%s saved answers survive stale reads', async kind => {
  const client = new QueryClient()
  const key = kind === 'settings' ? codeKeys.settings() : codeKeys.links('p')
  const old = kind === 'settings' ? { active_limit: 5, active_limit_min: 1, active_limit_max: 20 } : [{ linked: 'other' } as ProjectLink]
  client.setQueryData(key, old)
  let release!: (value: unknown) => void
  const pending = client.fetchQuery({ queryKey: key, queryFn: () => new Promise(resolve => { release = resolve }) }).catch(() => undefined)
  if (kind === 'settings') await saveCodeSettingsAnswer(client, { active_limit: 6, active_limit_min: 1, active_limit_max: 20 })
  else await saveUnlinkedCodeAnswer(client, 'p', 'other')
  release(old); await pending
  expect(client.getQueryData(key)).toEqual(kind === 'settings' ? { active_limit: 6, active_limit_min: 1, active_limit_max: 20 } : [])
  client.clear()
})
it('code query identities retain their arrays', () => {
  expect(codeSettingsQuery.queryKey).toEqual(['code', 'settings'])
  expect(codeStatusQuery('p').queryKey).toEqual(['projects', 'p', 'code', 'status'])
  expect(codeLinksQuery('p').queryKey).toEqual(['projects', 'p', 'code', 'links'])
})
