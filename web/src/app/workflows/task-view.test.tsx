// @vitest-environment happy-dom
import { act } from 'react'
import { afterEach, expect, it, vi } from 'vitest'
import { answers } from '@/test/fake-daemon'
import { planFixture, planTask } from '@/test/contract-fixtures'
import { type TestApp,startApp,until } from '@/test/test-app'
import { FakeResizeObserver } from '@/test/fake-resize-observer'
let app:TestApp|null=null
afterEach(()=>{app?.unmount();app=null;vi.unstubAllGlobals()})
const page='/projects/p1/workflows/w1'
it('opens phases from the URL and returns to the same graph with selection kept',async()=>{
  vi.stubGlobal('ResizeObserver',FakeResizeObserver)
  const plan=planFixture({tasks:[{...planTask(1,'Deliver'),phases:[{
    id:'build',number:1,title:'Build phase',goal:'Ship',acceptance:[1],waits_for:[],
  }]}],links:[]})
  app=await startApp(`${page}?task=1`,answers({plan}))
  const a=app
  await until(()=>a.button('Back to plan')!==undefined)
  expect(a.text()).toContain('Build phase')
  const graph=a.container.querySelector('[data-main-plan] .react-flow')
  await act(async()=>a.button('Back to plan')?.click())
  await until(()=>a.button('Back to plan')===undefined)
  expect(a.container.querySelector('[data-main-plan] .react-flow')).toBe(graph)
  expect(window.location.search).toContain('selected=1')
})
it('shows no phases with the task acceptance and handles an unavailable task',async()=>{
  app=await startApp(`${page}?task=1`,answers())
  await until(()=>app?.button('Back to plan')!==undefined)
  expect(app.text()).toContain('This task has no phases')
  expect(app.container.querySelector('aside[aria-label="T1"]')?.textContent).toContain('Acceptance')
})
it('names an unavailable task rather than silently showing another task',async()=>{
  app=await startApp(`${page}?task=999`,answers())
  await until(()=>app?.text().includes('Task T999 is unavailable')===true)
  expect(app.button('Back to plan')).toBeDefined()
})
it('opens a foreign phase at its pinned workflow and browser Back returns to the task',async()=>{
  const task={...planTask(1,'Deliver'),phases:[{
    id:'build',number:1,title:'Build phase',goal:'Ship',acceptance:[1],waits_for:[],
  }]}
  const plan=planFixture({tasks:[task],links:[],linked_phases:[{
    source:{plan_id:'plan1',workflow_id:'w1',version:1,plan_title:'Root',task:1,
      phase_id:'build',phase_number:1,title:'Build phase'},
    reference:{kind:'plan',plan_id:'other',version:1,task:3,phase:'db'},
    target:{plan_id:'other',workflow_id:'pinned-v1',version:1,plan_title:'Database',task:3,
      phase_id:'db',phase_number:2,title:'Database phase'},broken:null,
  }]})
  const routes=answers({plan})
  routes['GET /api/workflows/pinned-v1']=planFixture({id:'pinned-v1',plan_id:'other',
    state:'Frozen',version:1,tasks:[{...planTask(3,'Database'),phases:[{
      id:'allocate',number:1,title:'Allocate database',goal:'Allocate',acceptance:[1],waits_for:[],
    },{
      id:'db',number:2,title:'Database phase',goal:'Prepare',acceptance:[1],waits_for:[],
    }]}],links:[]})
  app=await startApp(`${page}?task=1&phases=true`,routes)
  const a=app
  await until(()=>a.button('Open T3 · P2 · Database v1')!==undefined)
  await act(async()=>a.button('Open T3 · P2 · Database v1')?.click())
  await until(()=>a.path().endsWith('/pinned-v1') && a.text().includes('Database phase'))
  expect(a.calls).toContain('GET /api/workflows/pinned-v1')
  expect(a.calls).not.toContain('GET /api/workflows/latest-draft')
  await act(async()=>window.history.back())
  await until(()=>a.path().endsWith('/w1') && a.text().includes('Build phase'))
  await act(async()=>a.button('Back to plan')?.click())
  await until(()=>a.button('Hide phases')!==undefined)
})
it('shows loading while the task plan is pending and then opens its task',async()=>{
  let resolve:((response:Response)=>void)|undefined
  const pending=new Promise<Response>(done=>{resolve=done})
  app=await startApp(`${page}?task=1`,answers({plan:()=>pending}))
  expect(app.text()).toContain('Reading the plan')
  await act(async()=>resolve?.(Response.json(planFixture())))
  await until(()=>app?.button('Back to plan')!==undefined)
})
it('shows a plan read failure at a task URL',async()=>{
  app=await startApp(`${page}?task=1`,answers({plan:()=>Response.json({
    code:'STORAGE_UNAVAILABLE',message:'Plan is offline',
  },{status:503})}))
  await until(()=>app?.text().includes('Plan is offline')===true)
})
it('Escape restores keyboard focus and a refreshed task keeps its open route',async()=>{
  let current=planFixture({tasks:[{...planTask(1,'Deliver'),phases:[{
    id:'build',number:1,title:'Before refresh',goal:'Ship',acceptance:[1],waits_for:[],
  }]}],links:[]})
  app=await startApp(`${page}?task=1`,answers({plan:()=>Response.json(current)}))
  const a=app
  await until(()=>a.button('Back to plan')!==undefined)
  current={...current,revision:2,tasks:[{...current.tasks[0]!,phases:[{
    ...current.tasks[0]!.phases![0]!,title:'After refresh',
  }]}]}
  const source=a.sources.find(s=>new URL(s.url).pathname==='/api/projects/p1/events')
  if (!source) throw new Error('No project stream')
  await act(async()=>{source.caughtUp(0);source.durable(1,'WorkflowEdited',null,{plan_id:'plan1',workflow_id:'w1'})})
  await until(()=>a.text().includes('After refresh'))
  expect(window.location.search).toContain('task=1')
  await act(async()=>a.button('Back to plan')?.dispatchEvent(new KeyboardEvent('keydown',{key:'Escape',bubbles:true})))
  await until(()=>a.button('Back to plan')===undefined)
  expect(document.activeElement).toBe(a.container.querySelector('nav[aria-label="Tasks"] button[aria-pressed="true"]'))
})
it('opens the same task from an expanded frame and the keyboard task list',async()=>{
  const plan=planFixture({tasks:[{...planTask(1,'Deliver'),phases:[{
    id:'build',number:1,title:'Build phase',goal:'Ship',acceptance:[1],waits_for:[],
  }]}],links:[]})
  app=await startApp(page,answers({plan}))
  const a=app
  await until(()=>a.button('Show phases')!==undefined)
  const canvas=a.container.querySelector('[data-main-plan] .react-flow')
  await act(async()=>a.button('Show phases')?.click())
  await until(()=>a.container.querySelector('[data-task-frame="t1"]')!==null)
  expect(a.button('Hide phases')).toBeDefined()
  await act(async()=>a.container.querySelector<HTMLElement>('[data-task-frame="t1"]')?.click())
  await until(()=>a.button('Back to plan')!==undefined)
  expect(window.location.search).toContain('task=1')
  await act(async()=>a.button('Back to plan')?.click())
  await until(()=>a.button('Hide phases')!==undefined)
  const button=a.container.querySelector<HTMLButtonElement>('nav[aria-label="Tasks"] button')
  await act(async()=>button?.click())
  await until(()=>a.button('Back to plan')!==undefined)
  expect(window.location.search).toContain('task=1')
  expect(a.container.querySelector('[data-main-plan] .react-flow')).toBe(canvas)
})
