// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import type { VisionView } from '@/api/design'
import { answers } from '@/test/fake-daemon'
import { type TestApp, startApp, typeInto, until } from '../test-app'
let open: TestApp | null = null
afterEach(() => { open?.unmount(); open = null })
const initial = (): VisionView => ({ revision: 1, content: {
  purpose: [], users: [], goals: [{ id: 'G1', text: 'Sell books', approved_from: 't1', revision: 1 }], boundaries: [], technical_direction: [],
} })
const field = (a: TestApp, label: string) => a.container.querySelector<HTMLInputElement | HTMLTextAreaElement>(`[aria-label="${label}"]`)
const proposal = (id: string, thread_id: string) => ({ id, thread_id, action: 'add', field: 'goals', item_id: null, text: 'Ship', old_text: null, expected_item_revision: null, status: 'Pending', outdated: false, created_at: '2026-10-12T00:00:00Z' })
it('committed saves survive SSE invalidating the concurrent refresh', async () => {
  let vision = initial()
  let app: TestApp
  let inject = false
  app = open = await startApp('/projects/p1/workspace', {...answers(),
    'GET /api/projects/p1/design/vision': async () => {
      if (inject) {
        inject=false
        app.sources.find(s => s.url.includes('/projects/p1/events'))?.emit('durable',JSON.stringify({
          seq:1,kind:'ProjectDesignChanged',operation_id:null,thread_id:null,payload:{project_id:'p1'}}))
        await new Promise(resolve => setTimeout(resolve,10))
      }
      return Response.json(vision)
    },
    'POST /api/projects/p1/design/edits': () => {
      vision={...vision,revision:2,content:{...vision.content,purpose:[{id:'PU1',text:'Saved purpose',revision:1,approved_from:null}]}}
      inject=true
      return Response.json({revision:2})
    },
  })
  await until(() => field(app,'Add purpose') !== null && app.sources.some(s => s.url.includes('/projects/p1/events')))
  typeInto(field(app,'Add purpose')!,'Saved purpose')
  act(() => app.button('Add Purpose')?.click())
  await until(() => app.text().includes('PU1') && !field(app,'Add users')?.disabled)
  expect(app.text()).not.toContain('CancelledError')
  expect(app.button('Reload')).toBeUndefined()
})
it('lists stable items and pending conversation links', async () => {
  const a = open = await startApp('/projects/p1/workspace', { ...answers(),
    'GET /api/projects/p1/design/vision': initial(),
    'GET /api/projects/p1/vision-proposals': [proposal('a','t1'),proposal('b','t1'),proposal('c','t2')],
  })
  await until(() => a.text().includes('G1 · Sell books'))
  expect(a.text()).toContain('3 proposals await your approval')
  expect(a.text()).toContain('2 pending')
  expect(a.container.querySelector('a[href="/projects/p1/threads/t1"]')).not.toBeNull()
  expect(a.container.querySelector('a[href="/projects/p1/threads/t2"]')).not.toBeNull()
})
it('adds exact Arabic text through an item operation', async () => {
  const a = open = await startApp('/projects/p1/workspace', { ...answers(),
    'GET /api/projects/p1/design/vision': initial(), 'POST /api/projects/p1/design/edits': { revision: 2 },
  })
  await until(() => field(a,'Add goals') !== null)
  typeInto(field(a,'Add goals')!, ' هدف عربي  ')
  act(() => a.button('Add Goals')?.click())
  await until(() => a.bodies.length > 0)
  expect(a.bodies.at(-1)).toMatchObject({ expected_revision: 1, ops: [{ kind: 'VisionItemAdd',field:'goals',text:' هدف عربي  ' }] })
})
it('edits and removes by stable id', async () => {
  const a = open = await startApp('/projects/p1/workspace', { ...answers(),
    'GET /api/projects/p1/design/vision': initial(), 'POST /api/projects/p1/design/edits': { revision: 2 },
  })
  await until(() => a.button('Edit G1') !== undefined)
  act(() => a.button('Edit G1')?.click())
  typeInto(field(a,'Edit G1')!, 'New goal')
  act(() => a.button('Save G1')?.click())
  await until(() => a.bodies.length === 1 && a.button('Edit G1') !== undefined)
  expect(a.bodies[0]).toMatchObject({ ops: [{kind:'VisionItemPut',id:'G1',text:'New goal'}] })
  act(() => a.button('Remove G1')?.click())
  await until(() => a.bodies.length === 2)
  expect(a.bodies[1]).toMatchObject({ ops: [{kind:'VisionItemRemove',id:'G1'}] })
})
it('preserves dirty text on conflict and resets it on project switch', async () => {
  let vision = initial()
  const a = open = await startApp('/projects/p1/workspace', { ...answers(),
    'GET /api/projects/p1/design/vision': () => Response.json(vision),
    'POST /api/projects/p1/design/edits': () => {
      vision = {...vision,revision:2,content:{...vision.content,goals:[{...vision.content.goals[0],text:'Elsewhere',revision:2}]}}
      return Response.json({code:'REVISION_CONFLICT',message:'changed',current_revision:2},{status:409})
    },
    'GET /api/projects/p2/threads': [], 'GET /api/projects/p2/workflows': [],
    'GET /api/projects/p2/vision-proposals': [], 'GET /api/projects/p2/design/vision': initial(),
  })
  await until(() => a.button('Edit G1') !== undefined)
  act(() => a.button('Edit G1')?.click())
  typeInto(field(a,'Edit G1')!, 'نصي غير المحفوظ')
  act(() => a.button('Save G1')?.click())
  await until(() => a.button('Reload') !== undefined)
  expect(field(a,'Edit G1')?.value).toBe('نصي غير المحفوظ')
  act(() => a.button('Reload')?.click())
  await until(() => a.text().includes('Elsewhere') && a.button('Edit G1') !== undefined)
  act(() => a.button('Edit G1')?.click())
  typeInto(field(a,'Edit G1')!, 'Another draft')
  const {router} = await import('@/router')
  await act(async () => { await router.navigate({to:'/projects/$projectId/workspace',params:{projectId:'p2'}}) })
  await until(() => a.text().includes('Sell books'))
  expect(field(a,'Edit G1')).toBeNull()
})
it('proposal events refetch pending cards while preserving an item draft', async () => {
  let vision = initial(), proposals: ReturnType<typeof proposal>[] = []
  const a = open = await startApp('/projects/p1/workspace', { ...answers(),
    'GET /api/projects/p1/design/vision': () => Response.json(vision),
    'GET /api/projects/p1/vision-proposals': () => Response.json(proposals),
  })
  await until(() => a.button('Edit G1') !== undefined && a.sources.some(s => s.url.includes('/projects/p1/events')))
  const source = a.sources.find(s => s.url.includes('/projects/p1/events'))!
  act(() => source.emit('caught-up',JSON.stringify({seq:0})))
  act(() => a.button('Edit G1')?.click())
  typeInto(field(a,'Edit G1')!, 'غير محفوظ')
  vision = {...vision,revision:2}; proposals = [proposal('p','t1')]
  act(() => source.emit('durable',JSON.stringify({seq:1,kind:'VisionProposed',operation_id:null,thread_id:'t1',payload:{project_id:'p1',proposal_id:'p'}})))
  await until(() => a.text().includes('1 proposal awaits your approval'))
  expect(field(a,'Edit G1')?.value).toBe('غير محفوظ')
  expect(a.button('Reload')).not.toBeUndefined()
})
