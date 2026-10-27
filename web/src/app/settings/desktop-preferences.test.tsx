// @vitest-environment happy-dom
import { act, StrictMode } from 'react'
import { createRoot, type Root } from 'react-dom/client'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { personQuery } from '@/api/person'
import { codeKeys } from '@/api/code'
import { GeneralSection } from './sections/general'
import { DesktopPreferencesControls } from './desktop-preferences'
import { until, typeInto } from '@/test/test-app'

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }))
vi.mock('@tauri-apps/api/core', () => ({ invoke }))
const defaults = { launchAtLogin: false, closeToTray: true }
let host: HTMLDivElement, root: Root, client: QueryClient, coreFail: boolean
const switchOf = (name: string) => host.querySelector<HTMLInputElement>(`input[role="switch"][aria-label="${name}"]`)!
const launch = () => switchOf('Start AYLUL with Windows')
const tray = () => switchOf('Closing the window hides it to the tray')
const button = (name: string) => [...host.querySelectorAll<HTMLButtonElement>('button')].find(item => item.textContent === name)!
beforeEach(() => {
  Object.assign(window, { __TAURI_INTERNALS__: {} }); coreFail = false
  host = document.createElement('div'); document.body.append(host); root = createRoot(host)
  client = new QueryClient({ defaultOptions: { queries: { retry: false, staleTime: Infinity } } })
  client.setQueryData(personQuery.queryKey, { name: 'Name', aylul_calls_me: null, language: null, theme: 'system', updated_at: null })
  client.setQueryData(codeKeys.settings(), { active_limit: 5 })
  invoke.mockReset().mockResolvedValue(defaults)
  vi.stubGlobal('fetch', vi.fn(async (request: Request) => {
    const body = await request.json()
    return coreFail ? Response.json({ code: 'STORAGE_UNAVAILABLE', message: 'Core save failed.' }, { status: 500 }) : Response.json({ ...body, updated_at: null })
  }))
})
afterEach(async () => {
  await act(async () => root.unmount()); client.clear(); host.remove(); vi.unstubAllGlobals()
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__
})
async function render(general = true) {
  await act(async () => root.render(<QueryClientProvider client={client}>{general ? <GeneralSection /> : <DesktopPreferencesControls />}</QueryClientProvider>))
}
it('browser General has no native preferences', async () => {
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__
  await render()
  expect(launch()).toBeNull(); expect(tray()).toBeNull(); expect(invoke).not.toHaveBeenCalled()
  for (const name of ['Language', 'Theme']) expect(host.querySelector(`[aria-label="${name}"]`)).not.toBeNull()
  expect(host.querySelector('[aria-label="Active projects"]')).not.toBeNull()
})
it('desktop defaults come from native without a write', async () => {
  let finish!: (value: typeof defaults) => void
  invoke.mockImplementation(() => new Promise(resolve => { finish = resolve }))
  await render(false)
  expect(launch().disabled).toBe(true); expect(tray().disabled).toBe(true)
  await act(async () => finish(defaults)); await until(() => !launch().disabled)
  expect(launch().checked).toBe(false); expect(tray().checked).toBe(true)
  expect(invoke.mock.calls).toEqual([['desktop_preferences', undefined]])
  await act(async () => root.render(null)); await render(false)
  expect(invoke).toHaveBeenCalledTimes(2)
})
it('a StrictMode read cannot overwrite accepted desktop preferences', async () => {
  let finish!: (value: typeof defaults) => void, reads = 0
  invoke.mockImplementation((command: string) => {
    if (command === 'set_desktop_preferences') return Promise.resolve({ launchAtLogin: true, closeToTray: true })
    return ++reads === 1 ? new Promise(resolve => { finish = resolve }) : Promise.resolve(defaults)
  })
  await act(async () => root.render(<StrictMode><QueryClientProvider client={client}><DesktopPreferencesControls /></QueryClientProvider></StrictMode>))
  await until(() => reads === 2 && !launch().disabled)
  act(() => launch().click()); await until(() => launch().checked && !launch().disabled)
  await act(async () => { finish(defaults); await new Promise(resolve => setTimeout(resolve, 0)) })
  expect(launch().checked).toBe(true)
})
it('each switch sends flat partial camelCase arguments', async () => {
  let finish!: (value: typeof defaults) => void
  invoke.mockImplementation((command: string) => command === 'desktop_preferences' ? Promise.resolve(defaults) : new Promise(resolve => { finish = resolve }))
  await render(); await until(() => !launch().disabled)
  act(() => launch().click())
  expect(launch().disabled).toBe(true); expect(tray().disabled).toBe(true)
  expect(invoke).toHaveBeenLastCalledWith('set_desktop_preferences', { launchAtLogin: true })
  await act(async () => finish({ launchAtLogin: true, closeToTray: true }))
  await until(() => !launch().disabled); expect(launch().checked).toBe(true)
  act(() => tray().click())
  expect(invoke).toHaveBeenLastCalledWith('set_desktop_preferences', { closeToTray: false })
  await act(async () => finish({ launchAtLogin: false, closeToTray: false }))
  await until(() => !tray().disabled)
  expect(launch().checked).toBe(false); expect(tray().checked).toBe(false)
  expect(fetch).not.toHaveBeenCalled()
})
it.each(['desktop_preferences_save_failed', 'desktop_preferences_autostart_failed'])('a failed setter rereads actual native values: %s', async code => {
  let reads = 0
  invoke.mockImplementation(async (command: string) => {
    if (command === 'set_desktop_preferences') throw code
    return ++reads === 1 ? defaults : { launchAtLogin: true, closeToTray: false }
  })
  await render(); await until(() => !launch().disabled)
  act(() => launch().click()); await until(() => reads === 2 && !launch().disabled)
  expect(host.textContent).toContain('Could not change this setting.')
  expect(host.textContent).not.toContain(code)
  expect(launch().checked).toBe(true); expect(tray().checked).toBe(false)
  expect(invoke.mock.calls.filter(([command]) => command === 'set_desktop_preferences')).toHaveLength(1)
})
it('failed recovery read keeps controls disabled until explicit Retry', async () => {
  let reads = 0, recover = false
  invoke.mockImplementation(async (command: string) => {
    if (command === 'set_desktop_preferences') throw 'desktop_preferences_save_failed'
    if (++reads > 1 && !recover) throw 'desktop_preferences_load_failed'
    return recover ? { launchAtLogin: true, closeToTray: false } : defaults
  })
  await render(); await until(() => !launch().disabled)
  act(() => launch().click()); await until(() => host.textContent?.includes('Could not load desktop settings.') === true)
  expect(launch().disabled).toBe(true); expect(tray().disabled).toBe(true)
  expect(launch().checked).toBe(false); expect(tray().checked).toBe(true)
  recover = true; act(() => button('Retry desktop settings').click()); await until(() => !launch().disabled)
  expect(launch().checked).toBe(true); expect(tray().checked).toBe(false)
  expect(invoke.mock.calls.filter(([command]) => command === 'set_desktop_preferences')).toHaveLength(1)
})
it.each(['desktop_preferences_load_failed', 'desktop_preferences_autostart_failed', 'malformed'])('load failure stays local: %s', async code => {
  invoke.mockImplementation(async () => { if (code === 'malformed') return { launchAtLogin: 'yes', closeToTray: true }; throw code })
  await render(); await until(() => host.textContent?.includes('Could not load desktop settings.') === true)
  expect(launch().disabled).toBe(true); expect(host.textContent).not.toContain(code)
  expect(host.querySelector<HTMLSelectElement>('[aria-label="Theme"]')?.closest('fieldset')?.disabled).toBe(false)
  expect(host.querySelector<HTMLInputElement>('[aria-label="Active projects"]')?.disabled).toBe(false)
  invoke.mockResolvedValue(defaults); act(() => button('Retry desktop settings').click()); await until(() => !launch().disabled)
  expect(invoke.mock.calls.every(([command]) => command === 'desktop_preferences')).toBe(true)
})
it('a failed Core save does not disable or invoke native switches', async () => {
  await render(); await until(() => !launch().disabled)
  coreFail = true
  act(() => host.querySelector<HTMLButtonElement>('button[aria-label="Dark"]')!.click())
  await until(() => host.textContent?.includes('Core save failed.') === true)
  expect(launch().disabled).toBe(false); expect(tray().disabled).toBe(false)
  expect(invoke.mock.calls).toEqual([['desktop_preferences', undefined]])
  // The ordinary code-index form also remains independent.
  typeInto(host.querySelector<HTMLInputElement>('[aria-label="Active projects"]')!, '8')
  expect(button('Save').disabled).toBe(false)
})
