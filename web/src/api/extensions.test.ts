import { QueryClient } from '@tanstack/react-query'
import { afterEach, expect, it, vi } from 'vitest'
import { extensionKeys, invalidateLibraryViews, projectExtensionsQuery, removeExtension, saveMcp, saveProjectExtensionAnswer, setProjectExtension, type AgentExtensions } from './extensions'
afterEach(() => vi.unstubAllGlobals())
it('MCP creation and editing use generated routes and preserve absent secret values', async () => {
  const calls: { method: string; path: string; body: unknown }[] = []
  vi.stubGlobal('fetch', async (request: Request) => {
    calls.push({ method: request.method, path: new URL(request.url).pathname, body: await request.json() })
    return Response.json({ id: 'm1', name: 'tools', description: '', transport: { type: 'http', url: 'https://example.com', headers: [] } })
  })
  await saveMcp(null, 'create', { name: 'tools', transport: { type: 'http', url: 'https://example.com', headers: [{ name: 'Authorization', value: 'Synthetic bearer' }] } })
  await saveMcp('m1', 'edit', { name: 'tools', transport: { type: 'http', url: 'https://example.com/new', headers: [{ name: 'Authorization' }] } })
  expect(calls[0]).toMatchObject({ method: 'POST', path: '/api/aylul/extensions/mcp', body: { command_id: 'create' } })
  expect(calls[1]).toEqual({ method: 'PUT', path: '/api/aylul/extensions/mcp/m1', body: { command_id: 'edit', name: 'tools', transport: { type: 'http', url: 'https://example.com/new', headers: [{ name: 'Authorization' }] } } })
})
it('MCP deletion uses the existing kind route', async () => {
  vi.stubGlobal('fetch', async (request: Request) => {
    expect(request.method).toBe('DELETE')
    expect(new URL(request.url).pathname).toBe('/api/aylul/extensions/mcp/m1')
    expect(new URL(request.url).searchParams.get('command_id')).toBe('remove')
    return new Response(null, { status: 204 })
  })
  await removeExtension('remove', 'mcp', 'm1')
})
it("project switches use the project routes with the caller's command id", async () => {
  const calls: { method: string; path: string; body: unknown }[] = []
  vi.stubGlobal('fetch', async (request: Request) => {
    const text = await request.text()
    calls.push({ method: request.method, path: new URL(request.url).pathname, body: text === '' ? null : JSON.parse(text) })
    return Response.json(request.method === 'GET' ? { plugins: [], skills: [], mcp: [] } : { id: 'm1', name: 'tools', description: '', enabled: true })
  })
  await new QueryClient().fetchQuery(projectExtensionsQuery('p1'))
  await setProjectExtension('p1', 'mcp', 'm1', true, 'cmd')
  expect(calls[0]).toMatchObject({ method: 'GET', path: '/api/projects/p1/planner-extensions' })
  expect(calls[1]).toEqual({ method: 'PUT', path: '/api/projects/p1/planner-extensions/mcp/m1', body: { command_id: 'cmd', enabled: true } })
})
const item = (id: string, enabled: boolean) => ({ id, name: id, description: '', enabled }) as AgentExtensions['skills'][number]
const view = (): AgentExtensions => ({ plugins: [], skills: [item('s1', false), item('s2', false)], mcp: [] })
it("a saved project switch replaces only its item in its project's view", async () => {
  const client = new QueryClient()
  client.setQueryData(extensionKeys.project('p1'), view())
  client.setQueryData(extensionKeys.project('p2'), view())
  await saveProjectExtensionAnswer(client, 'p1', 'skill', item('s1', true))
  const p1 = client.getQueryData<AgentExtensions>(extensionKeys.project('p1'))
  expect(p1?.skills).toEqual([item('s1', true), item('s2', false)])
  expect(client.getQueryData(extensionKeys.project('p2'))).toEqual(view())
})
it('invalidateLibraryViews refreshes the library, every agent and every project', async () => {
  const client = new QueryClient()
  const keys = [extensionKeys.library(), extensionKeys.agent('a'), extensionKeys.project('p1')]
  for (const key of keys) client.setQueryData(key, view())
  await invalidateLibraryViews(client)
  for (const key of keys) expect(client.getQueryState(key)?.isInvalidated).toBe(true)
})
