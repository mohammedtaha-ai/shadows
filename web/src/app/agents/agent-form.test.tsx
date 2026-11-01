// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { answers } from '@/test/fake-daemon'
import { startApp, typeInto, until, type TestApp } from '@/test/test-app'

const provider = { slug: 'fixture', name: 'Fixture', base_url: 'https://example.com',
  models: [{ id: 'model', context_window: 200000 }], key_set: true, created_at: 'now', updated_at: 'now' }
const agent = { slug: 'work', name: 'Work', provider: 'fixture', default_model: 'model',
  character_color: 'purple', description: 'Builds things', outfit: 'crown', instructions: '',
  deleted: false, created_at: 'now', updated_at: 'now' }
let app: TestApp | null = null
afterEach(() => { app?.unmount(); app = null; localStorage.clear() })
const field = (name: string) => document.querySelector<HTMLInputElement | HTMLTextAreaElement>(`[aria-label="${name}"]`)!
async function open(slug = 'new', instructions = '', extra = {}) {
  app = await startApp(`/aylul?settings=agents&agent=${slug}`, { ...answers(),
    'GET /api/aylul/providers': [provider],
    'GET /api/aylul/agents': { project_id: 'p1', agents: [{ ...agent, instructions }] },
    'GET /api/aylul/agent-template': { instructions: 'Bundled instructions' },
    'GET /api/aylul/agents/work/extensions': { plugins: [], skills: [], mcp: [] },
    'GET /api/aylul/agents/work/env': [],
    'POST /api/aylul/agents': () => Response.json(agent, { status: 201 }),
    'PUT /api/aylul/agents/work': () => Response.json(agent),
    ...extra,
  })
  await until(() => field('Agent instructions') !== null)
  return app
}

it('a new agent is prefilled from the template and saves outfit and description', async () => {
  const a = await open()
  await until(() => field('Agent instructions').value === 'Bundled instructions')
  typeInto(field('Agent name'), 'New agent')
  typeInto(field('Agent description'), 'Answers questions')
  act(() => document.querySelector<HTMLInputElement>('input[aria-label="Moon"]')!.click())
  act(() => a.button('Create agent')!.click())
  await until(() => a.calls.includes('POST /api/aylul/agents'))
  expect(a.bodies.at(-1)).toMatchObject({ name: 'New agent', description: 'Answers questions', outfit: 'moon' })
  expect(a.bodies.at(-1)).not.toHaveProperty('instructions')
})

it.each(['Saved instructions', ''])('an existing agent shows its instructions or the template: %s', async text => {
  await open('work', text)
  await until(() => field('Agent instructions').value === (text || 'Bundled instructions'))
  expect(field('Agent description').value).toBe('Builds things')
  expect(document.querySelector<HTMLInputElement>('input[aria-label="Crown"]')!.checked).toBe(true)
})

it('untouched large instructions are not sent and changed instructions are sent', async () => {
  const a = await open('work', 'x'.repeat(32768))
  typeInto(field('Agent description'), 'Changed description')
  act(() => a.button('Save agent')!.click())
  await until(() => a.calls.includes('PUT /api/aylul/agents/work'))
  expect(a.bodies.at(-1)).not.toHaveProperty('instructions')
  a.unmount(); app = null
  const b = await open('work', 'Original instructions')
  typeInto(field('Agent instructions'), 'Edited instructions')
  act(() => b.button('Save agent')!.click())
  await until(() => b.calls.includes('PUT /api/aylul/agents/work'))
  expect(b.bodies.at(-1)).toHaveProperty('instructions', 'Edited instructions')
})

it('editing and reverting instructions omits them from the save', async () => {
  const a = await open('work', 'Original instructions')
  typeInto(field('Agent instructions'), 'Changed')
  typeInto(field('Agent instructions'), 'Original instructions')
  act(() => a.button('Save agent')!.click())
  await until(() => a.calls.includes('PUT /api/aylul/agents/work'))
  expect(a.bodies.at(-1)).not.toHaveProperty('instructions')
})

it('a delayed template does not overwrite other edits or allow an early save', async () => {
  let release!: (response: Response) => void
  const response = new Promise<Response>(resolve => { release = resolve })
  const a = await open('new', '', { 'GET /api/aylul/agent-template': () => response })
  typeInto(field('Agent name'), 'My agent')
  expect(a.button('Create agent')!.disabled).toBe(true)
  await act(async () => release(Response.json({ instructions: 'Delayed instructions' })))
  await until(() => field('Agent instructions').value === 'Delayed instructions')
  expect(field('Agent name').value).toBe('My agent')
  expect(a.button('Create agent')!.disabled).toBe(false)
})

it('a template failure is shown and cannot silently create an uninitialized agent', async () => {
  const a = await open('new', '', {
    'GET /api/aylul/agent-template': () => Response.json({ code: 'AYLUL_UNAVAILABLE', message: 'Template is unavailable.' }, { status: 503 }),
  })
  await until(() => a.text().includes('Template is unavailable.'))
  expect(a.button('Create agent')!.disabled).toBe(true)
})

it('the form follows the design order and displays Core profile errors', async () => {
  const a = await open('work', '', {
    'PUT /api/aylul/agents/work': () => Response.json({ code: 'PROFILE_INVALID', message: 'Description is too long.' }, { status: 400 }),
  })
  await until(() => field('Agent instructions').value === 'Bundled instructions')
  const controls = [...document.querySelector('form')!.querySelectorAll('[aria-label]')]
    .map(node => node.getAttribute('aria-label'))
  const order = ['Outfit', 'Character colour', 'Agent name', 'Agent description', 'Provider', 'Default model', 'Agent instructions']
  expect(order.map(label => controls.indexOf(label))).toEqual([...order.map(label => controls.indexOf(label))].sort((a, b) => a - b))
  expect(a.text()).toContain('Changes apply to new and resumed conversations.')
  act(() => a.button('Save agent')!.click())
  await until(() => a.text().includes('Description is too long.'))
  expect(field('Agent description').value).toBe('Builds things')
})
