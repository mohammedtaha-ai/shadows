// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { answers } from '@/test/fake-daemon'
import { startApp, typeInto, until, type TestApp } from '@/test/test-app'

let app: TestApp | null = null
afterEach(() => { app?.unmount(); app = null })

const gopls = { id: 'gopls', added: false, wanted: true, extensions: ['.go'], state: { state: 'running' }, install: null }
const rust = { id: 'rust-analyzer', added: false, wanted: false, extensions: ['.rs'], state: { state: 'not_installed' }, install: 'rustup component add rust-analyzer' }
const mine = { id: 'mine', added: true, wanted: false, extensions: ['.zz'], state: { state: 'idle' }, install: null }
const field = (label: string) => [...document.querySelectorAll('label')].find(l => l.textContent?.startsWith(label))!.querySelector('input')!

it('lists_the_servers_with_their_state', async () => {
  app = await startApp('/projects/p1/new?settings=code', { ...answers(), 'GET /api/projects/p1/code/servers': [gopls, rust] })
  await until(() => app!.text().includes('rust-analyzer'))
  const text = app.text()
  expect(text).toContain('gopls'); expect(text).toContain('.go'); expect(text).toContain('Running')
  expect(text).toContain('Not installed'); expect(text).toContain('rustup component add rust-analyzer')
  expect(app.button('Copy')).toBeDefined()
})

it('switching_off_warns_first', async () => {
  app = await startApp('/projects/p1/new?settings=code', { ...answers(),
    'GET /api/projects/p1/code/servers': [gopls],
    'PUT /api/code/servers/gopls/off': () => new Response(null, { status: 204 }) })
  await until(() => app!.button('gopls on') !== undefined)
  act(() => app!.button('gopls on')!.click())
  expect(app.text()).toContain("Shadows' answers about this language get worse while it is off.")
  expect(app.calls).not.toContain('PUT /api/code/servers/gopls/off')
  act(() => app!.button('Cancel')!.click())
  expect(app.text()).not.toContain('get worse')
  act(() => app!.button('gopls on')!.click())
  act(() => app!.button('Switch off')!.click())
  await until(() => app!.calls.includes('PUT /api/code/servers/gopls/off'))
  expect(app.bodies.at(-1)).toEqual({ command_id: expect.any(String), off: true })
})

it('adds_and_removes_a_server', async () => {
  app = await startApp('/projects/p1/new?settings=code', { ...answers(),
    'GET /api/projects/p1/code/servers': [mine],
    'POST /api/code/servers': () => Response.json({ ...mine, id: 'zls', extensions: ['.zig'] }),
    'DELETE /api/code/servers/mine': () => new Response(null, { status: 204 }) })
  await until(() => app!.button('Remove') !== undefined)
  expect(app.text()).toContain('A server you add runs on your machine on your responsibility.')
  typeInto(field('Server id'), 'zls'); typeInto(field('Command'), 'zls'); typeInto(field('Arguments'), '--stdio')
  typeInto(field('Extensions'), 'zig')
  expect(app.button('Add server')!.disabled).toBe(true)
  typeInto(field('Extensions'), '.zig, .zon')
  act(() => app!.button('Add server')!.click())
  await until(() => app!.calls.includes('POST /api/code/servers'))
  expect(app.bodies.at(-1)).toEqual({ id: 'zls', command: 'zls', args: ['--stdio'], extensions: ['.zig', '.zon'], command_id: expect.any(String) })
  act(() => app!.button('Remove')!.click())
  await until(() => app!.calls.some(c => c.startsWith("DELETE /api/code/servers/mine")))
})
