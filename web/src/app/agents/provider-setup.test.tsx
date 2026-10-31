// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { answers } from '@/test/fake-daemon'
import { startApp, typeInto, until, type TestApp } from '@/test/test-app'

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
it.each(['new', 'saved'])('Enter adds a model without submitting the %s provider', async (mode) => {
  const overrides = { 'POST /api/aylul/providers': () => Response.json(provider), 'PUT /api/aylul/providers/p': () => Response.json(provider) }
  const a = mode === 'new' ? await setup('MiniMax', overrides) : app = await startApp('/aylul?settings=providers', { ...base(), ...overrides })
  if (mode === 'saved') {
    await until(() => a.button('Edit') !== undefined); act(() => a.button('Edit')!.click())
  }
  typeInto(input('Add a model by id'), 'manual')
  const enter = new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true })
  await act(async () => { input('Add a model by id').dispatchEvent(enter) })
  expect(enter.defaultPrevented).toBe(true)
  expect(input('manual').checked).toBe(true)
  expect(input('Add a model by id').value).toBe('')
  expect(a.calls).not.toContain('POST /api/aylul/providers')
  expect(a.calls).not.toContain('PUT /api/aylul/providers/p')
  act(() => a.button(mode === 'new' ? 'Add 1 model' : 'Save provider')!.click())
  await until(() => a.bodies.length === 1)
  expect(a.bodies[0]).toEqual(expect.objectContaining({ models: [...(mode === 'saved' ? provider.models : []), { id: 'manual' }] }))
})
it.each(['new', 'saved'])('saving the %s provider with an unadded id explains it and preserves the draft', async (mode) => {
  const overrides = { 'POST /api/aylul/providers': () => Response.json(provider), 'PUT /api/aylul/providers/p': () => Response.json(provider) }
  const a = mode === 'new' ? await setup('MiniMax', overrides) : app = await startApp('/aylul?settings=providers', { ...base(), ...overrides })
  if (mode === 'saved') {
    await until(() => a.button('Edit') !== undefined); act(() => a.button('Edit')!.click())
  } else addModel(a, 'm1')
  typeInto(input('Add a model by id'), 'manual')
  await act(async () => { a.button(mode === 'new' ? 'Add 1 model' : 'Save provider')!.click() })
  expect(document.querySelector('[role="alert"]')?.textContent).toBe('You have an unadded model id. Add it or clear the field before saving.')
  expect(input('Add a model by id').value).toBe('manual')
  expect(a.calls).not.toContain('POST /api/aylul/providers')
  expect(a.calls).not.toContain('PUT /api/aylul/providers/p')
  act(() => a.button('Add')!.click())
  expect(a.text()).not.toContain('You have an unadded model id')
  act(() => a.button(mode === 'new' ? 'Add 2 models' : 'Save provider')!.click())
  await until(() => a.bodies.length === 1)
  expect(a.bodies[0]).toEqual(expect.objectContaining({ models: [...(mode === 'saved' ? provider.models : [{ id: 'm1' }]), { id: 'manual' }] }))
})
it('clearing an unadded id removes its message and saves the picked models', async () => {
  const a = app = await startApp('/aylul?settings=providers', { ...base(), 'PUT /api/aylul/providers/p': () => Response.json(provider) })
  await until(() => a.button('Edit') !== undefined); act(() => a.button('Edit')!.click())
  typeInto(input('Add a model by id'), 'manual')
  await act(async () => { a.button('Save provider')!.click() })
  expect(a.text()).toContain('You have an unadded model id')
  typeInto(input('Add a model by id'), '')
  expect(a.text()).not.toContain('You have an unadded model id')
  act(() => a.button('Save provider')!.click())
  await until(() => a.bodies.length === 1)
  expect(a.bodies[0]).toEqual(expect.objectContaining({ models: provider.models }))
})
it('Shift+Enter adds a model like Enter', async () => {
  const a = app = await startApp('/aylul?settings=providers', base())
  await until(() => a.button('Edit') !== undefined); act(() => a.button('Edit')!.click())
  typeInto(input('Add a model by id'), 'manual')
  const enter = new KeyboardEvent('keydown', { key: 'Enter', shiftKey: true, bubbles: true, cancelable: true })
  await act(async () => { input('Add a model by id').dispatchEvent(enter) })
  expect(enter.defaultPrevented).toBe(true)
  expect(input('manual').checked).toBe(true)
  expect(a.calls).not.toContain('PUT /api/aylul/providers/p')
})
it('Enter that confirms an IME composition does not add the model', async () => {
  const a = app = await startApp('/aylul?settings=providers', base())
  await until(() => a.button('Edit') !== undefined); act(() => a.button('Edit')!.click())
  typeInto(input('Add a model by id'), 'モデル')
  await act(async () => { input('Add a model by id').dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', isComposing: true, bubbles: true, cancelable: true })) })
  expect(input('Add a model by id').value).toBe('モデル')
  expect(input('モデル')).toBeNull()
})
it('Enter with a duplicate model id does not submit the provider', async () => {
  const id = 'm1'
  const a = app = await startApp('/aylul?settings=providers', base())
  await until(() => a.button('Edit') !== undefined); act(() => a.button('Edit')!.click())
  typeInto(input('Add a model by id'), id)
  const enter = new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true })
  await act(async () => { input('Add a model by id').dispatchEvent(enter) })
  expect(enter.defaultPrevented).toBe(true)
  expect(document.querySelectorAll('input[aria-label="m1"]')).toHaveLength(2)
  expect(input('Add a model by id').value).toBe('')
  expect(a.calls).not.toContain('PUT /api/aylul/providers/p')
})
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
  act(() => a.button('Add 1 model')!.click())
  await until(() => a.calls.includes('PUT /api/aylul/providers/p/key'))
  expect(a.bodies[0]).toEqual({ command_id: expect.any(String), preset: 'minimax-china', label: 'Home', models: [{ id: 'MiniMax-M3' }] })
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
  expect(a.bodies[1]).toEqual(expect.objectContaining({ models: [{ id: 'b' }, { id: 'a', context_window: 1000000 }] }))
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
  addModel(a, 'manual'); act(() => a.button('Add 1 model')!.click())
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
  await until(() => a.text().includes('Context window must be from 1000 to 10000000.'))
  expect(a.calls).toContain('PUT /api/aylul/providers/p')
  expect(input('Context window for m1').value).toBe('999')
  typeInto(input('Context window for m1'), '1000')
  expect(a.text()).not.toContain('Context window must be from 1000 to 10000000.')
})

