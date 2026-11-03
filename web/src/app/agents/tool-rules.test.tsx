// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { answers } from '@/test/fake-daemon'
import { startApp, until, type TestApp, type Answer } from '@/test/test-app'

let app: TestApp | null = null
afterEach(() => { app?.unmount(); app = null; window.localStorage.clear() })
const toolsPath = '/api/aylul/agents/ay/extensions/mcp/m1/tools'
const rulesPath = '/api/aylul/agents/ay/extensions/mcp/m1/tool-rules'
const toolList = [{ name: 'read', description: 'Read a file' }, { name: 'exec', description: 'Run a command' }]
const stored = [{ tool: 'read', rule: 'allow' }]

async function open(view = { tools: toolList, rules: stored, list_error: null as string | null }, overrides: Record<string, Answer> = {}) {
  app = await startApp('/?settings=agents&agent=ay', {
    ...answers(),
    'GET /api/aylul/providers': [{ slug: 'p', name: 'Provider', base_url: 'https://example.com', models: [{ id: 'm1', context_window: 200000 }], key_set: false, created_at: 'now', updated_at: 'now' }],
    'GET /api/aylul/agents': { project_id: 'p1', agents: [{ slug: 'ay', name: 'AYLUL', provider: 'p', default_model: 'm1', deleted: false, created_at: 'now', updated_at: 'now' }] },
    'GET /api/aylul/agents/ay/env': [],
    'GET /api/aylul/agents/ay/extensions': { plugins: [], skills: [], mcp: [{ id: 'm1', name: 'docs', description: 'Docs tools', enabled: true, transport: { type: 'http', url: 'https://example.com/mcp', headers: [] } }] },
    [`GET ${toolsPath}`]: view,
    [`PUT ${rulesPath}`]: async (request: Request) => Response.json((await request.json()).rules.filter((r: { rule: string }) => r.rule !== 'ask')),
    ...overrides,
  })
  await until(() => app!.text().includes('Docs tools'))
  return app
}

it('lists tool descriptions and the current permission', async () => {
  const a = await open()
  await until(() => a.button('Allow read') !== undefined)
  expect(a.text()).toContain('Read a file')
  expect(a.text()).toContain('Run a command')
  expect(a.button('Allow read')!.getAttribute('aria-pressed')).toBe('true')
  expect(a.button('Ask before exec')!.getAttribute('aria-pressed')).toBe('true')
  expect(a.button('Block exec')!.getAttribute('aria-pressed')).toBe('false')
})

it('blocking a tool sends its rule and preserves other rules', async () => {
  const a = await open()
  await until(() => a.button('Block exec') !== undefined)
  act(() => a.button('Block exec')!.click())
  await until(() => a.bodies.length === 1)
  expect(a.bodies[0]).toEqual({ command_id: expect.any(String), rules: [{ tool: 'read', rule: 'allow' }, { tool: 'exec', rule: 'deny' }] })
  await until(() => a.button('Block exec')!.getAttribute('aria-pressed') === 'true')
})

it('set all applies to every listed tool and keeps missing rules', async () => {
  const a = await open({ tools: toolList, rules: [{ tool: 'missing', rule: 'deny' }], list_error: null })
  await until(() => a.button('Allow all tools') !== undefined)
  act(() => a.button('Allow all tools')!.click())
  await until(() => a.bodies.length === 1)
  expect(a.bodies[0]).toEqual({ command_id: expect.any(String), rules: [{ tool: 'missing', rule: 'deny' }, ...toolList.map(t => ({ tool: t.name, rule: 'allow' }))] })
})

it('a missing tool stays visible and can be cleared', async () => {
  const a = await open({ tools: toolList, rules: [...stored, { tool: 'missing', rule: 'deny' }], list_error: null })
  await until(() => a.button('Clear missing') !== undefined)
  expect(a.text()).toContain('No longer offered by the server')
  act(() => a.button('Clear missing')!.click())
  await until(() => a.bodies.length === 1)
  expect(a.bodies[0]).toEqual({ command_id: expect.any(String), rules: stored })
  await until(() => a.button('Clear missing') === undefined)
})

it('listing failure still allows stored rules to be cleared', async () => {
  const a = await open({ tools: [], rules: stored, list_error: 'the server refused the connection' })
  await until(() => a.button('Clear read') !== undefined)
  expect(a.text()).toContain('the server refused the connection')
  act(() => a.button('Clear read')!.click())
  await until(() => a.bodies.length === 1)
  expect(a.bodies[0]).toEqual({ command_id: expect.any(String), rules: [] })
})

it('a click during a save is queued and built on the saved answer', async () => {
  let release!: (response: Response) => void
  let writes = 0
  const a = await open(undefined, { [`PUT ${rulesPath}`]: async (request: Request) => {
    const body = await request.json()
    if (++writes === 1) return new Promise<Response>(resolve => { release = resolve })
    return Response.json(body.rules)
  } })
  await until(() => a.button('Block exec') !== undefined)
  act(() => { a.button('Block exec')!.click(); a.button('Block read')!.click() })
  await until(() => writes === 1)
  expect(a.button('Block read')!.disabled).toBe(true)
  expect(a.button('Allow all tools')!.disabled).toBe(true)
  await act(async () => release(Response.json([...stored, { tool: 'exec', rule: 'deny' }])))
  await until(() => writes === 2)
  expect(a.bodies[1]).toEqual({ command_id: expect.any(String), rules: [{ tool: 'exec', rule: 'deny' }, { tool: 'read', rule: 'deny' }] })
  await until(() => a.button('Block read')!.getAttribute('aria-pressed') === 'true')
})

it('a failed save keeps the selection and retries the same command', async () => {
  const a = await open(undefined, { [`PUT ${rulesPath}`]: () => Response.json({ code: 'STORAGE_UNAVAILABLE', message: 'Try again' }, { status: 503 }) })
  await until(() => a.button('Block exec') !== undefined)
  act(() => a.button('Block exec')!.click())
  await until(() => a.text().includes('Try again'))
  expect(a.button('Ask before exec')!.getAttribute('aria-pressed')).toBe('true')
  act(() => a.button('Block exec')!.click())
  await until(() => a.bodies.length === 2)
  expect(a.bodies[0]).toEqual(a.bodies[1])
})
