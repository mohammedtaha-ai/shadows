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
const open = async () => {
  await until(() => app!.button('Language servers') !== undefined)
  act(() => app!.button('Language servers')!.click())
}

it('starts_collapsed_with_the_running_count', async () => {
  app = await startApp('/projects/p1/new?settings=code', { ...answers(), 'GET /api/projects/p1/code/servers': [gopls, rust] })
  await until(() => app!.button('Language servers') !== undefined)
  expect(app.button('Language servers')!.getAttribute('aria-expanded')).toBe('false')
  expect(app.text()).toContain('1 running')
  expect(app.text()).not.toContain('Go')
})

it('opens_to_the_wanted_servers_by_language_and_reveals_the_rest', async () => {
  app = await startApp('/projects/p1/new?settings=code', { ...answers(), 'GET /api/projects/p1/code/servers': [gopls, rust] })
  await open()
  expect(app.button('Language servers')!.getAttribute('aria-expanded')).toBe('true')
  const text = app.text()
  expect(text).toContain('Go'); expect(text).toContain('Running')
  expect(text).not.toContain('gopls'); expect(text).not.toContain('Rust')
  expect(text).not.toContain('rustup'); expect(text).not.toContain('.go')
  expect(app.button('Copy')).toBeUndefined()
  act(() => app!.button('1 more language')!.click())
  expect(app.text()).toContain('Rust'); expect(app.text()).toContain('Not installed')
  expect(app.text()).not.toContain('rustup component add')
})

it('switching_off_warns_first', async () => {
  app = await startApp('/projects/p1/new?settings=code', { ...answers(),
    'GET /api/projects/p1/code/servers': [gopls],
    'PUT /api/code/servers/gopls/off': () => new Response(null, { status: 204 }) })
  await open()
  act(() => app!.button('Go on')!.click())
  expect(app.text()).toContain("Shadows' answers about this language get worse while it is off.")
  expect(app.calls).not.toContain('PUT /api/code/servers/gopls/off')
  act(() => app!.button('Cancel')!.click())
  expect(app.text()).not.toContain('get worse')
  act(() => app!.button('Go on')!.click())
  act(() => app!.button('Switch off')!.click())
  await until(() => app!.calls.includes('PUT /api/code/servers/gopls/off'))
  expect(app.bodies.at(-1)).toEqual({ command_id: expect.any(String), off: true })
})

it('adds_and_removes_a_server', async () => {
  let posts = 0
  app = await startApp('/projects/p1/new?settings=code', { ...answers(),
    'GET /api/projects/p1/code/servers': [mine],
    'POST /api/code/servers': () => ++posts === 1
      ? Response.json({ code: 'INVALID_COMMAND', message: 'each extension must start with a dot, as .go' }, { status: 422 })
      : Response.json({ ...mine, id: 'zls', extensions: ['.zig'] }),
    'DELETE /api/code/servers/mine': () => new Response(null, { status: 204 }) })
  await open()
  act(() => app!.button('1 more language')!.click())
  expect(app.text()).not.toContain('A server you add runs')
  act(() => app!.button('Add a server')!.click())
  expect(app.text()).toContain('A server you add runs on your machine on your responsibility.')
  typeInto(field('Server id'), 'zls'); typeInto(field('Command'), 'zls'); typeInto(field('Arguments'), '--stdio')
  // Core checks the extensions and the form shows its refusal.
  typeInto(field('Extensions'), 'zig')
  act(() => app!.button('Add server')!.click())
  await until(() => app!.text().includes('each extension must start with a dot, as .go'))
  typeInto(field('Extensions'), '.zig, .zon')
  act(() => app!.button('Add server')!.click())
  await until(() => posts === 2)
  expect(app.bodies.at(-1)).toEqual({ id: 'zls', command: 'zls', args: ['--stdio'], extensions: ['.zig', '.zon'], command_id: expect.any(String) })
  act(() => app!.button('Remove')!.click())
  await until(() => app!.calls.some(c => c.startsWith("DELETE /api/code/servers/mine")))
})
