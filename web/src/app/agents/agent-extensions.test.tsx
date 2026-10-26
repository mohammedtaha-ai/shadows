// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { agentExtensionsQuery, type AgentExtensions } from '@/api/extensions'
import { answers } from '@/test/fake-daemon'
import { startApp, until, typeInto, type TestApp, type Answer } from '../test-app'

const plugin = { id: '11111111-1111-4111-8111-111111111111', name: 'git', description: 'Git tools.' }
const skill = { id: '22222222-2222-4222-8222-222222222222', name: 'review', description: 'Review changes.' }
const view = (enabled = false): AgentExtensions => ({ plugins: [{ ...plugin, enabled: true }], skills: [{ ...skill, enabled }] })
const agent = { slug: 'ay', name: 'AYLUL', provider: 'p', default_model: 'm1', character_color: 'purple', deleted: false, created_at: 'now', updated_at: 'now' }
let app: TestApp | null = null
afterEach(() => { app?.unmount(); app = null; window.localStorage.clear() })
const switchOf = (name: string) => document.querySelector<HTMLInputElement>(`input[role="switch"][aria-label="${name}"]`)!
function routes(): Record<string, Answer> {
  return { ...answers(), 'GET /api/aylul/providers': [{ slug: 'p', name: 'Provider', base_url: 'https://example.com', models: [{ id: 'm1', context_window: 200000 }], key_set: false, created_at: 'now', updated_at: 'now' }],
    'GET /api/aylul/agents': { project_id: 'p1', agents: [agent, { ...agent, slug: 'zed', name: 'Zed' }] },
    'GET /api/aylul/agents/ay/env': [], 'GET /api/aylul/agents/zed/env': [],
    'GET /api/aylul/agents/ay/extensions': view(), 'GET /api/aylul/agents/zed/extensions': view(),
  }
}
async function open(overrides: Record<string, Answer> = {}) {
  const a = app = await startApp('/?settings=agents&agent=ay', { ...routes(), ...overrides })
  await until(() => switchOf('review') !== null)
  return a
}
it("the form lists the library's items for this agent", async () => {
  const a = await open()
  expect(switchOf('git').checked).toBe(true); expect(switchOf('review').checked).toBe(false)
  expect(a.text()).toContain('Git tools.'); expect(a.text()).toContain('Review changes.')
  expect(agentExtensionsQuery('ay').queryKey).not.toEqual(agentExtensionsQuery('zed').queryKey)
})
it('a switch saves at once', async () => {
  let resolve!: (response: Response) => void, enabled = false
  const a = await open({ 'GET /api/aylul/agents/ay/extensions': () => Response.json(view(enabled)),
    [`PUT /api/aylul/agents/ay/extensions/skill/${skill.id}`]: () => new Promise<Response>(done => { resolve = done }),
  })
  act(() => switchOf('review').click()); await until(() => a.bodies.length === 1)
  expect(switchOf('review').checked).toBe(true); expect(switchOf('review').disabled).toBe(true)
  expect(a.bodies[0]).toEqual({ enabled: true, command_id: expect.any(String) })
  expect(a.calls).not.toContain('PUT /api/aylul/agents/ay')
  enabled = true; await act(async () => resolve(Response.json({ ...skill, enabled: true })))
  await until(() => switchOf('review').disabled === false)
  expect(switchOf('review').checked).toBe(true)
})
it('a failed switch goes back', async () => {
  let count = 0, enabled = false
  const a = await open({ 'GET /api/aylul/agents/ay/extensions': () => Response.json(view(enabled)),
    [`PUT /api/aylul/agents/ay/extensions/skill/${skill.id}`]: async (request: Request) => {
      const input = await request.json(); if (++count === 1) return Response.json({ code: 'STORAGE_UNAVAILABLE', message: 'Toggle failed.' }, { status: 500 })
      enabled = input.enabled; return Response.json({ ...skill, enabled })
    },
  })
  act(() => switchOf('review').click()); await until(() => a.text().includes('Toggle failed.') && !switchOf('review').disabled)
  expect(switchOf('review').checked).toBe(false)
  expect(document.querySelector<HTMLInputElement>('input[aria-label="Agent name"]')?.disabled).toBe(false)
  act(() => switchOf('review').click()); await until(() => count === 2 && !switchOf('review').disabled)
  expect(a.bodies[1]).toEqual(a.bodies[0])
  act(() => switchOf('review').click()); await until(() => count === 3 && !switchOf('review').disabled)
  expect((a.bodies[2] as { command_id: string }).command_id).not.toBe((a.bodies[0] as { command_id: string }).command_id)
})
it('late toggles remain scoped', async () => {
  let resolve!: (response: Response) => void
  const a = await open({ [`PUT /api/aylul/agents/ay/extensions/skill/${skill.id}`]: () => new Promise<Response>(done => { resolve = done }) })
  act(() => switchOf('review').click()); await until(() => a.bodies.length === 1)
  act(() => a.button('Back')!.click()); await until(() => a.buttons('Edit').length === 2)
  act(() => a.buttons('Edit')[1].click()); await until(() => switchOf('review') !== null)
  const before = a.queryClient.getQueryData(agentExtensionsQuery('zed').queryKey)
  await act(async () => resolve(Response.json({ ...skill, enabled: true })))
  expect(switchOf('review').checked).toBe(false)
  expect(a.queryClient.getQueryData(agentExtensionsQuery('zed').queryKey)).toEqual(before)
})
it('a new agent shows the part after it is created', async () => {
  let agents = [agent]
  const a = app = await startApp('/?settings=agents&agent=new', { ...routes(),
    'GET /api/aylul/agents': () => Response.json({ project_id: 'p1', agents }),
    'POST /api/aylul/agents': async (request: Request) => { const created = { ...agent, ...await request.json(), slug: 'new-one' }; agents = [...agents, created]; return Response.json(created, { status: 201 }) },
    'GET /api/aylul/agents/new-one/env': [], 'GET /api/aylul/agents/new-one/extensions': { plugins: [{ ...plugin, enabled: false }], skills: [{ ...skill, enabled: false }] },
  })
  await until(() => document.querySelector('input[aria-label="Agent name"]') !== null)
  expect(switchOf('review')).toBeNull(); expect(a.calls.some(call => call.includes('/extensions'))).toBe(false)
  typeInto(document.querySelector<HTMLInputElement>('input[aria-label="Agent name"]')!, 'New one'); act(() => a.button('Create agent')!.click())
  await until(() => a.buttons('Edit').length === 2); act(() => a.buttons('Edit')[1].click())
  await until(() => switchOf('review') !== null)
  expect(switchOf('git').checked).toBe(false); expect(switchOf('review').checked).toBe(false)
})
it('an empty library links to the section', async () => {
  const a = app = await startApp('/?settings=agents&agent=ay', { ...routes(), 'GET /api/aylul/agents/ay/extensions': { plugins: [], skills: [] } })
  await until(() => a.text().includes('No plugins or skills yet.'))
  act(() => a.button('Open Plugins & Skills')!.click())
  await until(() => document.querySelector('[data-slot="dialog-title"]')?.textContent === 'Plugins & Skills')
  expect(document.querySelectorAll('[role="dialog"]')).toHaveLength(1); expect(a.path()).toBe('/')
})
it('an extension load failure is local', async () => {
  const a = app = await startApp('/?settings=agents&agent=ay', { ...routes(), 'GET /api/aylul/agents/ay/extensions': () => Response.json({ code: 'STORAGE_UNAVAILABLE', message: 'Extensions unavailable.' }, { status: 500 }) })
  await until(() => a.text().includes('Extensions unavailable.'))
  expect(a.text()).not.toContain('No plugins or skills yet.')
  expect(a.button('Save agent')?.disabled).toBe(false)
})
it('library removal clears every agent view', async () => {
  let removed = false
  const current = () => removed ? { plugins: [], skills: [] } : view()
  const a = await open({ 'GET /api/aylul/agents/ay/extensions': () => Response.json(current()), 'GET /api/aylul/agents/zed/extensions': () => Response.json(current()),
    'GET /api/aylul/extensions': () => Response.json({ plugins: removed ? [] : [plugin], skills: removed ? [] : [skill] }),
    [`DELETE /api/aylul/extensions/plugin/${plugin.id}`]: () => { removed = true; return new Response(null, { status: 204 }) },
  })
  a.queryClient.setQueryData(agentExtensionsQuery('zed').queryKey, view())
  act(() => a.button('Plugins & Skills')!.click()); await until(() => a.button('Remove') !== undefined)
  act(() => a.button('Remove')!.click()); act(() => a.buttons('Remove').at(-1)!.click())
  await until(() => a.text().includes('No plugins yet.'))
  act(() => a.button('Agents')!.click()); await until(() => a.buttons('Edit').length === 2)
  act(() => a.buttons('Edit')[1].click()); await until(() => a.text().includes('No plugins or skills yet.'))
  expect(switchOf('git')).toBeNull()
  act(() => a.button('Back')!.click()); await until(() => a.buttons('Edit').length === 2)
  act(() => a.buttons('Edit')[0].click()); await until(() => a.text().includes('No plugins or skills yet.'))
  expect(switchOf('git')).toBeNull()
})

