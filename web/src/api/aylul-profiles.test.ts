import { expect, it } from 'vitest'
import { profileKeys, providerPresetsQuery, providersQuery, agentsQuery, agentThreadsQuery, agentEnvQuery } from './aylul-profiles'
it('profile queries and refresh factories retain their exact identities', () => {
  expect(providerPresetsQuery.queryKey).toEqual(['aylul-provider-presets'])
  expect(providersQuery.queryKey).toEqual(['aylul-providers'])
  expect(agentsQuery.queryKey).toEqual(['aylul-agents'])
  expect(agentThreadsQuery('ay').queryKey).toEqual(['aylul-agent', 'ay', 'threads'])
  expect(agentEnvQuery('ay').queryKey).toEqual(['aylul-agent', 'ay', 'env'])
  expect(profileKeys.threads('ay')).toEqual(agentThreadsQuery('ay').queryKey)
})
