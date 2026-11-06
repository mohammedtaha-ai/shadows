// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, until } from '@/test/test-app'

let app: TestApp | null = null
afterEach(() => { app?.unmount(); app = null })
const part = (id: string, kind: string, serves: string[]) => ({
  id,parent:null,ordinal:0,revision:1,content:{title:id,responsibility:'',design:'',kind,
    folder:id,serves,layers:[{layer:'screens',place:`${id}/screens`,not_used:null}]},
})
const web = part('web','frontend',['G1'])
const mobile = part('mobile','frontend',['G2'])
const fixture = () => ({...answers(),
  'GET /api/projects/p1/design/parts':(r:Request) => Response.json({revision:1,
    items:new URL(r.url).searchParams.has('parent') ? [] : [web,mobile],next:null}),
  'GET /api/projects/p1/design/parts/web':{revision:1,part:web,ancestors:[],plans:[]},
  'GET /api/projects/p1/design/parts/mobile':{revision:1,part:mobile,ancestors:[],plans:[]},
})

it('the_canvas_draws_one_node_per_part', async () => {
  const a = app = await startApp('/projects/p1/workspace?view=map',fixture())
  await until(() => a.container.querySelectorAll('[data-map-part]').length === 2)
  expect(a.container.querySelector('[data-map-part="web"]')?.textContent).toContain('web/screens')
})

it('multiple_frontends_keep_distinct_part_ids', async () => {
  const a = app = await startApp('/projects/p1/workspace?view=map',fixture())
  await until(() => a.container.querySelectorAll('[data-map-part]').length === 2)
  expect([...a.container.querySelectorAll('[data-map-part]')].map(node => node.getAttribute('data-map-part')))
    .toEqual(['web','mobile'])
})

it('permissions_do_not_render_as_actual_dependencies', async () => {
  const a = app = await startApp('/projects/p1/workspace?view=map',fixture())
  await until(() => a.container.querySelector('[data-project-map]') !== null)
  expect(a.container.querySelectorAll('.react-flow__edge')).toHaveLength(0)
  expect(a.text()).toContain('Dependencies appear when elements are recorded')
})

it('served_items_show_only_the_parts_own', async () => {
  const a = app = await startApp('/projects/p1/workspace?view=map',fixture())
  await until(() => a.container.querySelector('[data-map-part="web"]') !== null)
  const node = a.container.querySelector('[data-map-part="web"]')!
  expect(node.textContent).toContain('G1')
  expect(node.textContent).not.toContain('G2')
})

it('map_and_list_preserve_selection_and_browser_back', async () => {
  const a = app = await startApp('/projects/p1/workspace?view=map&part=web',fixture())
  await until(() => a.button('Show list') !== undefined)
  act(() => a.button('Show list')!.click())
  expect(window.location.search).toContain('part=web')
  act(() => a.container.querySelector<HTMLAnchorElement>('a[href="/projects/p1/workspace?view=map&part=mobile"]')!.click())
  await until(() => window.location.search.includes('part=mobile'))
  act(() => window.history.back())
  await until(() => window.location.search.includes('part=web'))
  act(() => a.button('Show canvas')!.click())
  expect(window.location.search).toContain('part=web')
})