it('a stale inactive extension read cannot overwrite a saved switch', async () => {
  let releaseRead!: (value: Response) => void
  let releaseSave!: (value: Response) => void
  let reads = 0
  const a = await open({
    'GET /api/aylul/agents/ay/extensions': () => ++reads === 1 ? Response.json(view())
      : new Promise<Response>(resolve => { releaseRead = resolve }),
    [`PUT /api/aylul/agents/ay/extensions/skill/${skill.id}`]: () => new Promise<Response>(resolve => { releaseSave = resolve }),
  })
  act(() => switchOf('review').click())
  await until(() => a.bodies.length === 1)
  let pending!: Promise<unknown>
  act(() => { pending = a.queryClient.refetchQueries({ queryKey: agentExtensionsQuery('ay').queryKey, exact: true }) })
  await until(() => reads === 2)
  act(() => a.button('Back')!.click())
  await until(() => a.buttons('Edit').length === 2)
  await act(async () => releaseSave(Response.json({ ...skill, enabled: true })))
  await until(() => a.queryClient.getQueryData<AgentExtensions>(agentExtensionsQuery('ay').queryKey)?.skills[0].enabled === true)
  await act(async () => { releaseRead(Response.json(view())); await pending })
  expect(a.queryClient.getQueryData(agentExtensionsQuery('ay').queryKey)).toEqual(view(true))
})
