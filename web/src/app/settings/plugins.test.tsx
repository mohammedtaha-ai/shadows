// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { extensionsQuery, agentExtensionsQuery, type Item } from '@/api/extensions'
import { answers } from '@/test/fake-daemon'
import { startApp, until, typeInto, type TestApp, type Answer } from '@/test/test-app'

const plugin: Item = { id: '11111111-1111-4111-8111-111111111111', name: 'git', description: 'Git tools.' }
const skill: Item = { id: '22222222-2222-4222-8222-222222222222', name: 'review', description: 'Review changes.' }
let app: TestApp | null = null
afterEach(() => { app?.unmount(); app = null; window.localStorage.clear() })
const pathInput = () => document.querySelector<HTMLInputElement>('input[aria-label="Folder path"]')!
async function open(overrides: Record<string, Answer> = {}) {
  const a = app = await startApp('/?settings=plugins', { ...answers(), 'GET /api/aylul/extensions': { plugins: [plugin], skills: [skill] }, ...overrides })
  await until(() => a.button('Add') !== undefined)
  return a
}
async function add(a: TestApp) { act(() => a.button('Add')!.click()); await until(() => pathInput() !== null) }
function confirm(a: TestApp) { act(() => a.buttons('Remove').at(-1)!.click()) }
it('the library has tabs and no switches', async () => {
  const a = await open()
  expect(document.querySelector('[role="tab"][aria-selected="true"]')?.textContent).toBe('Plugins')
  expect(a.text()).toContain('git'); expect(a.text()).toContain('Git tools.')
  expect(a.text()).not.toContain('Review changes.')
  act(() => a.button('Skills')!.click())
  expect(a.text()).toContain('Review changes.'); expect(document.querySelector('[role="switch"]')).toBeNull()
  expect(a.calls.filter(call => call === 'GET /api/aylul/extensions')).toHaveLength(1)
  a.queryClient.setQueryData(extensionsQuery.queryKey, { plugins: [], skills: [] })
  await until(() => a.text().includes('No skills yet.'))
  act(() => a.button('Plugins')!.click()); expect(a.text()).toContain('No plugins yet.')
})
it('Add imports a folder on the daemon', async () => {
  let items: Item[] = []
  const a = await open({ 'GET /api/aylul/extensions': () => Response.json({ plugins: [], skills: items }),
    'POST /api/aylul/extensions': () => { items = [skill]; return Response.json(skill, { status: 201 }) },
  })
  act(() => a.button('Skills')!.click()); await add(a)
  typeInto(pathInput(), '   '); act(() => a.button('Add')!.click())
  expect(a.calls).not.toContain('POST /api/aylul/extensions')
  typeInto(pathInput(), 'C:/skills/review'); act(() => a.button('Add')!.click())
  await until(() => a.text().includes('Review changes.') && pathInput() === null)
  expect(a.bodies[0]).toEqual({ command_id: expect.any(String), kind: 'skill', path: 'C:/skills/review' })
  expect(document.querySelector('[role="tab"][aria-selected="true"]')?.textContent).toBe('Skills')
  expect(document.querySelector('input[type="file"]')).toBeNull()
})
it('import retry preserves command identity', async () => {
  let count = 0
  const a = await open({ 'POST /api/aylul/extensions': () => ++count <= 2 ? Response.json({ code: 'STORAGE_UNAVAILABLE', message: 'Import failed.' }, { status: 500 }) : Response.json(plugin, { status: 201 }) })
  await add(a); typeInto(pathInput(), 'C:/plugins/git'); act(() => a.button('Add')!.click())
  await until(() => a.text().includes('Import failed.') && a.button('Add')?.disabled === false)
  expect(pathInput().value).toBe('C:/plugins/git'); expect(count).toBe(1)
  act(() => a.button('Add')!.click()); await until(() => count === 2 && a.button('Add')?.disabled === false)
  expect(a.bodies[1]).toEqual(a.bodies[0])
  typeInto(pathInput(), 'C:/plugins/other'); act(() => a.button('Add')!.click())
  await until(() => count === 3)
  expect((a.bodies[2] as { command_id: string }).command_id).not.toBe((a.bodies[0] as { command_id: string }).command_id)
})
it('Back discards an unsubmitted import', async () => {
  const a = await open(); await add(a); typeInto(pathInput(), 'C:/unsaved')
  act(() => a.button('Back')!.click()); await add(a)
  expect(pathInput().value).toBe(''); expect(a.calls).not.toContain('POST /api/aylul/extensions')
  act(() => pathInput().dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true })))
  await until(() => document.querySelector('[role="dialog"]') === null)
  expect(a.calls).not.toContain('POST /api/aylul/extensions')
})
it('Remove confirms its library-wide effect', async () => {
  let removed = false, request: Request | undefined
  const a = await open({ 'GET /api/aylul/extensions': () => Response.json({ plugins: removed ? [] : [plugin], skills: [] }),
    [`DELETE /api/aylul/extensions/plugin/${plugin.id}`]: (incoming: Request) => { request = incoming; removed = true; return new Response(null, { status: 204 }) },
  })
  act(() => a.button('Remove')!.click())
  expect(a.text()).toContain('Remove git from the library and from every agent?')
  expect(document.querySelectorAll('[role="dialog"]')).toHaveLength(1)
  act(() => a.button('Cancel')!.click()); expect(request).toBeUndefined()
  act(() => a.button('Remove')!.click()); confirm(a)
  await until(() => a.text().includes('No plugins yet.'))
  expect(new URL(request!.url).searchParams.get('command_id')).toBeTruthy(); expect(await request!.text()).toBe('')
})
it('a failed Remove can be retried', async () => {
  const ids: string[] = []; let removed = false
  const a = await open({ 'GET /api/aylul/extensions': () => Response.json({ plugins: removed ? [] : [plugin], skills: [] }),
    [`DELETE /api/aylul/extensions/plugin/${plugin.id}`]: (incoming: Request) => {
      ids.push(new URL(incoming.url).searchParams.get('command_id')!); removed = ids.length > 1
      return removed ? new Response(null, { status: 204 }) : Response.json({ code: 'COMMAND_CONFLICT', message: 'Remove failed.' }, { status: 409 })
    },
  })
  for (const slug of ['ay', 'zed']) a.queryClient.setQueryData(agentExtensionsQuery(slug).queryKey, { plugins: [{ ...plugin, enabled: true }], skills: [] })
  act(() => a.button('Remove')!.click()); confirm(a)
  await until(() => a.text().includes('Remove failed.') && a.buttons('Remove').at(-1)?.disabled === false)
  expect(ids).toHaveLength(1); expect(a.text()).toContain('Git tools.')
  confirm(a); await until(() => a.text().includes('No plugins yet.'))
  expect(ids[1]).toBe(ids[0])
  for (const slug of ['ay', 'zed']) expect(a.queryClient.getQueryState(agentExtensionsQuery(slug).queryKey)?.isInvalidated).toBe(true)
})
it('library load failure is not empty', async () => {
  let fail = true
  const a = app = await startApp('/?settings=plugins', { ...answers(), 'GET /api/aylul/extensions': () => fail ? Response.json({ code: 'STORAGE_UNAVAILABLE', message: 'Library unavailable.' }, { status: 500 }) : Response.json({ plugins: [], skills: [] }) })
  await until(() => a.text().includes('Library unavailable.'))
  expect(a.text()).not.toContain('No plugins yet.')
  fail = false; act(() => a.button('Retry')!.click()); await until(() => a.text().includes('No plugins yet.'))
  expect(a.button('General')?.disabled).toBe(false)
})
