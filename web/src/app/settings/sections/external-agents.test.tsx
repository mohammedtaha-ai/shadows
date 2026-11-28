// @vitest-environment happy-dom

import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { projectFixture } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, typeInto, until } from '@/test/test-app'

let app: TestApp | undefined
afterEach(() => { app?.unmount(); app = undefined })

it('issues a token for the checked projects, shows client setup, and revokes it in global settings', async () => {
  const table = answers()
  const connection = {
    id: 'c1', name: 'Laptop Codex', project_ids: ['p1', 'p2'], project_revision: 0,
    created_at: '2026-11-18T00:00:00Z', revoked_at: null,
  }
  table['GET /api/projects'] = [
    projectFixture,
    { ...projectFixture, id: 'p2', name: 'Second', slug: 'second' },
    { ...projectFixture, id: 'p3', name: 'Third', slug: 'third' },
  ]
  table['GET /api/external-connections'] = []
  table['POST /api/external-connections'] = () => {
    table['GET /api/external-connections'] = [connection]
    return Response.json({ connection, token: 'single-use-test-token', mcp_url: 'http://127.0.0.1:4318/mcp' })
  }
  const changed = { ...connection, project_ids: ['p2', 'p3'], project_revision: 1 }
  table['PATCH /api/external-connections/c1'] = changed
  table['DELETE /api/external-connections/c1'] = { ...changed, revoked_at: '2026-11-18T01:00:00Z' }
  const page = await startApp('/projects/p1/settings?settings=external-agents', table)
  app = page
  await until(() => page.checkbox('Second(second)') !== undefined)
  expect(page.button('Create connection')?.disabled).toBe(true)
  const form = document.querySelector<HTMLFormElement>('form[aria-label="Create external connection"]')!
  const checkboxes = [...form.querySelectorAll<HTMLInputElement>('input[type=checkbox]')]
  act(() => checkboxes[0]?.click())
  act(() => checkboxes[1]?.click())
  typeInto(form.querySelector<HTMLInputElement>('input[type=text]')!, connection.name)
  expect(page.button('Create connection')?.disabled).toBe(false)
  act(() => form.requestSubmit())
  await until(() => page.text().includes('single-use-test-token'))
  expect(page.bodies.at(-1)).toMatchObject({
    command_id: expect.any(String), name: connection.name, project_ids: ['p1', 'p2'],
  })
  expect(page.text()).toContain('Streamable HTTP')
  expect(page.text()).toContain('Authorization')
  expect(page.text()).not.toContain('mcp add')

  const cli = document.querySelector<HTMLSelectElement>('[aria-label="Connection setup"] select')!
  act(() => {
    cli.value = 'codex'
    cli.dispatchEvent(new Event('change', { bubbles: true }))
  })
  expect(page.text()).toContain('codex mcp add shadows --url')
  act(() => {
    cli.value = 'claude'
    cli.dispatchEvent(new Event('change', { bubbles: true }))
  })
  expect(page.text()).toContain('claude mcp add --transport http --scope user shadows "')
  act(() => page.button('Done')?.click())
  expect(page.text()).not.toContain('single-use-test-token')

  await until(() => page.button('Edit projects') !== undefined)
  const row = [...document.querySelectorAll('[aria-label="Connections"] li')]
    .find(element => element.textContent?.includes(connection.name))!
  act(() => page.button('Edit projects')?.click())
  const edit = row.querySelector<HTMLFormElement>('form')!
  const projects = [...edit.querySelectorAll<HTMLInputElement>('input[type=checkbox]')]
  expect(projects.map(project => project.checked)).toEqual([true, true, false])
  act(() => projects[0]?.click())
  act(() => projects[2]?.click())
  act(() => edit.requestSubmit())
  await until(() => row.querySelector('form') === null)
  expect(page.bodies.at(-1)).toMatchObject({
    command_id: expect.any(String), expected_revision: 0, project_ids: ['p2', 'p3'],
  })
  expect(row.textContent).toContain('Second, Third')
  expect(page.text()).not.toContain('single-use-test-token')

  act(() => page.button('Revoke')?.click())
  await until(() => page.text().includes('Revoked'))
  expect(page.calls).not.toContain('GET /api/projects/p1/mcp-grants')
})

const hookConnection = {
  id: 'c1', name: 'Laptop', project_ids: ['p1'], project_revision: 0,
  created_at: '2026-11-28T00:00:00Z', revoked_at: null,
  wait_url: 'http://127.0.0.1:4318/api/external/task-wait',
}

it('a_connection_row_offers_wait_hooks_for_its_projects', async () => {
  const table = answers()
  table['GET /api/projects'] = [projectFixture]
  table['GET /api/external-connections'] = [hookConnection]
  const page = await startApp('/projects/p1/settings?settings=external-agents', table)
  app = page
  await until(() => page.button('Wait hooks') !== undefined)
  act(() => page.button('Wait hooks')?.click())
  const panel = document.querySelector('[aria-label="Wait hooks"]')!
  expect(panel.textContent).toContain(hookConnection.wait_url)
  expect(panel.textContent).toContain('shadows-wait.')
  expect(panel.textContent).toContain('SHADOWS_TOKEN')
  expect(panel.textContent).toContain('The files hold no token')
  expect(panel.textContent).toContain('.claude')
})

it('a_revoked_connection_offers_no_hooks', async () => {
  const table = answers()
  table['GET /api/projects'] = [projectFixture]
  table['GET /api/external-connections'] = [{ ...hookConnection, revoked_at: '2026-11-28T01:00:00Z' }]
  const page = await startApp('/projects/p1/settings?settings=external-agents', table)
  app = page
  await until(() => page.text().includes('Revoked'))
  expect(page.button('Wait hooks')).toBeUndefined()
})

it('a_project_with_no_folder_offers_no_hook', async () => {
  const table = answers()
  table['GET /api/projects'] = [{ ...projectFixture, directory: null }]
  table['GET /api/external-connections'] = [hookConnection]
  const page = await startApp('/projects/p1/settings?settings=external-agents', table)
  app = page
  await until(() => page.button('Wait hooks') !== undefined)
  act(() => page.button('Wait hooks')?.click())
  expect(page.text()).toContain('This project has no folder recorded')
  expect(document.querySelector('[aria-label="Wait hooks"] pre')).toBeNull()
})
