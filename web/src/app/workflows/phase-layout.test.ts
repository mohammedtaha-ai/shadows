import { expect, it } from 'vitest'
import { planFixture, planTask } from '@/test/contract-fixtures'
import type { Phase, PhaseEndpoint } from '@/api/workflows'
import { layoutTask, layoutExpandedPlan, phaseNodeId } from './phase-layout'

const phase = (id:string,number:number,title:string):Phase => ({
  id,number,title,goal:title,acceptance:[1],waits_for:[],
})

it('lays out the selected task and ordered phases with prerequisite arrows', () => {
  const task = {...planTask(1,'Deliver'),phases:[phase('a',1,'Prepare'),phase('b',2,'Build')]}
  const plan = planFixture({tasks:[task],links:[]})
  const graph = layoutTask(plan,1)
  const phases = graph.nodes.filter(n => n.type==='phase')
  expect(phases.map(n=>n.data.phase?.title)).toEqual(['Prepare','Build'])
  expect(graph.edges.some(e=>e.source===phases[0]?.id && e.target===phases[1]?.id)).toBe(true)
  expect(phases[1]?.position.x).toBeGreaterThan(phases[0]?.position.x ?? 0)
})

it('uses exact workflow metadata for a dashed foreign phase and incoming waits', () => {
  const task = {...planTask(1,'Deliver'),phases:[phase('a',1,'Build')]}
  const plan = planFixture({tasks:[task],links:[]})
  const source:PhaseEndpoint = {plan_id:plan.plan_id,workflow_id:plan.id,version:plan.version,
    plan_title:plan.title,task:1,phase_id:'a',phase_number:1,title:'Build'}
  const foreign:PhaseEndpoint = {plan_id:'other',workflow_id:'pinned-v1',version:1,
    plan_title:'Database',task:3,phase_id:'db',phase_number:2,title:'Prepare database'}
  plan.linked_phases=[{source,reference:{kind:'plan',plan_id:'other',version:1,task:3,phase:'db'},
    target:foreign,broken:null}]
  const graph=layoutTask(plan,1)
  const node=graph.nodes.find(n=>n.id===phaseNodeId(foreign))
  expect(node?.type==='phase' && node.data.foreign).toBe(true)
  expect(node?.type==='phase' ? node.data.endpoint : undefined).toEqual(foreign)
  expect(graph.edges.some(e=>e.source===phaseNodeId(foreign) && e.target===phaseNodeId(source))).toBe(true)
  plan.linked_phases=[{source:foreign,reference:{kind:'plan',plan_id:plan.plan_id,
    version:plan.version,task:1,phase:'a'},target:source,broken:null}]
  const incoming=layoutTask(plan,1)
  expect(incoming.edges.some(e=>e.source===phaseNodeId(source) && e.target===phaseNodeId(foreign))).toBe(true)
})

it('expands phase frames while preserving task completion constraints and plain tasks', async () => {
  const {layoutExpandedPlan,boundaryNodeId}=await import('./phase-layout')
  const plan=planFixture({tasks:[
    {...planTask(1,'Build'),phases:[phase('a',1,'Prepare'),phase('b',2,'Build')]},
    planTask(2,'Plain task'), planTask(3,'Final check'),
  ],links:[{task:2,after:1,kind:'needs',label:'API ready',waiting_items:[]},
    {task:1,after:3,kind:'completes_after',label:'Final acceptance',waiting_items:[1]}]})
  const graph=layoutExpandedPlan(plan)
  expect(graph.nodes.find(n=>n.id==='t1')?.type).toBe('task_group')
  expect(graph.nodes.find(n=>n.id==='t2')?.type).toBe('task')
  expect(graph.edges.find(e=>e.data?.label==='API ready')?.source).toBe(boundaryNodeId(1,'complete'))
  expect(graph.edges.find(e=>e.data?.label==='Final acceptance')?.target).toBe(boundaryNodeId(1,'complete'))
  const frames=graph.nodes.filter(n=>n.type==='task_group')
  expect(graph.nodes.filter(n=>n.type==='phase').every(n=>n.parentId===frames[0]?.id)).toBe(true)
})

