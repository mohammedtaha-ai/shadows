// @vitest-environment happy-dom
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { act } from 'react'
import { createRoot, type Root } from 'react-dom/client'
import { afterEach, expect, it, vi } from 'vitest'
import { typeInto, until } from '@/test/test-app'
import { McpForm } from './mcp-form'
import type { McpItem } from '@/api/extensions'

let root: Root | undefined
afterEach(() => { act(() => root?.unmount()); document.body.replaceChildren(); vi.unstubAllGlobals() })
const field = (name: string) => document.querySelector<HTMLInputElement>(`[aria-label="${name}"]`)!
const button = (name: string) => [...document.querySelectorAll<HTMLButtonElement>('button')].find(b => b.textContent === name)!
function render(item: McpItem | null = null) {
  const done = vi.fn()
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  const container = document.createElement('div'); document.body.append(container); root = createRoot(container)
  act(() => root!.render(<QueryClientProvider client={client}><McpForm item={item} onDone={done} /></QueryClientProvider>))
  return done
}
it.each(['http', 'stdio'] as const)('adds %s items with password values and clears them on success', async type => {
  let body: Record<string, unknown> | undefined
  vi.stubGlobal('fetch', async (request: Request) => { body = await request.json(); return Response.json({ id: 'm1' }, { status: 201 }) })
  const done = render()
  typeInto(field('MCP name'), 'tools')
  if (type === 'stdio') {
    const select = document.querySelector<HTMLSelectElement>('[aria-label="MCP transport"]')!
    act(() => { select.value = 'stdio'; select.dispatchEvent(new Event('change', { bubbles: true })) })
    typeInto(field('MCP command'), 'node')
    act(() => button('Add argument').click()); typeInto(field('Argument 1'), 'a spaced argument')
    expect(document.body.textContent).toContain("Runs on this machine's daemon only")
  } else typeInto(field('MCP URL'), 'https://example.com/mcp')
  act(() => button(type === 'http' ? 'Add header' : 'Add environment variable').click())
  typeInto(field('Secret name 1'), type === 'http' ? 'Authorization' : 'TOKEN')
  expect(field('Secret value 1').type).toBe('password')
  typeInto(field('Secret value 1'), 'synthetic-secret')
  act(() => button('Add MCP tool').click())
  await until(() => done.mock.calls.length === 1)
  expect(JSON.stringify(body)).toContain('synthetic-secret')
  expect(body?.transport).toMatchObject(type === 'http' ? { type, headers: [{ name: 'Authorization', value: 'synthetic-secret' }] } : { type, command: 'node', args: ['a spaced argument'], env: [{ name: 'TOKEN', value: 'synthetic-secret' }] })
  expect(document.querySelector('[aria-label="Secret value 1"]')).toBeNull()
  expect(document.body.textContent).not.toContain('synthetic-secret')
})
it('editing shows Set metadata and retains the id and absent secret values', async () => {
  let requestBody: unknown
  vi.stubGlobal('fetch', async (request: Request) => { expect(new URL(request.url).pathname).toBe('/api/aylul/extensions/mcp/m1'); requestBody = await request.json(); return Response.json({ id: 'm1' }) })
  const done = render({ id: 'm1', name: 'tools', description: 'Tools', transport: { type: 'http', url: 'https://example.com', headers: [{ name: 'Authorization', set: true }] } })
  expect(document.body.textContent).toContain('Set')
  expect(field('Secret value 1').value).toBe('')
  act(() => button('Save MCP tool').click()); await until(() => done.mock.calls.length === 1)
  expect(requestBody).toMatchObject({ name: 'tools', transport: { headers: [{ name: 'Authorization' }] } })
  expect(JSON.stringify(requestBody)).not.toContain('"value"')
})
it.each([400, 409, 503])('Core %s errors preserve the form and retry command identity', async status => {
  const bodies: unknown[] = []
  vi.stubGlobal('fetch', async (request: Request) => { bodies.push(await request.json()); return Response.json({ code: status === 503 ? 'SECRETS_UNAVAILABLE' : status === 409 ? 'EXTENSION_NAME_CONFLICT' : 'PROFILE_INVALID', message: `Core refused ${status}` }, { status }) })
  render(); typeInto(field('MCP name'), 'tools')
  act(() => button('Add MCP tool').click()); await until(() => document.body.textContent!.includes(`Core refused ${status}`) && !button('Add MCP tool').disabled)
  act(() => button('Add MCP tool').click()); await until(() => bodies.length === 2)
  expect(bodies[1]).toEqual(bodies[0]); expect(field('MCP name').value).toBe('tools')
})
