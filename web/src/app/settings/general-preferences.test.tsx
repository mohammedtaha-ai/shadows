// @vitest-environment happy-dom
import { act } from 'react'
import { createRoot } from 'react-dom/client'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { afterEach, expect, it } from 'vitest'
import { personQuery, type Person } from '@/api/person'
import { answers } from '@/test/fake-daemon'
import { GeneralPreferences } from './general-preferences'
import { YouCard } from './you-card'
import { startApp, typeInto, until, type TestApp, type Answer } from '@/test/test-app'

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
it('General instantly saves only the picked preference', async () => {
  const a = await open(); expect(select('Language').value).toBe('en'); expect(select('Theme').value).toBe('system')
  act(() => a.button('Dark')!.click())
  await until(() => a.bodies.length === 1)
  expect(a.bodies[0]).toEqual({ command_id: expect.any(String), theme: 'dark' })
  await until(() => !select('Language').disabled)
  choose('Language', '')
  await until(() => a.bodies.length === 2)
  expect(a.bodies[1]).toEqual({ command_id: expect.any(String), language: null })
})
it('an unknown saved language survives a theme-only change', async () => {
  const a = await open(answers({ person: { ...person, language: 'ar' } }))
  expect(select('Language').value).toBe('ar')
  expect([...select('Language').options].find(option => option.value === 'ar')?.disabled).toBe(true)
  act(() => a.button('Light')!.click())
  await until(() => a.bodies.length === 1)
  expect(a.bodies[0]).toEqual({ command_id: expect.any(String), theme: 'light' })
  await until(() => !select('Theme').disabled)
  expect(select('Language').value).toBe('ar')
})
it('preference patches omit even newly observed names', async () => {
  const a = await open()
  await act(async () => a.queryClient.setQueryData(personQuery.queryKey, { ...person, name: 'New name', aylul_calls_me: 'New nickname' }))
  act(() => a.button('Dark')!.click())
  await until(() => a.bodies.length === 1)
  expect(a.bodies[0]).toEqual({ command_id: expect.any(String), theme: 'dark' })
})
it('a failed preference save retains its draft and command', async () => {
  const a = await open({ 'PATCH /api/me': () => Response.json({ code: 'STORAGE_UNAVAILABLE', message: 'Save refused.' }, { status: 500 }) })
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
  const a = await open({ 'PATCH /api/me': () => new Promise<Response>(resolve => { finish = resolve }) })
  const button = a.button('Dark')!
  act(() => { button.click(); button.click() })
  await until(() => a.bodies.length === 1)
  expect(button.disabled).toBe(true)
  expect(select('Theme').closest('fieldset')?.disabled).toBe(true)
  await act(async () => finish(Response.json({ ...person, theme: 'dark' })))
  await until(() => !select('Theme').closest('fieldset')?.disabled)
  act(() => a.button('Account')!.click())
  await until(() => select('Theme') === null)
  expect(a.calls.filter(call => call === 'PATCH /api/me')).toHaveLength(1)
})
it('changing language after a failed theme save sends only language', async () => {
  const a = await open({ 'PATCH /api/me': () => Response.json({ code: 'STORAGE_UNAVAILABLE', message: 'Save refused.' }, { status: 500 }) })
  act(() => a.button('Dark')!.click())
  await until(() => a.text().includes('Save refused.') && !select('Language').disabled)
  choose('Language', '')
  await until(() => a.bodies.length === 2)
  expect(a.bodies[1]).toEqual({ command_id: expect.any(String), language: null })
  expect((a.bodies[1] as { command_id: string }).command_id).not.toBe((a.bodies[0] as { command_id: string }).command_id)
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

it.each([0, 1])('both forms save from the same snapshot with patch %s applied first', async first => {
  const routes = answers({ person })
  const applyPatch = routes['PATCH /api/me'] as (request: Request) => Promise<Response>
  const pending: { request: Request; finish: (response: Response) => void }[] = []
  const a = app = await startApp('/', {
    ...routes,
    'PATCH /api/me': (request: Request) => new Promise<Response>(finish => pending.push({ request, finish })),
  })
  const host = document.createElement('div'); document.body.append(host)
  const root = createRoot(host)
  const client = new QueryClient({ defaultOptions: { queries: { enabled: false, initialData: person } } })
  try {
    await act(async () => root.render(<QueryClientProvider client={client}><YouCard /><GeneralPreferences /></QueryClientProvider>))
    typeInto(host.querySelector<HTMLInputElement>('input[aria-label="Your name"]')!, 'Mo')
    act(() => {
      host.querySelector<HTMLButtonElement>('form[aria-label="You"] button[type="submit"]')!.click()
      host.querySelector<HTMLButtonElement>('button[aria-label="Dark"]')!.click()
    })
    await until(() => pending.length === 2)
    expect(client.getQueryData(personQuery.queryKey)).toEqual(person)
    expect(a.bodies).toEqual(expect.arrayContaining([
      { command_id: expect.any(String), name: 'Mo' },
      { command_id: expect.any(String), theme: 'dark' },
    ]))
    for (const index of [first, 1 - first]) {
      await act(async () => pending[index]!.finish(await applyPatch(pending[index]!.request)))
    }
    await until(() => !host.querySelector<HTMLFieldSetElement>('form[aria-label="You"] fieldset')!.disabled && !select('Theme').disabled)
    const stored = await client.fetchQuery({ ...personQuery, staleTime: 0 })
    expect(stored).toEqual({ ...person, name: 'Mo', theme: 'dark', updated_at: expect.any(String) })
  } finally {
    await act(async () => root.unmount()); client.clear(); host.remove()
  }
})
