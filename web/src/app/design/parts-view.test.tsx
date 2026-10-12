// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { answers as baseAnswers } from '@/test/fake-daemon'
import type { DesignEdit, PartView } from '@/api/design'
import { type TestApp, startApp, typeInto, until } from '../test-app'

let app: TestApp | null = null

it('shows mandatory kinds added after map approval', async () => {
  const a = app = await startApp('/projects/p1/workspace?view=map', {
    ...answers(),
    'GET /api/projects/p1/stage': {stage:'structure',missing:['billing'],drift:[]},
  })
  await until(() => a.text().includes('Missing mandatory parts: billing'))
})
const answers = () => ({ ...baseAnswers(),
  'GET /api/projects/p1/stage': {stage:'structure',missing:[],drift:[]} })
afterEach(() => { app?.unmount(); app = null })
const part = (id: string, title = id, parent: string | null = null) => ({ id, parent, ordinal: 0, revision: 1,
  content: { title, responsibility: 'مسؤولية', design: 'تصميم', kind: null } })

it('hides creation before the first map and shows drift with actionable proposals', async () => {
  const a = app = await startApp('/projects/p1/workspace?view=map', {
    ...answers(), 'GET /api/projects/p1/stage': {stage:'map',missing:['database'],drift:['G4 is served by no part']},
    'GET /api/projects/p1/map-proposals': [{id:'m1',thread_id:'t1',action:'first',status:'Pending',outdated:false}],
  })
  await until(() => a.text().includes('The first map comes from the Planner'))
  expect(a.button('Create part')).toBeUndefined()
  expect(a.text()).toContain('G4 is served by no part')
  expect(a.text()).toContain('1 map proposal awaits your approval')
  expect(a.container.querySelector('a[href="/projects/p1/threads/t1"]')).not.toBeNull()
})

it('edits served goals and normalised folder input without dropping starter files', async () => {
  const saved = {revision:1,part:{...part('n1'),content:{...part('n1').content,serves:['G1'],folders:['src'],
    files:[{path:'src/main.rs',content:'fn main() {}'}],contract:{header:'Core contract',rules:['S1']}}},ancestors:[],plans:[]}
  const a = app = await startApp('/projects/p1/workspace?view=map&part=n1', {...answers(),
    'GET /api/projects/p1/design/parts/n1':saved,
    'GET /api/projects/p1/design/vision':{revision:1,content:{purpose:[],users:[],goals:[{id:'G1',text:'Read',revision:1,approved_from:null}],boundaries:[],technical_direction:[]}},
    'POST /api/projects/p1/design/edits':() => Response.json({code:'STAGE_BLOCKED',message:'missing part: backend'},{status:409}),
  })
  await until(() => a.container.querySelector('[aria-label="Part folders"]') !== null)
  typeInto(a.container.querySelector<HTMLTextAreaElement>('[aria-label="Part folders"]')!,'src\nlib')
  act(() => a.button('Save part')?.click())
  await until(() => a.text().includes('missing part: backend'))
  const body = a.bodies.at(-1) as DesignEdit
  expect(body.ops[0]).toMatchObject({kind:'PartPut',content:{folders:['src','lib'],serves:['G1'],files:saved.part.content.files}})
})

it('part save replays the committed command after its follow-up read fails', async () => {
  let saved: PartView = { revision: 1, part: part('n1', 'قسم'), ancestors: [], plans: [] }
  let failReads = false
  let committed: DesignEdit | undefined
  const a = app = await startApp('/projects/p1/workspace?view=map&part=n1', {
    ...answers(),
    'GET /api/projects/p1/design/parts': { revision: 1, items: [saved.part], next: null },
    'GET /api/projects/p1/design/parts/n1': () => failReads
      ? Response.json({ code: 'STORAGE_UNAVAILABLE', message: 'read unavailable' }, { status: 503 })
      : Response.json(saved),
    'POST /api/projects/p1/design/edits': async (request: Request) => {
      const body = await request.json() as DesignEdit
      if (committed) return JSON.stringify(body) === JSON.stringify(committed)
        ? Response.json({ revision: 2 })
        : Response.json({ code: 'REVISION_CONFLICT', message: 'already committed', current_revision: 2 }, { status: 409 })
      committed = body
      const put = body.ops.find(op => op.kind === 'PartPut')
      if (put?.kind !== 'PartPut') throw new Error('expected a part edit')
      saved = { ...saved, revision: 2, part: { ...saved.part, revision: 2, content: put.content } }
      failReads = true
      return Response.json({ revision: 2 })
    },
  })
  const title = () => a.container.querySelector<HTMLInputElement>('input[aria-label="Part title"]')
  await until(() => title()?.value === 'قسم')
  typeInto(title()!, 'تعديل محفوظ  ')
  act(() => a.button('Save part')?.click())
  await until(() => a.text().includes('read unavailable') && a.button('Save part')?.disabled === false)
  expect(title()?.value).toBe('تعديل محفوظ  ')
  failReads = false
  act(() => a.button('Save part')?.click())
  await until(() => a.bodies.length === 2)
  expect(a.bodies[1]).toEqual(a.bodies[0])
  await until(() => a.button('Save part')?.disabled === true && !a.text().includes('read unavailable'))
  expect(title()?.value).toBe('تعديل محفوظ  ')
  expect(a.text()).not.toContain('already committed')
})

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
