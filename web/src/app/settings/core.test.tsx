// @vitest-environment happy-dom
import { act } from 'react'
import { createRoot, type Root } from 'react-dom/client'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { CoreSection } from './sections/core'

const { invoke, listen } = vi.hoisted(() => ({ invoke: vi.fn(), listen: vi.fn() }))
vi.mock('@tauri-apps/api/core', () => ({ invoke }))
vi.mock('@tauri-apps/api/event', () => ({ listen }))
let host: HTMLDivElement, root: Root
const view = { revision: 1, state: 'ready', configurationId: 'local', origin: 'http://127.0.0.1:4318', error: null }
beforeEach(() => {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true })
  Object.assign(window, { __TAURI_INTERNALS__: {} })
  host = document.createElement('div'); document.body.append(host); root = createRoot(host)
  listen.mockReset().mockResolvedValue(() => {})
  invoke.mockReset().mockImplementation(async (command: string) => command === 'desktop_connection' ? view : {
    configurationId: 'local', executable: 'shadows.exe', db: 'existing.sqlite', node: 'node.exe', adapter: 'adapter.js', harness: 'claude.exe', aylulConfig: 'aylul.json', origin: view.origin,
  })
})
afterEach(async () => {
  await act(async () => root.unmount()); host.remove()
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__
})
async function render() { await act(async () => { root.render(<CoreSection />); await new Promise(resolve => setTimeout(resolve, 10)) }) }
it('the Local Core paths are under a closed Advanced', async () => {
  await render()
  expect(host.textContent).toContain(view.origin)
  expect(host.querySelector('[role="status"]')?.textContent).toContain('Running')
  const advanced = host.querySelector('details')!
  expect(advanced).not.toBeNull()
  expect(advanced.open).toBe(false)
  const executable = [...host.querySelectorAll('label')].find(label => label.textContent?.includes('Shadows executable'))!
  expect(executable.closest('details')).toBe(advanced)
  await act(async () => host.querySelector('summary')!.click())
  expect(advanced.open).toBe(true)
  expect(executable.querySelector('input')?.value).toBe('shadows.exe')
})
it('the Core state follows newer native events and ignores older snapshots', async () => {
  listen.mockImplementation(async (_name: string, receive: (event: unknown) => void) => {
    receive({ payload: { ...view, revision: 3, state: 'failed' } }); return () => {}
  })
  await render()
  expect(host.querySelector('[role="status"]')?.textContent).toContain('Not running')
})
