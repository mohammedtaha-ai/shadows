// @vitest-environment happy-dom
import { act } from 'react'
import { createRoot } from 'react-dom/client'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { afterEach, expect, it } from 'vitest'
import { personQuery, type Person } from '@/api/person'
import { answers } from '@/test/fake-daemon'
import { GeneralPreferences } from './general-preferences'
import { startApp, until, type TestApp, type Answer } from '@/test/test-app'

const person: Person = { name: 'Mohammed', aylul_calls_me: 'Abu Taha', language: 'en', theme: 'system', updated_at: null }
let app: TestApp | null = null
afterEach(() => { app?.unmount(); app = null })
const select = (name: string) => document.querySelector<HTMLSelectElement>(`select[aria-label="${name}"]`)!
function choose(name: string, value: string) { act(() => { select(name).value = value; select(name).dispatchEvent(new Event('change', { bubbles: true })) }) }
async function open(overrides: Record<string, Answer> = {}) {
  const a = app = await startApp('/?settings=general', { ...answers({ person }), ...overrides })
  await until(() => select('Theme') !== null && !select('Theme').disabled)
  return a
}
it('General reads and saves the whole person', async () => {
  const a = await open(); expect(select('Language').value).toBe('en'); expect(select('Theme').value).toBe('system')
  act(() => a.button('Dark')!.click())
  await until(() => a.bodies.length === 1)
  expect(a.bodies[0]).toEqual({ command_id: expect.any(String), name: 'Mohammed', aylul_calls_me: 'Abu Taha', language: 'en', theme: 'dark' })
  choose('Language', '')
  await until(() => a.bodies.length === 2)
  expect(a.bodies[1]).toEqual(expect.objectContaining({ language: null }))
})
it('an unknown saved language survives a theme-only change', async () => {
  const a = await open({ 'GET /api/me': { ...person, language: 'ar' } })
  expect(select('Language').value).toBe('ar')
  expect([...select('Language').options].find(option => option.value === 'ar')?.disabled).toBe(true)
  act(() => a.button('Light')!.click())
  await until(() => a.bodies.length === 1)
  expect(a.bodies[0]).toEqual(expect.objectContaining({ language: 'ar', theme: 'light' }))
})
it('observed names are preserved while editing preferences', async () => {
  const a = await open()
  await act(async () => a.queryClient.setQueryData(personQuery.queryKey, { ...person, name: 'New name', aylul_calls_me: 'New nickname' }))
  act(() => a.button('Dark')!.click())
  await until(() => a.bodies.length === 1)
  expect(a.bodies[0]).toEqual(expect.objectContaining({ name: 'New name', aylul_calls_me: 'New nickname', theme: 'dark' }))
})
it('a failed preference save retains its draft and command', async () => {
  const a = await open({ 'PUT /api/me': () => Response.json({ code: 'STORAGE_UNAVAILABLE', message: 'Save refused.' }, { status: 500 }) })
  act(() => a.button('Dark')!.click())
  await until(() => a.text().includes('Save refused.') && a.button('Dark')?.disabled === false)
  expect(select('Theme').value).toBe('dark')
  act(() => a.button('Dark')!.click())
  await until(() => a.bodies.length === 2 && !a.button('Dark')?.disabled)
  expect(a.bodies[1]).toEqual(a.bodies[0])
  act(() => a.button('Light')!.click())
  await until(() => a.bodies.length === 3)
  expect((a.bodies[2] as { command_id: string }).command_id).not.toBe((a.bodies[0] as { command_id: string }).command_id)
})
it('pending saves cannot submit twice and closing an unsaved form sends nothing', async () => {
  let finish!: (response: Response) => void
  const a = await open({ 'PUT /api/me': () => new Promise<Response>(resolve => { finish = resolve }) })
  const button = a.button('Dark')!
  act(() => { button.click(); button.click() })
  await until(() => a.bodies.length === 1)
  expect(button.disabled).toBe(true)
  expect(select('Theme').closest('fieldset')?.disabled).toBe(true)
  await act(async () => finish(Response.json({ ...person, theme: 'dark' })))
  await until(() => !select('Theme').closest('fieldset')?.disabled)
  act(() => a.button('Account')!.click())
  await until(() => select('Theme') === null)
  expect(a.calls.filter(call => call === 'PUT /api/me')).toHaveLength(1)
})
it('an old read cannot undo saved preferences', async () => {
  let reads = 0, release!: (response: Response) => void
  const a = await open({ 'GET /api/me': () => ++reads === 1 ? Response.json(person) : new Promise<Response>(resolve => { release = resolve }) })
  act(() => { void a.queryClient.invalidateQueries({ queryKey: personQuery.queryKey }) }); await until(() => reads === 2)
  act(() => a.button('Light')!.click())
  await until(() => a.queryClient.getQueryData<Person>(personQuery.queryKey)?.theme === 'light')
  await act(async () => release(Response.json(person)))
  expect(a.queryClient.getQueryData<Person>(personQuery.queryKey)?.theme).toBe('light')
  expect(select('Theme').value).toBe('light')
})
it('loading preferences disables the fields', async () => {
  const host = document.createElement('div'); document.body.append(host); const root = createRoot(host)
  const client = new QueryClient({ defaultOptions: { queries: { enabled: false } } })
  try {
    await act(async () => root.render(<QueryClientProvider client={client}><GeneralPreferences /></QueryClientProvider>))
    expect(host.querySelector('fieldset')?.disabled).toBe(true)
  } finally { await act(async () => root.unmount()); client.clear(); host.remove() }
})
it('failed preference loading shows no editable invented values', async () => {
  const a = app = await startApp('/?settings=general', { ...answers(), 'GET /api/me': () => Response.json({ code: 'STORAGE_UNAVAILABLE', message: 'Person unavailable.' }, { status: 500 }) })
  await until(() => a.text().includes('Person unavailable.'))
  expect(select('Theme')).toBeNull()
})
