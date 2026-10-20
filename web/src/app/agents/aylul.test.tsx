// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { projectFixture, threadFixture, userEntry } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, choose, menuItem, startApp, typeInto, until } from '../test-app'

const provider = { slug: 'test', name: 'Test provider', base_url: 'https://example.com',
  models: ['fake-large', 'fake-small'], context_window: 200000, key_set: true,
  created_at: 'now', updated_at: 'now' }
const agent = { slug: 'work', name: 'Work', provider: 'test', default_model: 'fake-large',
  deleted: false, created_at: 'now', updated_at: 'now' }
const thread = { ...threadFixture, harness: 'aylul', profile: 'work', title: 'Work chat' }
const table = () => ({ ...answers(),
  'GET /api/projects': [{ ...projectFixture, assistant: true, allowed_modes: { ...projectFixture.allowed_modes, aylul: ['acceptEdits', 'plan', 'bypassPermissions'] } }],
  'GET /api/projects/p1/threads': [thread],
  'GET /api/aylul/providers': [provider],
  'GET /api/aylul/agents': { project_id: 'p1', agents: [agent] },
  'GET /api/aylul/agents/work/threads': [thread],
  'GET /api/threads/t1': thread,
})
let open: TestApp | null = null
afterEach(() => { open?.unmount(); open = null; window.localStorage.clear() })
const input = (name: string) => document.querySelector<HTMLInputElement>(`input[aria-label="${name}"]`)!
const models = () => document.querySelector<HTMLTextAreaElement>('textarea[aria-label="Models"]')!

it('switcher_shows_both_with_check_on_current', async () => {
  const a = open = await startApp('/', table())
  await until(() => a.button('Workspace') !== undefined)
  act(() => a.button('Workspace')!.click())
  await until(() => menuItem('AYLUL') !== undefined)
  expect(menuItem('Shadows')?.getAttribute('aria-checked')).toBe('true')
  expect(menuItem('AYLUL')?.textContent).toContain('Your agents and their conversations')
})

it('switcher_choice_survives_reload', async () => {
  let a = open = await startApp('/', table())
  act(() => a.button('Workspace')!.click())
  await until(() => menuItem('AYLUL') !== undefined)
  act(() => menuItem('AYLUL')!.click())
  await until(() => a.path() === '/aylul')
  a.unmount(); open = null
  a = open = await startApp('/', table())
  await until(() => a.button('Workspace')?.textContent?.includes('AYLUL') === true)
})

it('aylul_thread_url_opens_aylul_workspace', async () => {
  const a = open = await startApp('/projects/p1/threads/t1', table())
  await until(() => a.button('Workspace')?.textContent?.includes('AYLUL') === true)
  expect(a.button('New agent')).toBeDefined()
})

it('switching_navigates_to_that_workspace', async () => {
  const a = open = await startApp('/aylul/agents/work/settings', table())
  await until(() => a.button('Workspace') !== undefined)
  act(() => a.button('Workspace')!.click())
  await until(() => menuItem('Shadows') !== undefined)
  act(() => menuItem('Shadows')!.click())
  await until(() => a.path() === '/')
})

it('shadows_workspace_has_no_aylul_entry', async () => {
  const a = open = await startApp('/', table())
  await until(() => a.calls.includes('GET /api/projects'))
  expect([...document.querySelectorAll('nav a')].some(l => l.textContent?.trim() === 'AYLUL')).toBe(false)
})

it('agent_list_shows_conversations_per_agent', async () => {
  const a = open = await startApp('/aylul', table())
  await until(() => a.button('Work') !== undefined)
  act(() => a.button('Work')!.click())
  await until(() => a.text().includes('Work chat'))
  expect(document.querySelector('a[href="/projects/p1/threads/t1"]')).not.toBeNull()
})

it('provider_form_saves_and_clears_key', async () => {
  const a = open = await startApp('/aylul/providers', { ...table(),
    'POST /api/aylul/providers': () => Response.json(provider, { status: 201 }),
    'PUT /api/aylul/providers/test/key': () => Response.json({ name: 'PROVIDER_API_KEY', updated_at: 'now' }),
  })
  await until(() => input('Provider name') !== null)
  typeInto(input('Provider name'), 'Test provider')
  typeInto(input('Provider URL'), 'https://example.com')
  typeInto(models(), 'fake-large\nfake-small')
  typeInto(input('API key'), 'synthetic-ui-key')
  act(() => a.button('Save provider')!.click())
  await until(() => a.calls.includes('PUT /api/aylul/providers/test/key'))
  await until(() => input('API key').value === '')
  expect(a.text()).not.toContain('synthetic-ui-key')
})

it('models_editor_rejects_duplicates_and_default', async () => {
  const a = open = await startApp('/aylul/providers', table())
  await until(() => input('Provider name') !== null)
  typeInto(input('Provider name'), 'Provider')
  typeInto(input('Provider URL'), 'https://example.com')
  for (const value of ['fake-large\nfake-large', 'default']) {
    typeInto(models(), value)
    act(() => a.button('Save provider')!.click())
    await until(() => a.text().includes('Models must'))
  }
  expect(a.calls).not.toContain('POST /api/aylul/providers')
})

it('agent_form_lists_provider_models', async () => {
  open = await startApp('/aylul/agents/new', table())
  await until(() => document.querySelector('select[aria-label="Default model"] option') !== null)
  expect([...document.querySelectorAll('select[aria-label="Default model"] option')]
    .map(o => o.textContent)).toEqual(['fake-large', 'fake-small'])
})

it('first_send_keeps_chosen_model', async () => {
  const a = open = await startApp('/aylul/agents/work/new', { ...table(),
    'POST /api/projects/p1/threads': () => Response.json(thread),
  })
  await until(() => a.button('Model') !== undefined)
  await choose(a, 'Model', 'fake-small')
  typeInto(document.querySelector<HTMLTextAreaElement>('textarea[aria-label="Message"]')!, 'hello')
  act(() => a.button('Send')!.click())
  await until(() => a.calls.includes('POST /api/threads/t1/turns'))
  expect(a.bodies).toContainEqual(expect.objectContaining({ profile: 'work', harness: 'aylul' }))
  expect(a.bodies).toContainEqual(expect.objectContaining({ model: 'fake-small' }))
  expect(a.calls.indexOf('PUT /api/threads/t1/session/model'))
    .toBeLessThan(a.calls.indexOf('POST /api/threads/t1/turns'))
})

it('deleted_agent_conversation_reads', async () => {
  const a = open = await startApp('/projects/p1/threads/t1', { ...table(),
    'GET /api/aylul/agents': { project_id: 'p1', agents: [] },
    'GET /api/threads/t1/entries': [{ ...userEntry, body: 'Existing history' }],
    'POST /api/threads/t1/session': () => Response.json({ code: 'AYLUL_PROFILE_NOT_FOUND',
      message: 'Deleted agent' }, { status: 404 }),
  })
  await until(() => a.text().includes('Existing history') && document.querySelector('main header')?.textContent?.includes('Deleted agent') === true)
  expect(document.querySelector('main header')?.textContent).toContain('Deleted agent')
})

it('empty_states_guide_to_provider_then_agent', async () => {
  let a = open = await startApp('/aylul', { ...table(), 'GET /api/aylul/providers': [],
    'GET /api/aylul/agents': { project_id: 'p1', agents: [] } })
  await until(() => a.text().includes('Add a provider first'))
  a.unmount(); open = null
  a = open = await startApp('/aylul', { ...table(),
    'GET /api/aylul/agents': { project_id: 'p1', agents: [] } })
  await until(() => a.text().includes('Add an agent to start'))
})
