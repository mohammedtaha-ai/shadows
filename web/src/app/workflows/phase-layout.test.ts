import { expect, it } from 'vitest'
import { planFixture, planTask } from '@/test/contract-fixtures'
import type { Phase, PhaseEndpoint } from '@/api/workflows'
import { layoutTask, phaseNodeId } from './phase-layout'

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
  expect(node?.data.endpoint).toEqual(foreign)
  expect(graph.edges.some(e=>e.source===phaseNodeId(foreign) && e.target===phaseNodeId(source))).toBe(true)
  plan.linked_phases=[{source:foreign,reference:{kind:'plan',plan_id:plan.plan_id,
    version:plan.version,task:1,phase:'a'},target:source,broken:null}]
  const incoming=layoutTask(plan,1)
  expect(incoming.edges.some(e=>e.source===phaseNodeId(source) && e.target===phaseNodeId(foreign))).toBe(true)
})
