// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, typeInto, until } from '../test-app'

let app: TestApp | null = null
afterEach(() => { app?.unmount(); app = null })

const base = {
  version: 1,
  parts: [{ name: 'backend', owns: 'All logic.', waivable: false }],
  rules: [{ id: 'S1', text: 'Each service owns its tables.', parts: ['backend'] }],
  contract_template: { rules: ['Trace every claim.'], shape: ['header', 'obligations'] },
}
const fixture = () => ({ ...answers(), 'GET /api/projects/p1/standards': { base, additions: null } })
const field = (a: TestApp, label: string) => a.container.querySelector<HTMLInputElement | HTMLTextAreaElement>(`[aria-label="${label}"]`)!

it('shows the read-only base and saves a project part as a new version', async () => {
  const a = app = await startApp('/projects/p1/workspace?view=standards', {
    ...fixture(),
    'PUT /api/projects/p1/standards/additions': async (request: Request) => {
      const { content } = await request.json()
      return Response.json({ number: 1, content: { rules: content.rules, parts: content.parts }, created_at: '2026-10-10T00:00:00Z' })
    },
  })
  await until(() => a.text().includes('Each service owns its tables.'))
  expect(a.text()).toContain('never waived')
  expect(a.text()).toContain('Trace every claim.')
  expect(a.container.querySelector('input, textarea')).toBeNull()
  act(() => a.button('Add part')?.click())
  typeInto(field(a, 'Part 1 name'), 'billing')
  typeInto(field(a, 'Part 1 owns'), 'Payments.')
  act(() => a.button('Save')?.click())
  await until(() => a.text().includes('version 1'))
  expect(a.button('Save')?.disabled).toBe(true)
  expect(a.bodies.at(-1)).toEqual({ command_id: expect.any(String), content: { rules: [], parts: [{ name: 'billing', owns: 'Payments.' }] } })
})

it('shows refusals and reuses the command for an unchanged retry', async () => {
  let calls = 0
  const a = app = await startApp('/projects/p1/workspace?view=standards', {
    ...fixture(),
    'PUT /api/projects/p1/standards/additions': async (request: Request) => {
      const { content } = await request.json()
      if (++calls === 1) return Response.json({ code: 'INVALID_COMMAND', message: 'Rule P1 refused.' }, { status: 400 })
      return Response.json({ number: 1, content, created_at: '2026-10-10T00:00:00Z' })
    },
  })
  await until(() => a.button('Add rule') !== undefined)
  act(() => a.button('Add rule')?.click())
  typeInto(field(a, 'Rule P1 text'), 'قاعدة عربية  ')
  typeInto(field(a, 'Rule P1 parts'), 'backend, billing')
  expect(field(a, 'Rule P1 text').dir).toBe('auto')
  act(() => a.button('Save')?.click())
  await until(() => a.text().includes('Rule P1 refused.'))
  const first = a.bodies.at(-1)
  act(() => a.button('Save')?.click())
  await until(() => a.text().includes('version 1'))
  expect(a.bodies.at(-1)).toEqual(first)
  expect(a.bodies.at(-1)).toMatchObject({ content: { rules: [{ id: 'P1', text: 'قاعدة عربية  ', parts: ['backend', 'billing'] }] } })
})

it('keeps a successful save when the authoritative refetch fails', async () => {
  let reads = 0
  const a = app = await startApp('/projects/p1/workspace?view=standards', {
    ...fixture(),
    'GET /api/projects/p1/standards': () => ++reads === 1 ? Response.json({ base, additions: null }) : Response.json({ code: 'STORAGE', message: 'Read failed' }, { status: 500 }),
    'PUT /api/projects/p1/standards/additions': async (request: Request) => {
      const { content } = await request.json()
      return Response.json({ number: 1, content, created_at: '2026-10-10T00:00:00Z' })
    },
  })
  await until(() => a.button('Add part') !== undefined)
  act(() => a.button('Add part')?.click())
  typeInto(field(a, 'Part 1 name'), 'billing')
  typeInto(field(a, 'Part 1 owns'), 'Payments.')
  act(() => a.button('Save')?.click())
  await until(() => a.text().includes('version 1') && a.button('Save')?.disabled === true)
  const stream = a.sources.find(s => s.url.includes('/projects/p1/events'))!
  act(() => stream.durable(1, 'ProjectStandardsSaved'))
  await until(() => a.text().includes('Read failed'))
  expect(a.text()).toContain('version 1')
  expect(field(a, 'Part 1 name').value).toBe('billing')
  expect(a.button('Save')?.disabled).toBe(true)
  expect(a.calls.filter(c => c.startsWith('PUT'))).toHaveLength(1)
})

it('keeps a newer journal-refetched version when an older save response arrives', async () => {
  let finishSave: ((response: Response) => void) | undefined
  let latest: unknown = { base, additions: null }
  const a = app = await startApp('/projects/p1/workspace?view=standards', {
    ...fixture(),
    'GET /api/projects/p1/standards': () => Response.json(latest),
    'PUT /api/projects/p1/standards/additions': () => new Promise<Response>(resolve => { finishSave = resolve }),
  })
  await until(() => a.button('Add part') !== undefined)
  act(() => a.button('Add part')?.click())
  typeInto(field(a, 'Part 1 name'), 'billing')
  typeInto(field(a, 'Part 1 owns'), 'Payments.')
  act(() => a.button('Save')?.click())
  await until(() => finishSave !== undefined)
  latest = { base, additions: { number: 2, content: { rules: [], parts: [{ name: 'shipping', owns: 'Deliveries.' }] }, created_at: '2026-10-10T00:00:01Z' } }
  const stream = a.sources.find(s => s.url.includes('/projects/p1/events'))!
  act(() => stream.durable(2, 'ProjectStandardsSaved'))
  await until(() => a.text().includes('version 2'))
  act(() => finishSave!(Response.json({ number: 1, content: { rules: [], parts: [{ name: 'billing', owns: 'Payments.' }] }, created_at: '2026-10-10T00:00:00Z' })))
  await until(() => a.button('Save')?.disabled === false)
  expect(a.text()).toContain('version 2')
  expect(a.text()).not.toContain('version 1')
})
