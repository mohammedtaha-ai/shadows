// @vitest-environment happy-dom
import { personKeys } from '@/api/person'
import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { answers } from '@/test/fake-daemon'
import { startApp, typeInto, until, type TestApp } from '@/test/test-app'

let app: TestApp | null = null
afterEach(() => { app?.unmount(); app = null })
const person = { name: 'Mohammed', aylul_calls_me: 'Abu Taha', language: 'ar', theme: 'dark', updated_at: null }
const input = (name: string) => document.querySelector<HTMLInputElement>(`input[aria-label="${name}"]`)!
async function open(overrides = {}) {
  const a = app = await startApp('/?settings=account', { ...answers(), ...overrides })
  await until(() => input('Your name') !== null && !input('Your name').disabled)
  return a
}
it('the You card shows the saved names', async () => {
  await open({ 'GET /api/me': person })
  expect(input('Your name').value).toBe('Mohammed'); expect(input('What AYLUL calls you').value).toBe('Abu Taha')
})
// A GET after the save must answer what was saved, as Core does; a fixed GET answer would let
// any refetch put the old name back.
function stored(initial: typeof person) {
  let saved = initial
  return {
    'GET /api/me': () => Response.json(saved),
    'PUT /api/me': async (request: Request) => {
      const { command_id: _command, ...input } = await request.json()
      saved = { ...input, updated_at: null }
      return Response.json(saved)
    },
  }
}
it('saving sends the whole record', async () => {
  const a = await open(stored(person))
  typeInto(input('What AYLUL calls you'), 'Mo')
  expect(a.calls).not.toContain('PUT /api/me')
  act(() => a.button('Save')!.click())
  await until(() => a.calls.includes('PUT /api/me'))
  expect(a.bodies[0]).toEqual({ command_id: expect.any(String), name: 'Mohammed', aylul_calls_me: 'Mo', language: 'ar', theme: 'dark' })
  // Save is disabled while the PUT is pending too; the save has finished once the fields are
  // enabled again.
  await until(() => !input('What AYLUL calls you').disabled && a.button('Save')?.disabled === true)
  expect(input('What AYLUL calls you').value).toBe('Mo')
})
it('an emptied field saves as null', async () => {
  const a = await open({ 'GET /api/me': person }); typeInto(input('Your name'), '')
  act(() => a.button('Save')!.click()); await until(() => a.bodies.length > 0)
  expect(a.bodies[0]).toEqual(expect.objectContaining({ name: null }))
})
it('a fresh person shows empty fields', async () => {
  const a = await open()
  expect(input('Your name').value).toBe(''); expect(input('What AYLUL calls you').value).toBe('')
  expect(a.text()).toContain('AYLUL uses this name when it speaks to you. Without it, AYLUL uses your name.')
})
it('more than 64 characters is refused before sending', async () => {
  const a = await open(); typeInto(input('Your name'), 'x'.repeat(65))
  act(() => a.button('Save')!.click())
  expect(a.text()).toContain('At most 64 characters.'); expect(a.calls).not.toContain('PUT /api/me')
})
it('a failed save keeps the typed values and retries with the same command', async () => {
  let count = 0
  const a = await open({ 'PUT /api/me': async (request: Request) => {
    const body = await request.json(); return ++count === 1 ? Response.json({ code: 'STORAGE_UNAVAILABLE', message: 'Please retry.' }, { status: 500 }) : Response.json(body)
  } })
  typeInto(input('Your name'), 'Mo'); act(() => a.button('Save')!.click())
  await until(() => a.text().includes('Please retry.') && a.button('Save')?.disabled === false)
  expect(input('Your name').value).toBe('Mo')
  act(() => a.button('Save')!.click()); await until(() => a.bodies.length === 2)
  expect(a.bodies[1]).toEqual(a.bodies[0])
})
it('the browser lists Account with only You', async () => {
  const a = await open()
  expect(document.querySelector('[data-slot="dialog-title"]')?.textContent).toBe('Account')
  expect(document.querySelector('[aria-label="Account state"]')).toBeNull()
  expect(a.button('Sign in')).toBeUndefined()
})
it('a failed person load shows an error and no form', async () => {
  const a = app = await startApp('/?settings=account', { ...answers(), 'GET /api/me': () => Response.json({ code: 'STORAGE_UNAVAILABLE', message: 'Person unavailable.' }, { status: 500 }) })
  await until(() => a.text().includes('Person unavailable.'))
  expect(input('Your name')).toBeNull()
})
it('a read sent before the save cannot bring the old name back', async () => {
  let release: (() => void) | null = null
  let reads = 0
  const a = await open({
    'GET /api/me': async () => {
      if (++reads === 2) await new Promise<void>(resolve => { release = resolve })
      return Response.json(person)
    },
    'PUT /api/me': async (request: Request) => {
      const { command_id: _command, ...input } = await request.json()
      return Response.json({ ...input, updated_at: null })
    },
  })
  void act(() => a.queryClient.invalidateQueries({ queryKey: personKeys.record() }))
  await until(() => release !== null)
  typeInto(input('What AYLUL calls you'), 'Mo')
  act(() => a.button('Save')!.click())
  await until(() => !input('What AYLUL calls you').disabled && a.button('Save')?.disabled === true)
  await act(async () => { release!(); await new Promise(resolve => setTimeout(resolve, 50)) })
  expect(input('What AYLUL calls you').value).toBe('Mo')
})