it('sends duplicate models to Core', async () => {
  const a = await setup('MiniMax')
  addModel(a, 'same'); addModel(a, 'same')
  act(() => a.button('Add 2 models')!.click())
  await until(() => a.text().includes('Each model is listed once.'))
  const rows = document.querySelectorAll<HTMLInputElement>('input[aria-label="same"]')
  expect(rows).toHaveLength(2)
  expect(rows[1].closest('div')?.textContent).toContain('Each model is listed once.')
})

it('clears a duplicate refusal when its row is removed', async () => {
  const a = await setup('MiniMax')
  addModel(a, 'same'); addModel(a, 'same'); addModel(a, 'same')
  act(() => a.button('Add 3 models')!.click())
  await until(() => a.text().includes('Each model is listed once.'))
  const rows = document.querySelectorAll<HTMLInputElement>('input[aria-label="same"]')
  act(() => rows[1].click())
  expect(a.text()).not.toContain('Each model is listed once.')
})

it('keeps a model refusal on its row after reordering', async () => {
  const a = app = await startApp('/aylul?settings=providers', base())
  await until(() => a.button('Edit') !== undefined); act(() => a.button('Edit')!.click())
  typeInto(input('Context window for m2'), '999'); act(() => a.button('Save provider')!.click())
  await until(() => a.text().includes('Context window must be from 1000 to 10000000.'))
  act(() => input('m2 default').click())
  expect(input('Context window for m2').closest('div')?.textContent).toContain('Context window must be from 1000 to 10000000.')
  typeInto(input('Context window for m1'), '1234')
  expect(a.text()).toContain('Context window must be from 1000 to 10000000.')
  typeInto(input('Context window for m2'), '1234')
  expect(a.text()).not.toContain('Context window must be from 1000 to 10000000.')
})

it('keeps a model refusal on its row after removing an earlier model', async () => {
  const a = app = await startApp('/aylul?settings=providers', base())
  await until(() => a.button('Edit') !== undefined); act(() => a.button('Edit')!.click())
  typeInto(input('Context window for m2'), '999'); act(() => a.button('Save provider')!.click())
  await until(() => a.text().includes('Context window must be from 1000 to 10000000.'))
  act(() => input('m1').click())
  expect(input('Context window for m2').closest('div')?.textContent).toContain('Context window must be from 1000 to 10000000.')
})

