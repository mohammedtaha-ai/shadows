// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { projectFixture, threadFixture, userEntry } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, choose, menuItem, startApp, typeInto, until } from '../test-app'

const provider = { slug: 'fixture-provider', name: 'Test provider', base_url: 'https://example.com',
  models: [{ id: 'fake-large', context_window: 200000 }, { id: 'fake-small', context_window: 200000 }], key_set: true,
  created_at: 'now', updated_at: 'now' }
const agent = { slug: 'work', name: 'Work', provider: 'fixture-provider', default_model: 'fake-large',
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
  const a = open = await startApp('/aylul?settings=agents&agent=work', table())
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
  const a = open = await startApp('/aylul?settings=providers', { ...table(),
    'POST /api/aylul/providers': () => Response.json(provider, { status: 201 }),
    'PUT /api/aylul/providers/fixture-provider/key': () => Response.json({ name: 'PROVIDER_API_KEY', updated_at: 'now' }),
  })
  await addProvider(a, 'Custom')
  typeInto(input('Provider name'), 'Test provider')
  typeInto(input('Provider URL'), 'https://example.com')
  typeInto(models(), 'fake-large\nfake-small')
  typeInto(input('API key'), 'synthetic-ui-key')
  act(() => a.button('Save provider')!.click())
  await until(() => a.calls.includes('PUT /api/aylul/providers/fixture-provider/key'))
  await until(() => document.querySelector('[aria-label="Saved providers"]') !== null)
  act(() => a.button('Edit')!.click())
  await until(() => input('API key')?.value === '')
  expect(a.text()).not.toContain('synthetic-ui-key')
})

