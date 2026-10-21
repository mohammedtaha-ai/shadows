// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { answers } from '@/test/fake-daemon'
import { startApp, typeInto, until, type TestApp } from '../test-app'

let app: TestApp | null = null
afterEach(() => { app?.unmount(); app = null; window.localStorage.clear() })
const input = (label: string) => document.querySelector<HTMLInputElement>(`input[aria-label="${label}"]`)!
const provider = { slug: 'p', name: 'MiniMax', preset: 'minimax', label: 'Work key', base_url: 'https://api.minimax.io/anthropic', models: [{ id: 'm1', context_window: 500000 }, { id: 'm2', context_window: 200000 }], key_set: true, created_at: 'now', updated_at: 'now' }
const base = () => ({ ...answers(), 'GET /api/aylul/providers': [provider], 'GET /api/aylul/agents': { project_id: 'p1', agents: [] } })
async function setup(name = 'MiniMax', overrides = {}) {
  const a = app = await startApp('/aylul?settings=providers', { ...base(), ...overrides })
  await until(() => a.button('Add provider') !== undefined)
  act(() => a.button('Add provider')!.click())
  await until(() => a.button(name) !== undefined)
  act(() => a.button(name)!.click()); act(() => a.button('Next')!.click())
  await until(() => input('API key') !== null)
  return a
}
async function testKey(a: TestApp) {
  typeInto(input('API key'), 'sk-1'); act(() => a.button('Test connection')!.click())
  await until(() => a.text().includes('Key works.') || document.querySelector('[role="alert"]') !== null)
}
function addModel(a: TestApp, id: string) {
  typeInto(input('Add a model by id'), id); act(() => a.button('Add')!.click())
}
it('presets come from Core, grouped by name', async () => {
  const a = app = await startApp('/aylul?settings=providers', base())
  await until(() => a.button('Add provider') !== undefined); act(() => a.button('Add provider')!.click())
  await until(() => a.button('MiniMax') !== undefined)
  expect([...document.querySelectorAll('[aria-pressed]')].map(button => button.textContent)).toEqual(['MiniMax', 'Anthropic', 'DeepSeek', 'OpenRouter', 'Moonshot (Kimi)', 'Custom'])
  expect(a.calls).toContain('GET /api/aylul/provider-presets')
})
it('the list names a provider by preset and label', async () => {
  const a = app = await startApp('/aylul?settings=providers', { ...base(), 'GET /api/aylul/providers': [provider, { ...provider, slug: 'china', preset: 'minimax-china', label: null }] })
  await until(() => a.text().includes('MiniMax China'))
  const rows = [...document.querySelectorAll('[aria-label="Saved providers"] li')]
  expect(rows.map(row => row.firstElementChild?.textContent)).toEqual(['MiniMax (Work key)', 'MiniMax China'])
})
it('a preset saves its id, not its URL', async () => {
  const a = await setup('MiniMax', { 'POST /api/aylul/providers': () => Response.json(provider), 'PUT /api/aylul/providers/p/key': {} })
  act(() => input('China').click()); typeInto(input('Label'), 'Home'); typeInto(input('API key'), 'sk-1'); addModel(a, 'MiniMax-M3')
  act(() => a.button('Add 1 models')!.click())
  await until(() => a.calls.includes('PUT /api/aylul/providers/p/key'))
  expect(a.bodies[0]).toEqual({ command_id: expect.any(String), preset: 'minimax-china', label: 'Home', models: [{ id: 'MiniMax-M3', context_window: 200000 }] })
  expect(a.bodies[1]).toEqual({ command_id: expect.any(String), value: 'sk-1' })
})
it("Test connection lists the key's models", async () => {
  const a = await setup('MiniMax', { 'POST /api/aylul/providers/test': { ok: true, listed: true, models: [{ id: 'a', context_window: 1000000 }, { id: 'b', context_window: null }] }, 'POST /api/aylul/providers': () => Response.json(provider), 'PUT /api/aylul/providers/p/key': {} })
  await testKey(a)
  expect(a.bodies[0]).toEqual({ preset: 'minimax', api_key: 'sk-1' })
  expect(a.text()).toContain('Key works. Choose the models to add.')
  expect(input('a').checked).toBe(false); expect(input('b').checked).toBe(false)
  act(() => input('a').click()); act(() => input('b').click()); act(() => input('b default').click())
  act(() => a.button('Add 2 models')!.click())
  await until(() => a.calls.includes('POST /api/aylul/providers'))
  expect(a.bodies[1]).toEqual(expect.objectContaining({ models: [{ id: 'b', context_window: 200000 }, { id: 'a', context_window: 1000000 }] }))
})
it('search filters the models', async () => {
  const a = await setup('MiniMax', { 'POST /api/aylul/providers/test': { ok: true, listed: true, models: ['MiniMax-M3', 'MiniMax-M3.1-Flash', 'MiniMax-M2.5'].map(id => ({ id })) } })
  await testKey(a); act(() => input('MiniMax-M3').click()); typeInto(input('Search models'), 'flash')
  expect([...document.querySelectorAll('input[type="checkbox"]')].map(element => element.getAttribute('aria-label'))).toEqual(['MiniMax-M3.1-Flash'])
  typeInto(input('Search models'), ''); expect(input('MiniMax-M3').checked).toBe(true)
})
it('a refused or unreachable key says so', async () => {
  let outcome = { ok: false, listed: false, models: [], reason: 'key_refused', status: 401, message: '' }
  const a = await setup('MiniMax', { 'POST /api/aylul/providers/test': () => Response.json(outcome) })
  await testKey(a); expect(a.text()).toContain('The provider refused this key.')
  expect(document.querySelector('input[type="checkbox"]')).toBeNull()
  outcome = { ...outcome, reason: 'unreachable' }; act(() => a.button('Test connection')!.click())
  await until(() => a.text().includes('Could not reach MiniMax. Check your connection and retry.'))
  outcome = { ...outcome, reason: 'unexpected', message: 'Odd answer.' }; act(() => a.button('Test connection')!.click())
  await until(() => a.text().includes('Odd answer.'))
})
it('Custom tests its URL and offers add by hand when nothing is listed', async () => {
  const a = await setup('Custom', { 'POST /api/aylul/providers/test': { ok: true, listed: false, models: [] }, 'POST /api/aylul/providers': () => Response.json(provider), 'PUT /api/aylul/providers/p/key': {} })
  typeInto(input('Provider name'), 'Mine'); typeInto(input('Provider URL'), 'https://x.example/anthropic'); await testKey(a)
  expect(a.bodies[0]).toEqual({ base_url: 'https://x.example/anthropic', api_key: 'sk-1' })
  expect(a.text()).toContain('This provider does not list its models: add them by id.')
  addModel(a, 'manual'); act(() => a.button('Add 1 models')!.click())
  await until(() => a.calls.includes('POST /api/aylul/providers'))
  expect(a.bodies[1]).toEqual(expect.objectContaining({ preset: null, name: 'Mine', base_url: 'https://x.example/anthropic' }))
})
it('Edit tests with the stored key', async () => {
  const a = app = await startApp('/aylul?settings=providers', { ...base(), 'POST /api/aylul/providers/p/test': { ok: true, listed: false, models: [] } })
  await until(() => a.button('Edit') !== undefined); act(() => a.button('Edit')!.click())
  act(() => a.button('Test connection')!.click()); await until(() => a.bodies.length === 1)
  expect(a.bodies[0]).toEqual({}); expect(input('API key').value).toBe('')
  await until(() => a.button('Test connection')?.disabled === false)
  typeInto(input('API key'), 'sk-2'); act(() => a.button('Test connection')!.click())
  await until(() => input('API key').value === '')
  expect(a.bodies[1]).toEqual({ api_key: 'sk-2' })
})
it('Edit shows the saved models with their windows', async () => {
  const a = app = await startApp('/aylul?settings=providers', base())
  await until(() => a.button('Edit') !== undefined); act(() => a.button('Edit')!.click())
  expect(input('m1').checked).toBe(true); expect(input('m2').checked).toBe(true); expect(input('m1 default').checked).toBe(true)
  expect(input('Context window for m1').value).toBe('500000'); expect(input('Context window for m2').value).toBe('200000')
  expect(input('Context window')).toBeNull(); expect(a.button('Save provider')).toBeDefined()
})
it('a window out of range is refused', async () => {
  const a = app = await startApp('/aylul?settings=providers', base())
  await until(() => a.button('Edit') !== undefined); act(() => a.button('Edit')!.click())
  typeInto(input('Context window for m1'), '999'); act(() => a.button('Save provider')!.click())
  expect(a.text()).toContain('Context window must be a whole number from 1000 to 10000000.')
  expect(a.calls).not.toContain('PUT /api/aylul/providers/p')
})
