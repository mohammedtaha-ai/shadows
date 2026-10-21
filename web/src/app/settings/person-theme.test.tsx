// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { personQuery } from '@/api/person'
import { answers } from '@/test/fake-daemon'
import { startApp, until, type TestApp } from '../test-app'
import { PersonTheme } from './person-theme'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createRoot } from 'react-dom/client'

let app: TestApp | null = null
let dark = false
const listeners = new Set<(event: { matches: boolean }) => void>()
const person = { name: null, aylul_calls_me: null, language: null, theme: 'system' as const, updated_at: null }
beforeEach(() => {
  dark = false; listeners.clear(); document.documentElement.classList.add('dark'); document.documentElement.removeAttribute('data-theme')
  const original = window.matchMedia.bind(window)
  vi.spyOn(window, 'matchMedia').mockImplementation(query => query === '(prefers-color-scheme: dark)' ? {
    get matches() { return dark }, media: query, onchange: null,
    addEventListener: (_: string, callback: (event: { matches: boolean }) => void) => { listeners.add(callback) },
    removeEventListener: (_: string, callback: (event: { matches: boolean }) => void) => { listeners.delete(callback) },
  } as MediaQueryList : original(query))
})
afterEach(() => { app?.unmount(); app = null; vi.restoreAllMocks(); listeners.clear(); document.documentElement.removeAttribute('data-theme') })
it('theme follows the shared person', async () => {
  const a = app = await startApp('/?settings=general', answers({ person: { ...person, theme: 'dark' } }))
  await until(() => document.documentElement.dataset.theme === 'dark')
  expect(document.documentElement.classList.contains('dark')).toBe(true)
  const select = document.querySelector<HTMLSelectElement>('select[aria-label="Theme"]')!
  act(() => { select.value = 'light'; select.dispatchEvent(new Event('change', { bubbles: true })) })
  act(() => a.button('Save preferences')!.click())
  await until(() => document.documentElement.dataset.theme === 'light')
  expect(document.documentElement.classList.contains('dark')).toBe(false)
  expect(a.path()).toBe('/')
})
it('System follows the OS and fixed themes do not', async () => {
  const a = app = await startApp('/', answers({ person }))
  await until(() => document.documentElement.dataset.theme === 'light')
  expect(listeners.size).toBe(1)
  await act(async () => { dark = true; for (const listener of listeners) listener({ matches: true }) })
  expect(document.documentElement.dataset.theme).toBe('dark')
  await act(async () => a.queryClient.setQueryData(personQuery.queryKey, { ...person, theme: 'light' }))
  await until(() => document.documentElement.dataset.theme === 'light')
  expect(document.documentElement.dataset.theme).toBe('light'); expect(listeners.size).toBe(0)
  dark = true; expect(document.documentElement.classList.contains('dark')).toBe(false)
  a.unmount(); app = null
  expect(document.documentElement.classList.contains('dark')).toBe(true)
  expect(document.documentElement.hasAttribute('data-theme')).toBe(false); expect(listeners.size).toBe(0)
})
it('failed person loading preserves the initial theme', async () => {
  const a = app = await startApp('/', { ...answers(), 'GET /api/me': () => Response.json({ code: 'STORAGE_UNAVAILABLE', message: 'Unavailable.' }, { status: 500 }) })
  await until(() => a.queryClient.getQueryState(personQuery.queryKey)?.status === 'error')
  expect(document.documentElement.classList.contains('dark')).toBe(true)
  expect(document.documentElement.hasAttribute('data-theme')).toBe(false)
})
it('unmount removes System listeners and restores prior attributes', async () => {
  const host = document.createElement('div'); document.body.append(host); const root = createRoot(host)
  const client = new QueryClient({ defaultOptions: { queries: { staleTime: Infinity } } }); client.setQueryData(personQuery.queryKey, person)
  document.documentElement.dataset.theme = 'previous'
  try {
    await act(async () => root.render(<QueryClientProvider client={client}><PersonTheme /></QueryClientProvider>))
    expect(listeners.size).toBe(1)
  } finally { await act(async () => root.unmount()); client.clear(); host.remove() }
  expect(document.documentElement.dataset.theme).toBe('previous'); expect(listeners.size).toBe(0)
})