it('uses a new command after a definitive provider refusal', async () => {
  const a = await setup('MiniMax')
  addModel(a, 'default'); act(() => a.button('Add 1 model')!.click())
  await until(() => a.text().includes('default is reserved.'))
  act(() => a.button('Add 1 model')!.click())
  await until(() => a.bodies.length === 2)
  const attempts = a.bodies as { command_id: string }[]
  expect(attempts[0].command_id).not.toBe(attempts[1].command_id)
})

it.each(['new', 'saved'])('shows Core model refusals on the %s form', async mode => {
  const a = mode === 'new' ? await setup('MiniMax') : app = await startApp('/aylul?settings=providers', base())
  if (mode === 'saved') { await until(() => a.button('Edit') !== undefined); act(() => a.button('Edit')!.click()) }
  addModel(a, 'default')
  act(() => a.button(mode === 'new' ? 'Add 1 model' : 'Save provider')!.click())
  await until(() => a.text().includes('default is reserved.'))
  expect(input('default').closest('div')?.textContent).toContain('default is reserved.')
  act(() => input('default').click())
  expect(a.text()).not.toContain('default is reserved.')
})

it.each(['new', 'saved'])('shows the models-list refusal on the %s form', async mode => {
  const a = mode === 'new' ? await setup('MiniMax') : app = await startApp('/aylul?settings=providers', base())
  if (mode === 'saved') {
    await until(() => a.button('Edit') !== undefined); act(() => a.button('Edit')!.click())
    act(() => { input('m1').click(); input('m2').click() })
  }
  act(() => a.button(mode === 'new' ? 'Add 0 models' : 'Save provider')!.click())
  await until(() => a.text().includes('Models must be 1 to 20 ids.'))
  expect(document.querySelector('[aria-label="Provider models"]')?.textContent).toContain('Models must be 1 to 20 ids.')
  addModel(a, 'fixed'); expect(a.text()).not.toContain('Models must be 1 to 20 ids.')
})

it.each([
  ['name', 'Provider name'], ['base_url', 'Provider URL'], ['label', 'Label'],
])('shows the %s refusal beside its input and clears it on edit', async (field, label) => {
  const message = `Core refused ${field}`
  const a = await setup('Custom', { 'POST /api/aylul/providers': () => Response.json({ code: 'PROFILE_INVALID', field, message }, { status: 400 }) })
  typeInto(input('Provider name'), 'My provider'); typeInto(input('Provider URL'), 'https://example.com'); typeInto(input('Label'), 'Draft')
  addModel(a, 'model')
  act(() => a.button('Add 1 model')!.click())
  await until(() => a.text().includes(message))
  expect(input(label).parentElement?.textContent).toContain(message)
  const previous = input(label).value
  expect(previous).not.toBe('')
  typeInto(input(label), `${previous}x`)
  expect(a.text()).not.toContain(message)
})

it('shows a fieldless refusal above save', async () => {
  const a = await setup('MiniMax', { 'POST /api/aylul/providers': () => Response.json({ code: 'PROFILE_INVALID', message: 'Core refused this provider.' }, { status: 400 }) })
  addModel(a, 'model'); act(() => a.button('Add 1 model')!.click())
  await until(() => a.text().includes('Core refused this provider.'))
  expect(input('model').checked).toBe(true)
})

it('shows an edit refusal after create succeeded but key save failed', async () => {
  const a = await setup('MiniMax', {
    'POST /api/aylul/providers': () => Response.json(provider),
    'PUT /api/aylul/providers/p/key': () => Response.json({ code: 'STORAGE_UNAVAILABLE', message: 'Key failed.' }, { status: 500 }),
    'PUT /api/aylul/providers/p': () => Response.json({ code: 'PROFILE_INVALID', field: 'label', message: 'Label refused.' }, { status: 400 }),
  })
  typeInto(input('API key'), 'secret'); typeInto(input('Label'), 'Draft'); addModel(a, 'model')
  act(() => a.button('Add 1 model')!.click()); await until(() => a.text().includes('Key failed.'))
  act(() => a.button('Add 1 model')!.click()); await until(() => a.text().includes('Label refused.'))
  expect(a.calls.filter(call => call === 'POST /api/aylul/providers')).toHaveLength(1)
  expect(input('Label').value).toBe('Draft')
  typeInto(input('Label'), 'Changed'); expect(a.text()).not.toContain('Label refused.')
})
it('the add button reads Add 1 model, Add 2 models', async () => {
  const a = await setup('MiniMax')
  addModel(a, 'one')
  expect(a.button('Add 1 model')).toBeDefined()
  addModel(a, 'two')
  expect(a.button('Add 2 models')).toBeDefined()
})
