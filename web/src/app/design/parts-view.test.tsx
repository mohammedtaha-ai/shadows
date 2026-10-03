// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, typeInto, until } from '../test-app'

let app: TestApp | null = null
afterEach(() => { app?.unmount(); app = null })
const part = (id: string, title = id, parent: string | null = null) => ({ id, parent, ordinal: 0, revision: 1,
  content: { title, responsibility: 'مسؤولية', design: 'تصميم', kind: null } })

// Eager subtree loading, forgotten page boundaries or title-based links fail here.
it('parts_view_loads_only_open_branches', async () => {
  const items = Array.from({ length: 51 }, (_, i) => part(`n${i}`, `قسم ${i}`))
  const a = app = await startApp('/projects/p1/workspace?view=map', {
    ...answers(),
    'GET /api/projects/p1/design/parts': (r: Request) => {
      const query = new URL(r.url).searchParams
      if (query.get('parent') === 'n0') return Response.json({ revision: 1, items: [part('child', 'ابن', 'n0')], next: null })
      if (query.has('parent')) return Response.json({ revision: 1, items: [], next: null })
      return Response.json({ revision: 1, items: query.has('after') ? items.slice(50) : items.slice(0, 50), next: query.has('after') ? null : 'n49' })
    },
    'GET /api/projects/p1/design/parts/n0': { revision: 1, part: items[0], ancestors: [], plans: [] },
  })
  await until(() => a.text().includes('قسم 49'))
  expect(a.text()).not.toContain('قسم 50')
  act(() => a.button('Load more')?.click())
  await until(() => a.text().includes('قسم 50'))
  expect(a.container.querySelectorAll('[data-part-id]').length).toBe(51)
  expect(a.calls.filter(c => c.includes('/design/parts')).length).toBe(2)
  act(() => a.button('Expand قسم 0')?.click())
  await until(() => a.text().includes('ابن'))
  expect(a.calls.filter(c => c.includes('/design/parts')).length).toBe(3)
})

// Dirty editors must not silently overwrite or erase work on invalidation.
it('part deep link follows identity after move and keeps stale input until Reload', async () => {
  let saved = { revision: 1, part: part('child', 'ابن', 'old'), ancestors: [part('old', 'الأب')], plans: [] }
  const a = app = await startApp('/projects/p1/workspace?view=map&part=child', {
    ...answers(),
    'GET /api/projects/p1/design/parts': (r: Request) => Response.json({ revision: 1, items: new URL(r.url).searchParams.get('parent') === 'child' ? [part('grandchild', 'حفيدة', 'child')] : [part('old', 'الأب')], next: null }),
    'GET /api/projects/p1/design/parts/child': () => Response.json(saved),
    'POST /api/projects/p1/design/edits': () => {
      saved = { revision: 2, part: { ...saved.part, content: { ...saved.part.content, title: 'جديد' }, parent: 'new' }, ancestors: [part('new', 'أب آخر')], plans: [] }
      return Response.json({ code: 'REVISION_CONFLICT', message: 'changed', current_revision: 2 }, { status: 409 })
    },
  })
  const title = () => a.container.querySelector<HTMLInputElement>('input[aria-label="Part title"]')
  await until(() => title()?.value === 'ابن')
  await until(() => a.text().includes('حفيدة'))
  expect(a.text()).toContain('الأب')
  typeInto(title()!, 'نصي المحلي')
  act(() => a.button('Save part')?.click())
  await until(() => a.button('Reload') !== undefined)
  expect(title()?.value).toBe('نصي المحلي')
  act(() => a.button('Reload')?.click())
  await until(() => title()?.value === 'جديد')
  expect(a.text()).toContain('أب آخر')
  expect(window.location.search).toContain('part=child')
})

it('a create retry keeps its command and revision after an unrelated refetch', async () => {
  let revision = 0
  let writes = 0
  const a = app = await startApp('/projects/p1/workspace?view=map', {
    ...answers(),
    'GET /api/projects/p1/design/parts': () => Response.json({ revision, items: [], next: null }),
    'POST /api/projects/p1/design/edits': () => {
      writes++; revision = 1
      return writes === 1 ? Response.json({ code: 'STORAGE_UNAVAILABLE', message: 'response lost' }, { status: 503 }) : Response.json({ revision: 1 })
    },
  })
  act(() => a.button('Create part')?.click())
  await until(() => a.container.querySelector('input[aria-label="Part title"]') !== null)
  typeInto(a.container.querySelector<HTMLInputElement>('input[aria-label="Part title"]')!, 'قسم جديد')
  act(() => a.button('Create')?.click())
  await until(() => a.text().includes('response lost'))
  await act(async () => { await a.queryClient.invalidateQueries({ queryKey: ['projects', 'p1', 'design'] }) })
  await act(async () => { await new Promise(resolve => setTimeout(resolve, 20)) })
  act(() => a.button('Create')?.click())
  await until(() => writes === 2)
  const edits = a.bodies as { command_id: string; expected_revision: number; ops: unknown[] }[]
  expect(edits[0].expected_revision).toBe(0)
  expect(edits[1]).toEqual(edits[0])
})