it('models_editor_rejects_duplicates_and_default', async () => {
  const a = open = await startApp('/aylul?settings=providers', table())
  await addProvider(a, 'Custom')
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
  open = await startApp('/aylul?settings=agents&agent=new', table())
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

async function addProvider(a: TestApp, name: string) {
  await until(() => a.button('Add provider') !== undefined)
  act(() => a.button('Add provider')!.click())
  act(() => a.button(name)!.click())
  act(() => a.button('Next')!.click())
  await until(() => input('API key') !== null)
}
async function savePreset(name: string, china = false) {
  const a = open = await startApp('/aylul?settings=providers', { ...table(),
    'POST /api/aylul/providers': () => Response.json(provider, { status: 201 }),
    'PUT /api/aylul/providers/fixture-provider/key': () => Response.json({ name: 'PROVIDER_API_KEY', updated_at: 'now' }),
  })
  await addProvider(a, name)
  expect(input('Provider URL')).toBeNull()
  if (china) {
    const radio = document.querySelector<HTMLInputElement>('input[aria-label="China"]')!
    expect(document.querySelector<HTMLInputElement>('input[aria-label="International"]')?.checked).toBe(true)
    act(() => radio.click())
  }
  typeInto(input('API key'), 'synthetic-ui-key'); typeInto(models(), 'model-one')
  act(() => a.button('Save provider')!.click())
  await until(() => a.calls.includes('POST /api/aylul/providers'))
  return a
}
it('a preset hides the URL and sends it', async () => {
  const a = await savePreset('MiniMax')
  expect(a.bodies).toContainEqual(expect.objectContaining({ name: 'MiniMax', base_url: 'https://api.minimax.io/anthropic' }))
})
it('Custom asks for the URL', async () => {
  const a = open = await startApp('/aylul?settings=providers', table())
  await addProvider(a, 'Custom')
  expect(input('Provider URL').required).toBe(true)
})
it('MiniMax China sends the China URL', async () => {
  const a = await savePreset('MiniMax', true)
  expect(a.bodies).toContainEqual(expect.objectContaining({ base_url: 'https://api.minimaxi.com/anthropic' }))
})
it('Moonshot China sends the China URL', async () => {
  const a = await savePreset('Moonshot (Kimi)', true)
  expect(a.bodies).toContainEqual(expect.objectContaining({ base_url: 'https://api.moonshot.cn/anthropic' }))
})
it('a one-region preset shows no region', async () => {
  const a = open = await startApp('/aylul?settings=providers', table())
  await addProvider(a, 'Anthropic')
  expect(document.querySelector('[aria-label="Region"]')).toBeNull()
})
it('Edit keeps the saved region', async () => {
  const a = open = await startApp('/aylul?settings=providers', { ...table(), 'GET /api/aylul/providers': [{ ...provider, base_url: 'https://api.moonshot.cn/anthropic' }] })
  await until(() => a.button('Edit') !== undefined)
  act(() => a.button('Edit')!.click())
  expect(document.querySelector<HTMLInputElement>('input[aria-label="China"]')?.checked).toBe(true)
  expect(input('Provider URL')).toBeNull()
  expect(input('Provider name')).toBeNull()
})
it('the list names each provider with its model count', async () => {
  const a = open = await startApp('/aylul?settings=providers', { ...table(), 'GET /api/aylul/providers': [
    { ...provider, name: 'MiniMax', models: [{ id: 'one', context_window: 200000 }, { id: 'two', context_window: 200000 }, { id: 'three', context_window: 200000 }] }, { ...provider, slug: 'unset', name: 'OpenAI', key_set: false },
  ] })
  await until(() => a.button('Set up') !== undefined)
  const rows = [...document.querySelectorAll('[aria-label="Saved providers"] li')].map(row => [...row.children].map(child => child.textContent).join(' '))
  expect(rows).toEqual(['MiniMax 3 models connected Edit', 'OpenAI No key not set Set up'])
})
it('closing mid-form saves nothing', async () => {
  const a = open = await startApp('/aylul?settings=providers', table())
  await addProvider(a, 'Custom'); typeInto(input('API key'), 'unsaved-key')
  act(() => input('API key').dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true })))
  await until(() => document.querySelector('[role="dialog"]') === null)
  act(() => a.button('Settings')!.click())
  await until(() => a.button('Providers') !== undefined)
  act(() => a.button('Providers')!.click())
  await until(() => document.querySelector('[aria-label="Saved providers"]') !== null)
  expect(input('API key')).toBeNull()
  expect(a.calls).not.toContain('POST /api/aylul/providers')
})
it('the sidebar has no Providers link', async () => {
  open = await startApp('/aylul', table())
  expect([...document.querySelectorAll('aside a, aside button')].some(element => element.textContent?.trim() === 'Providers')).toBe(false)
})
it('provider deletion confirms inside the settings dialog', async () => {
  const a = open = await startApp('/aylul?settings=providers', { ...table(),
    'DELETE /api/aylul/providers/fixture-provider': () => new Response(null, { status: 204 }),
  })
  await until(() => a.button('Edit') !== undefined)
  act(() => a.button('Edit')!.click())
  act(() => a.button('Delete')!.click())
  expect(document.querySelectorAll('[role="dialog"]')).toHaveLength(1)
  expect(a.calls).not.toContain('DELETE /api/aylul/providers/fixture-provider')
  act(() => a.button('Cancel')!.click())
  expect(a.calls).not.toContain('DELETE /api/aylul/providers/fixture-provider')
  act(() => a.button('Delete')!.click())
  act(() => a.button('Delete provider')!.click())
  await until(() => a.calls.includes('DELETE /api/aylul/providers/fixture-provider'))
})
it("the AYLUL home's Add a provider opens the dialog on Providers", async () => {
  const a = open = await startApp('/aylul', { ...table(), 'GET /api/aylul/providers': [] })
  await until(() => a.button('Add a provider') !== undefined)
  act(() => a.button('Add a provider')!.click())
  await until(() => document.querySelector('[data-slot="dialog-title"]')?.textContent === 'Providers')
  expect(a.path()).toBe('/aylul')
})

