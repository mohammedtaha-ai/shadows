import { afterEach, expect, it, vi } from 'vitest'
import { profileKeys, providerPresetsQuery, providersQuery, agentsQuery, agentThreadsQuery, agentEnvQuery } from './aylul-profiles'
import { agentTemplateQuery, saveAgent } from './aylul-profiles'
afterEach(() => vi.unstubAllGlobals())
it('the instructions template uses the generated GET route', async () => {
  const fetcher = vi.fn(async (request: Request) => {
    expect(request.method).toBe('GET')
    expect(new URL(request.url).pathname).toBe('/api/aylul/agent-template')
    return Response.json({ instructions: 'Template' })
  })
  vi.stubGlobal('fetch', fetcher)
  expect(await agentTemplateQuery.queryFn!({} as never)).toEqual({ instructions: 'Template' })
  expect(fetcher).toHaveBeenCalledTimes(1)
})
it('agent updates preserve absent instructions and send explicit edited text', async () => {
  const bodies: unknown[] = []
  vi.stubGlobal('fetch', async (request: Request) => {
    expect(request.method).toBe('PUT')
    bodies.push(await request.json())
    return Response.json({ slug: 'work' })
  })
  const input = { name: 'Work', provider: 'p', default_model: 'm', outfit: 'moon', description: 'Description' }
  await saveAgent('work', 'cmd1', input)
  await saveAgent('work', 'cmd2', { ...input, instructions: '' })
  expect(bodies[0]).toEqual({ command_id: 'cmd1', ...input })
  expect(bodies[1]).toEqual({ command_id: 'cmd2', ...input, instructions: '' })
})
it('profile queries and refresh factories retain their exact identities', () => {
  expect(providerPresetsQuery.queryKey).toEqual(['aylul-provider-presets'])
  expect(providersQuery.queryKey).toEqual(['aylul-providers'])
  expect(agentsQuery.queryKey).toEqual(['aylul-agents'])
  expect(agentThreadsQuery('ay').queryKey).toEqual(['aylul-agent', 'ay', 'threads'])
  expect(agentEnvQuery('ay').queryKey).toEqual(['aylul-agent', 'ay', 'env'])
  expect(profileKeys.threads('ay')).toEqual(agentThreadsQuery('ay').queryKey)
})
