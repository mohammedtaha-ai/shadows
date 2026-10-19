// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, typeInto, until } from '../test-app'

let open: TestApp | null = null
afterEach(() => { open?.unmount(); open = null })
const field = (name: string) => document.querySelector<HTMLInputElement>(`input[aria-label="${name}"]`)
const entry = { name: 'API_KEY', updated_at: '2026-10-19T12:00:00Z' }

it('lists_names_without_values', async () => {
  const app = (open = await startApp('/settings', { ...answers(), 'GET /api/aylul/env': [entry] }))
  await until(() => app.text().includes('API_KEY'))
  expect(app.text()).toContain(entry.updated_at)
  expect(app.button('Replace API_KEY')).toBeDefined()
  expect(app.button('Remove API_KEY')).toBeDefined()
  expect(field('Environment value')?.type).toBe('password')
})

it('adding_clears_the_input', async () => {
  const table = { ...answers(), 'GET /api/aylul/env': () => Response.json([]),
    'PUT /api/aylul/env/API_KEY': () => Response.json(entry) }
  const app = (open = await startApp('/settings', table))
  await until(() => field('Environment name') !== null)
  typeInto(field('Environment name')!, 'api_key')
  expect(field('Environment name')?.value).toBe('API_KEY')
  typeInto(field('Environment value')!, 'synthetic-ui-fixture')
  act(() => app.button('Add variable')?.click())
  await until(() => app.calls.includes('PUT /api/aylul/env/API_KEY'))
  await until(() => field('Environment value')?.value === '')
  expect(!app.text().includes('synthetic-ui-fixture')).toBe(true)
  expect(field('Environment value')?.value === '').toBe(true)
})

it('hidden_only_when_not_configured', async () => {
  const app = (open = await startApp('/settings', { ...answers(),
    'GET /api/aylul/env': () => Response.json({ code: 'AYLUL_NOT_CONFIGURED', message: 'absent' },
      { status: 404 }) }))
  await until(() => app.calls.includes('GET /api/aylul/env'))
  await until(() => !app.text().includes('AYLUL environment'))
  expect(app.text()).toContain('Active projects')
})

it('shows_unavailable_error', async () => {
  const app = (open = await startApp('/settings', { ...answers(),
    'GET /api/aylul/env': () => Response.json({ code: 'SECRETS_UNAVAILABLE',
      message: 'encrypted environment storage is unavailable' }, { status: 503 }) }))
  await until(() => app.text().includes('encrypted environment storage is unavailable'))
  expect(app.text()).toContain('AYLUL environment')
})

it('replaces_and_removes_a_variable', async () => {
  let rows = [entry]
  const table = { ...answers(), 'GET /api/aylul/env': () => Response.json(rows),
    'PUT /api/aylul/env/API_KEY': () => Response.json(entry),
    'DELETE /api/aylul/env/API_KEY': () => { rows = []; return new Response(null, { status: 204 }) } }
  const app = (open = await startApp('/settings', table))
  await until(() => app.button('Replace API_KEY') !== undefined)
  act(() => app.button('Replace API_KEY')?.click())
  typeInto(field('Replace value for API_KEY')!, 'synthetic-replacement')
  act(() => app.button('Save API_KEY')?.click())
  await until(() => field('Replace value for API_KEY') === null)
  expect(!app.text().includes('synthetic-replacement')).toBe(true)
  act(() => app.button('Remove API_KEY')?.click())
  await until(() => app.button('Remove API_KEY') === undefined)
  expect(app.calls).toContain('DELETE /api/aylul/env/API_KEY')
})