it('Agent settings opens the dialog on that agent', async () => {
  const a = open = await startApp('/aylul', table())
  await until(() => a.button('Work') !== undefined)
  act(() => a.button('Work')!.click())
  await until(() => a.button('Agent settings') !== undefined)
  act(() => a.button('Agent settings')!.click())
  await until(() => input('Agent name') !== null)
  expect(document.querySelector('[data-slot="dialog-title"]')?.textContent).toBe('Agents')
  expect(a.button('Back')).toBeDefined()
  expect(input('Agent name').value).toBe('Work')
  expect(a.path()).toBe('/aylul')
})
it('New agent opens an empty form', async () => {
  let agents = [agent]
  const a = open = await startApp('/aylul', { ...table(),
    'GET /api/aylul/agents': () => Response.json({ project_id: 'p1', agents }),
    'POST /api/aylul/agents': async (request: Request) => {
      const value = await request.json(); const saved = { ...agent, ...value, slug: 'new-agent' }
      agents = [...agents, saved]; return Response.json(saved, { status: 201 })
    },
  })
  act(() => a.button('New agent')!.click())
  await until(() => input('Agent name') !== null)
  expect(input('Agent name').value).toBe('')
  typeInto(input('Agent name'), 'New fixture agent')
  act(() => a.button('Create agent')!.click())
  await until(() => document.querySelector('[aria-label="Saved agents"]')?.textContent?.includes('New fixture agent') === true)
  expect(input('Agent name')).toBeNull()
  expect(new URLSearchParams(window.location.search).has('agent')).toBe(false)
  expect(a.path()).toBe('/aylul')
})
it("the list shows each agent's provider and model", async () => {
  const a = open = await startApp('/aylul?settings=agents', table())
  await until(() => a.button('Edit') !== undefined)
  const row = document.querySelector('[aria-label="Saved agents"] li')!
  expect([...row.children].map(element => element.textContent).join(' ')).toBe('Work Test provider fake-large Edit Delete')
})
it('a deleted agent in the URL shows the list with a note', async () => {
  const a = open = await startApp('/aylul?settings=agents&agent=gone', table())
  await until(() => a.text().includes('This agent no longer exists.'))
  expect(document.querySelector('[aria-label="Saved agents"]')).not.toBeNull()
  expect(input('Agent name')).toBeNull()
})
it('closing an unsaved agent form discards the draft', async () => {
  const a = open = await startApp('/aylul?settings=agents&agent=work', table())
  await until(() => input('Agent name') !== null)
  typeInto(input('Agent name'), 'Unsaved name')
  act(() => input('Agent name').dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true })))
  await until(() => document.querySelector('[role="dialog"]') === null)
  act(() => a.button('Settings')!.click())
  await until(() => a.button('Agents') !== undefined)
  act(() => a.button('Agents')!.click())
  await until(() => document.querySelector('[aria-label="Saved agents"]') !== null)
  expect(input('Agent name')).toBeNull()
  expect(a.calls).not.toContain('PUT /api/aylul/agents/work')
})
it('saving and Back return to the Agents list', async () => {
  const a = open = await startApp('/aylul?settings=agents&agent=work', { ...table(),
    'PUT /api/aylul/agents/work': () => Response.json(agent),
  })
  await until(() => input('Agent name') !== null)
  act(() => a.button('Back')!.click())
  await until(() => document.querySelector('[aria-label="Saved agents"]') !== null)
  act(() => a.button('Edit')!.click())
  await until(() => input('Agent name') !== null)
  act(() => a.button('Save agent')!.click())
  await until(() => input('Agent name') === null)
  expect(a.calls).toContain('PUT /api/aylul/agents/work')
  expect(document.querySelector('[aria-label="Saved agents"]')).not.toBeNull()
})
it('agent deletion confirms inline and preserves the current page', async () => {
  let agents = [agent]
  const a = open = await startApp('/aylul?settings=agents', { ...table(),
    'GET /api/aylul/agents': () => Response.json({ project_id: 'p1', agents }),
    'DELETE /api/aylul/agents/work': () => { agents = []; return new Response(null, { status: 204 }) },
  })
  await until(() => a.button('Delete') !== undefined)
  act(() => a.button('Delete')!.click())
  expect(document.querySelectorAll('[role="dialog"]')).toHaveLength(1)
  expect(a.calls).not.toContain('DELETE /api/aylul/agents/work')
  act(() => a.button('Delete agent')!.click())
  await until(() => document.querySelector('[aria-label="Saved agents"]')?.textContent?.includes('Work') === false)
  expect(a.calls).toContain('DELETE /api/aylul/agents/work')
  expect(a.path()).toBe('/aylul')
})
