// @vitest-environment happy-dom
import {act} from 'react'
import {afterEach,expect,it} from 'vitest'
import type {RoadmapProposal} from '@/api/design'
import {agentEntry} from '@/test/contract-fixtures'
import {answers} from '@/test/fake-daemon'
import {type TestApp,startApp,until} from '../test-app'
import {RoadmapProposalCard} from './roadmap-proposal-card'

let app: TestApp | null = null
afterEach(() => {app?.unmount();app=null})
const pending = (): RoadmapProposal => ({id:'r1',thread_id:'t1',action:'first',outcome_id:null,old:null,
  status:'Pending',outdated:false,created_at:'2026-10-12',
  outcomes:[{key:'login',parent_key:null,parent:null,parts:['part-1'],
    content:{title:'People sign in',intended_result:'A person reaches their projects',
      acceptance:['A wrong password shows an error']}},
  {key:'sso',parent_key:'login',parent:null,parts:['part-1'],
    content:{title:'Single sign-on',intended_result:'Company accounts work',acceptance:[]}}]})
const entry = (card: RoadmapProposal = pending()) =>
  ({...agentEntry('e1','Roadmap proposal'),kind:'RoadmapProposal' as const,card:{...card}})
const part = {revision:1,part:{id:'part-1',revision:1,parent:null,ordinal:0,content:{title:'Backend',
  responsibility:'',design:'',kind:'backend',serves:[],folders:[],files:[],
  contract:{header:'',rules:[]}}},ancestors:[],plans:[]}

it('shows the outcome tree with the titles of the parts it references', async () => {
  const a = app = await startApp('/projects/p1/threads/t1',{...answers({entries:[entry()]}),
    'GET /api/projects/p1/roadmap-proposals':[pending()],
    'GET /api/projects/p1/design/parts/part-1':part,
  })
  await until(() => a.button('Approve roadmap') !== undefined)
  expect(a.text()).toContain('First roadmap')
  expect(a.text()).toContain('People sign in')
  expect(a.text()).toContain('A wrong password shows an error')
  expect(a.text()).toContain('Single sign-on')
  await until(() => a.text().includes('Backend'))
})

it.each(['human','planner:claude-code'])('shows live %s roadmap approval attribution',async agent => {
  const by = {principal:'user:local',agent,request:agent==='human' ? null :
    {thread_id:'t2',entry_id:'request',ordinal:7}}
  const a = app = await startApp('/projects/p1/threads/t1',{...answers({entries:[entry()]}),
    'GET /api/projects/p1/roadmap-proposals':[{...pending(),status:'Approved',approved_by:by}],
  })
  await until(() => a.text().includes(agent==='human' ? 'Approved by you' : 'Approved by the Planner'))
  if (agent!=='human') expect(a.container.querySelector('a[href="/projects/p1/threads/t2#entry-7"]')).not.toBeNull()
})

it('posts one replayable approval and hides the button once approved', async () => {
  expect(RoadmapProposalCard).toBeTypeOf('function')
  let live = pending()
  const a = app = await startApp('/projects/p1/threads/t1',{...answers({entries:[entry()]}),
    'GET /api/projects/p1/roadmap-proposals':() => Response.json([live]),
    'POST /api/projects/p1/roadmap-proposals/r1/approve':() => {
      live={...live,status:'Approved'};return Response.json({revision:1})},
  })
  await until(() => a.button('Approve roadmap') !== undefined)
  act(() => a.button('Approve roadmap')?.click())
  await until(() => a.text().includes('Approved'))
  expect(a.bodies.at(-1)).toMatchObject({command_id:expect.any(String)})
  expect(a.button('Approve roadmap')).toBeUndefined()
})

it.each([['Pending',true],['Superseded',false],['Approved',false]] as const)(
  'hides approval for live %s state (outdated %s)',async (status,outdated) => {
    const a = app = await startApp('/projects/p1/threads/t1',{...answers({entries:[entry()]}),
      'GET /api/projects/p1/roadmap-proposals':[{...pending(),status,outdated}],
    })
    await until(() => a.text().includes(outdated ? 'Outdated' : status))
    expect(a.button('Approve roadmap')).toBeUndefined()
  })

it('shows the old outcome beside a change and keeps the command on refusal', async () => {
  const change: RoadmapProposal = {...pending(),action:'change',outcome_id:'o1',
    outcomes:[pending().outcomes[0]],
    old:{title:'People log in',intended_result:'Old result',acceptance:[]}}
  const a = app = await startApp('/projects/p1/threads/t1',{...answers({entries:[entry(change)]}),
    'GET /api/projects/p1/roadmap-proposals':[change],
    'POST /api/projects/p1/roadmap-proposals/r1/approve':() =>
      Response.json({code:'STAGE_BLOCKED',message:'proposal outdated; propose again'},{status:409}),
  })
  await until(() => a.button('Approve roadmap') !== undefined)
  expect(a.text()).toContain('Change outcome')
  expect(a.text()).toContain('People log in')
  act(() => a.button('Approve roadmap')?.click())
  await until(() => a.text().includes('proposal outdated; propose again'))
  act(() => a.button('Approve roadmap')?.click())
  await until(() => a.bodies.length===2)
  expect(a.bodies[1]).toEqual(a.bodies[0])
})
