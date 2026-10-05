// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { type TestApp, startApp, typeInto, until } from '../test-app'
import { answers } from '@/test/fake-daemon'
let open: TestApp | null = null
afterEach(() => { open?.unmount(); open = null })
it('agreement editor preserves unsaved text after a revision conflict', async () => {
  const version = { agreement_id: 'a1', project_id: 'p1', version: 1, revision: 0,
    state: 'Draft', reason: null, writer: { kind: 'User', id: 'local', operation_id: null },
    created_at: '2026-10-04', agreed_at: null, issues: [], content: {
      capability: 'Login', purpose: 'Login', behavior: '', acceptance: ['Works'], parties: [],
      openapi: { openapi: '3.1.0', info: { title: 'Login', version: '1' }, paths: {} },
    } }
  const app = open = await startApp('/projects/p1/agreements?agreement=a1', {
    ...answers(), 'GET /api/projects/p1/agreements': [version],
    'GET /api/projects/p1/agreements/a1': version,
    'GET /api/projects/p1/design/parts': { revision: 0, items: [], next: null },
    'PUT /api/projects/p1/agreements/a1': () => Response.json({
      code: 'REVISION_CONFLICT', message: 'Agreement changed', current_revision: 1,
    }, { status: 409 }),
  })
  const field = () => app.container.querySelector<HTMLTextAreaElement>('textarea[aria-label="Purpose"]')
  await until(() => field() !== null)
  typeInto(field()!, 'دخول آمن محلي')
  act(() => app.button('Save Draft')?.click())
  await until(() => app.text().includes('Agreement changed'))
  expect(field()?.value).toBe('دخول آمن محلي')
  expect(app.bodies.at(-1)).toMatchObject({ expected_revision: 0,
    content: { purpose: 'دخول آمن محلي' } })
})
