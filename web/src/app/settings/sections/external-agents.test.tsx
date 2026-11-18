// @vitest-environment happy-dom

import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { projectFixture } from '@/test/contract-fixtures'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, until } from '@/test/test-app'

let app: TestApp | undefined
afterEach(() => { app?.unmount(); app = undefined })

it('issues a token for the checked projects, shows client setup, and revokes it in global settings', async () => {
  const table = answers()
  const connection = {
    id: 'c1', project_ids: ['p1', 'p2'], created_at: '2026-11-18T00:00:00Z', revoked_at: null,
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
  table['DELETE /api/external-connections/c1'] = { ...connection, revoked_at: '2026-11-18T01:00:00Z' }
  const page = await startApp('/projects/p1/settings?settings=external-agents', table)
  app = page
  await until(() => page.checkbox('Second(second)') !== undefined)
  expect(page.button('Connect')?.disabled).toBe(true)
  const form = document.querySelector<HTMLFormElement>('form[aria-label="Connect external agent"]')!
  const checkboxes = [...form.querySelectorAll<HTMLInputElement>('input[type=checkbox]')]
  act(() => checkboxes[0]?.click())
  act(() => checkboxes[1]?.click())
  expect(page.button('Connect')?.disabled).toBe(false)
  act(() => form.requestSubmit())
  await until(() => page.text().includes('single-use-test-token'))
  expect(page.bodies.at(-1)).toMatchObject({ command_id: expect.any(String), project_ids: ['p1', 'p2'] })
  expect(page.text()).toContain('codex mcp add shadows-c1')

  const cli = document.querySelector<HTMLSelectElement>('[aria-label="Connection setup"] select')!
  act(() => {
    cli.value = 'claude'
    cli.dispatchEvent(new Event('change', { bubbles: true }))
  })
  expect(page.text()).toContain('claude mcp add')
  act(() => page.button('Done')?.click())
  expect(page.text()).not.toContain('single-use-test-token')

  await until(() => page.button('Revoke') !== undefined)
  act(() => page.button('Revoke')?.click())
  await until(() => page.text().includes('Revoked'))
  expect(page.calls).not.toContain('GET /api/projects/p1/mcp-grants')
})
