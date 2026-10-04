// @vitest-environment happy-dom
import { act } from 'react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { createRoot } from 'react-dom/client'
import { afterEach, expect, it, vi } from 'vitest'
import { VisionEditor } from './vision-editor'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, typeInto, until } from '../test-app'

let open: TestApp | null = null
afterEach(() => { open?.unmount(); open = null })

it('workspace keeps text on conflict and clears editor state when switching projects', async () => {
  let vision = { revision: 0, content: { purpose: '', users: '', goals: '', boundaries: '', technical_direction: '' } }
  const app = open = await startApp('/projects/p1/workspace', {
    ...answers(),
    'GET /api/projects/p1/design/vision': () => Response.json(vision),
    'POST /api/projects/p1/design/edits': () => {
      vision = { revision: 1, content: { ...vision.content, purpose: 'رؤية من التبويب الآخر' } }
      return Response.json({ code: 'REVISION_CONFLICT', message: 'changed', current_revision: 1 }, { status: 409 })
    },
    'GET /api/projects/p2/threads': [], 'GET /api/projects/p2/workflows': [],
    'GET /api/projects/p2/design/vision': { revision: 0, content: { ...vision.content, purpose: 'مشروع آخر' } },
  })
  const field = () => app.container.querySelector<HTMLTextAreaElement>('textarea[aria-label="Purpose"]')
  await until(() => field() !== null)
  typeInto(field()!, 'نصي المحلي غير المحفوظ  ')
  act(() => app.button('Save vision')?.click())
  await until(() => app.button('Reload') !== undefined)
  expect(field()?.value).toBe('نصي المحلي غير المحفوظ  ')
  expect(app.bodies.at(-1)).toMatchObject({ expected_revision: 0, ops: [{ kind: 'VisionPut', content: { purpose: 'نصي المحلي غير المحفوظ  ' } }] })
  act(() => app.button('Reload')?.click())
  await until(() => field()?.value === 'رؤية من التبويب الآخر')
  const { router } = await import('@/router')
  await act(async () => { await router.navigate({ to: '/projects/$projectId/workspace', params: { projectId: 'p2' } }) })
  await until(() => field()?.value === 'مشروع آخر')
  expect(app.text()).not.toContain('نصي المحلي غير المحفوظ')
})

it('vision_editor_keeps_unsaved_text_on_conflict', async () => {
  let saved = { revision: 0, content: { purpose: '', users: '', goals: '', boundaries: '', technical_direction: '' } }
  vi.stubGlobal('fetch', async (request: Request) => {
    if (request.method === 'GET') return Response.json(saved)
    const body = await request.json()
    if (body.expected_revision !== saved.revision) {
      return Response.json({ code: 'REVISION_CONFLICT', message: 'changed', current_revision: saved.revision }, { status: 409 })
    }
    saved = { revision: saved.revision + 1, content: body.ops[0].content }
    return Response.json({ revision: saved.revision })
  })
  const tabs = [document.createElement('div'), document.createElement('div')]
  const roots = tabs.map((tab) => { document.body.append(tab); return createRoot(tab) })
  const clients = tabs.map(() => new QueryClient({ defaultOptions: { queries: { retry: false } } }))
  const purpose = (tab: number) => tabs[tab].querySelector<HTMLTextAreaElement>('textarea[aria-label="Purpose"]')!
  const button = (tab: number, text: string) => [...tabs[tab].querySelectorAll('button')].find((b) => b.textContent === text)
  try {
    await act(async () => {
      roots.forEach((root, i) => root.render(<QueryClientProvider client={clients[i]}><VisionEditor projectId="p1" /></QueryClientProvider>))
    })
    await until(() => Boolean(purpose(0) && purpose(1)))
    typeInto(purpose(1), 'محلي من التبويب الثاني  ')
    typeInto(purpose(0), 'محفوظ من الأول')
    act(() => button(0, 'Save vision')?.click())
    await until(() => saved.revision === 1)
    await until(() => button(0, 'Save vision')?.disabled === true)
    expect(purpose(0).value).toBe('محفوظ من الأول')
    act(() => button(1, 'Save vision')?.click())
    await until(() => button(1, 'Reload') !== undefined)
    expect(purpose(1).value).toBe('محلي من التبويب الثاني  ')
    expect(saved.content.purpose).toBe('محفوظ من الأول')
    act(() => button(1, 'Reload')?.click())
    await until(() => purpose(1).value === 'محفوظ من الأول')
  } finally {
    act(() => roots.forEach((root) => root.unmount()))
    tabs.forEach((tab) => tab.remove())
    clients.forEach((client) => client.clear())
    vi.unstubAllGlobals()
  }
})

it('project SSE and reconnect refresh authoritative vision while keeping dirty text', async () => {
  let vision = { revision: 0, content: { purpose: '', users: '', goals: '', boundaries: '', technical_direction: '' } }
  const app = open = await startApp('/projects/p1/workspace', {
    ...answers(), 'GET /api/projects/p1/design/vision': () => Response.json(vision),
  })
  const field = () => app.container.querySelector<HTMLTextAreaElement>('textarea[aria-label="Purpose"]')
  await until(() => field() !== null && app.sources.some((s) => s.url.includes('/projects/p1/events')))
  const source = app.sources.find((s) => s.url.includes('/projects/p1/events'))!
  act(() => source.emit('caught-up', JSON.stringify({ seq: 0 })))
  typeInto(field()!, 'غير محفوظ')
  vision = { revision: 1, content: { ...vision.content, purpose: 'من التبويب الآخر' } }
  act(() => source.emit('durable', JSON.stringify({ seq: 1, kind: 'ProjectDesignChanged', operation_id: null, thread_id: null,
    payload: { project_id: 'p1', revision: 1, changed_parts: [], changed_outcomes: [], vision_changed: true } })))
  await until(() => app.button('Reload') !== undefined)
  expect(field()?.value).toBe('غير محفوظ')
  const before = app.calls.filter((call) => call === 'GET /api/projects/p1/design/vision').length
  act(() => source.emit('caught-up', JSON.stringify({ seq: 1 })))
  await until(() => app.calls.filter((call) => call === 'GET /api/projects/p1/design/vision').length > before)
  expect(field()?.value).toBe('غير محفوظ')
  expect(app.sources.filter((s) => s.url.includes('/projects/p1/events'))).toHaveLength(1)
})