it('keeps pinned foreign metadata and missing references visible in expanded plans', () => {
  const plan=planFixture({tasks:[{...planTask(1,'Deliver'),phases:[phase('a',1,'Build')]}],links:[]})
  const source:PhaseEndpoint={plan_id:plan.plan_id,workflow_id:plan.id,version:plan.version,
    plan_title:plan.title,task:1,phase_id:'a',phase_number:1,title:'Build'}
  const target:PhaseEndpoint={...source,plan_id:'other',workflow_id:'pinned-v1',
    version:1,plan_title:'Database',task:3,phase_id:'db',title:'Prepare database'}
  plan.linked_phases=[{source,target,reference:{kind:'plan',plan_id:'other',version:1,task:3,phase:'db'}},
    {source,reference:{kind:'local',task:7,phase:'missing'},broken:'Phase was removed'}]
  const graph=layoutExpandedPlan(plan)
  const pinned=graph.nodes.find(n=>n.id===phaseNodeId(target))
  expect(pinned?.type==='phase' && pinned.data.endpoint).toEqual(target)
  const missing=graph.nodes.find(n=>n.type==='phase' && n.data.broken==='Phase was removed')
  expect(missing?.type==='phase' && missing.data.endpoint).toBeUndefined()
  expect(graph.edges.find(e=>e.source===missing?.id)?.data?.broken).toBe(true)
  expect(graph.edges.some(e=>e.source===phaseNodeId(target) && e.target===phaseNodeId(source))).toBe(true)
})

it('lays out 100 tasks and 400 phases with every child inside a non-overlapping frame',async()=>{
  const {layoutExpandedPlan}=await import('./phase-layout')
  const tasks=Array.from({length:100},(_,i)=>({...planTask(i+1,`Task ${i+1}`),
    phases:Array.from({length:4},(_,j)=>phase(`${i}-${j}`,j+1,`Phase ${j+1}`))}))
  const plan=planFixture({tasks,links:Array.from({length:99},(_,i)=>({
    task:i+2,after:i+1,kind:'needs' as const,label:'Ready',waiting_items:[],
  }))})
  const graph=layoutExpandedPlan(plan)
  const groups=graph.nodes.filter(n=>n.type==='task_group')
  expect(groups).toHaveLength(100)
  expect(graph.nodes.filter(n=>n.type==='phase')).toHaveLength(400)
  for(const child of graph.nodes.filter(n=>n.parentId)) {
    const parent=groups.find(n=>n.id===child.parentId)!
    expect(child.position.x).toBeGreaterThanOrEqual(0)
    expect(child.position.y).toBeGreaterThanOrEqual(0)
    expect(child.position.x+(child.width??0)).toBeLessThanOrEqual(parent.width??0)
    expect(child.position.y+(child.height??0)).toBeLessThanOrEqual(parent.height??0)
  }
  for(const [i,a] of groups.entries()) for(const b of groups.slice(i+1)) {
    expect(a.position.x+(a.width??0)<=b.position.x || b.position.x+(b.width??0)<=a.position.x ||
      a.position.y+(a.height??0)<=b.position.y || b.position.y+(b.height??0)<=a.position.y).toBe(true)
  }
},20000)

it('keeps valid interleaved phase dependencies at phase endpoints',async()=>{
  const {layoutExpandedPlan}=await import('./phase-layout')
  const a1=phase('a1',1,'A prepare'),a2=phase('a2',2,'A build')
  const b1=phase('b1',1,'B prepare'),b2=phase('b2',2,'B build')
  a2.waits_for=[{kind:'local',task:2,phase:'b1'}]
  b2.waits_for=[{kind:'local',task:1,phase:'a1'}]
  const plan=planFixture({tasks:[{...planTask(1,'A'),phases:[a1,a2]},
    {...planTask(2,'B'),phases:[b1,b2]}],links:[]})
  const graph=layoutExpandedPlan(plan)
  const waits=graph.edges.filter(e=>e.data?.kind==='phase_wait')
  expect(waits).toHaveLength(2)
  const absoluteX=(id:string)=>{
    const node=graph.nodes.find(n=>n.id===id)!
    return node.position.x+(graph.nodes.find(n=>n.id===node.parentId)?.position.x??0)
  }
  for(const edge of waits) expect(absoluteX(edge.target)).toBeGreaterThan(absoluteX(edge.source))
})
