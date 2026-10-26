// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { type TestApp, startApp, typeInto, until } from '@/test/test-app'
import { agreementKeys } from '@/api/agreements'
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

it('a stale agreement read cannot overwrite a saved draft', async () => {
  const version = { agreement_id: 'a1', project_id: 'p1', version: 1, revision: 0,
    state: 'Draft', reason: null, writer: { kind: 'User', id: 'local', operation_id: null },
    created_at: '2026-10-04', agreed_at: null, issues: [], content: {
      capability: 'Login', purpose: 'Login', behavior: '', acceptance: ['Works'], parties: [],
      openapi: { openapi: '3.1.0', info: { title: 'Login', version: '1' }, paths: {} },
    } }
  let release!: (value: Response) => void
  let reads = 0
  const saved = { ...version, revision: 1, content: { ...version.content, purpose: 'Saved purpose' } }
  const app = open = await startApp('/projects/p1/agreements?agreement=a1', {
    ...answers(), 'GET /api/projects/p1/agreements': [version],
    'GET /api/projects/p1/agreements/a1': () => ++reads === 1 ? Response.json(version)
      : new Promise<Response>(resolve => { release = resolve }),
    'GET /api/projects/p1/design/parts': { revision: 0, items: [], next: null },
    'PUT /api/projects/p1/agreements/a1': saved,
  })
  const field = () => app.container.querySelector<HTMLTextAreaElement>('textarea[aria-label="Purpose"]')
  await until(() => field() !== null)
  const key = agreementKeys.version('p1', 'a1')
  let pending!: Promise<unknown>
  act(() => { pending = app.queryClient.refetchQueries({ queryKey: key, exact: true }) })
  await until(() => reads === 2)
  typeInto(field()!, 'Saved purpose')
  act(() => app.button('Save Draft')!.click())
  await until(() => app.queryClient.getQueryData<{ revision: number }>(key)?.revision === 1)
  await act(async () => { release(Response.json(version)); await pending })
  expect(app.queryClient.getQueryData(key)).toEqual(saved)
})

it('agreement stream invalidation refreshes the feature read under the same identity', async () => {
  let revision = 0
  const version = () => ({ agreement_id: 'a1', project_id: 'p1', version: 1, revision,
    state: 'Draft', reason: null, writer: { kind: 'User', id: 'local', operation_id: null },
    created_at: '2026-10-04', agreed_at: null, issues: [], content: {
      capability: 'Login', purpose: 'Login', behavior: '', acceptance: [], parties: [],
      openapi: { openapi: '3.1.0', info: { title: 'Login', version: '1' }, paths: {} },
    } })
  const app = open = await startApp('/projects/p1/agreements?agreement=a1', {
    ...answers(), 'GET /api/projects/p1/agreements': () => Response.json([version()]),
    'GET /api/projects/p1/agreements/a1': () => Response.json(version()),
    'GET /api/projects/p1/design/parts': { revision: 0, items: [], next: null },
  })
  const key = agreementKeys.version('p1', 'a1')
  const source = () => app.sources.find(s => s.url.includes('/projects/p1/events'))
  await until(() => source() !== undefined && app.queryClient.getQueryData(key) !== undefined)
  act(() => source()!.caughtUp(0))
  await until(() => !app.queryClient.isFetching({ queryKey: key }))
  revision = 2
  act(() => source()!.durable(1, 'AgreementChanged'))
  await until(() => app.queryClient.getQueryData<{ revision: number }>(key)?.revision === 2)
  expect(app.queryClient.getQueryData(key)).toEqual(version())
  expect(key).toEqual(['projects', 'p1', 'agreement', 'a1', undefined])
})
