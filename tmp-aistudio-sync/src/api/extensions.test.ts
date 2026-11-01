import { afterEach, expect, it, vi } from 'vitest'
import { saveMcp, removeExtension } from './extensions'
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
