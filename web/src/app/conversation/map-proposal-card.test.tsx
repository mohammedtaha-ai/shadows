// @vitest-environment happy-dom
import {act} from 'react'
import {afterEach,expect,it} from 'vitest'
import type {MapProposal} from '@/api/design'
import {agentEntry} from '@/test/contract-fixtures'
import {answers} from '@/test/fake-daemon'
import {type TestApp,startApp,until} from '@/test/test-app'
import {MapProposalCard} from './map-proposal-card'

let app: TestApp | null = null
afterEach(() => {app?.unmount();app=null})
const pending = (): MapProposal => ({id:'m1',thread_id:'t1',action:'first',part_id:null,old:null,
  status:'Pending',outdated:false,created_at:'2026-10-12',waivers:[{part:'frontend',reason:'Terminal tool'}],
  parts:[{key:'backend',parent_key:null,parent:null,content:{title:'Backend',responsibility:'Logic',design:'',
    kind:'backend',serves:['G1'],folder:'backend',layers:[],
    contract:{header:'Backend contract',rules:['S1']}}}]})
const entry = () => ({...agentEntry('e1','Project map proposal'),kind:'MapProposal' as const,card:{...pending()}})

it('the_map_card_shows_each_parts_layers', async () => {
  const proposed = pending()
  proposed.parts[0].content.layers = [{layer:'domain',place:'backend/Domain',not_used:null}]
  const a = app = await startApp('/projects/p1/threads/t1',{...answers({entries:[{
    ...entry(),card:{...proposed},
  }]}),'GET /api/projects/p1/map-proposals':[proposed]})
  await until(() => a.text().includes('backend/Domain'))
  expect(a.text()).toContain('domain')
})

it('the_map_card_links_a_grouping_parts_children', async () => {
  const proposed = pending()
  const content = proposed.parts[0].content
  proposed.parts = [
    {key:'clients',parent_key:null,parent:null,content:{...content,title:'Clients',kind:'frontend',folder:'clients'}},
    {key:'web',parent_key:'clients',parent:null,content:{...content,title:'Web',kind:'frontend',folder:'clients/web'}},
    {key:'tools',parent_key:null,parent:null,content:{...content,title:'Tools',kind:null,folder:'tools'}},
    {key:'cli',parent_key:'tools',parent:null,content:{...content,title:'Cli',kind:null,folder:'tools/cli'}},
  ]
  const a = app = await startApp('/projects/p1/threads/t1',{...answers({entries:[{
    ...entry(),card:{...proposed},
  }]}),'GET /api/projects/p1/map-proposals':[proposed]})
  await until(() => a.text().includes('Groups:'))
  const lines = [...a.container.querySelectorAll('p')].filter(p => p.textContent?.startsWith('Groups:'))
  expect(lines).toHaveLength(1)
  expect(lines[0].querySelector('a[href="#map-part-web"]')?.textContent).toBe('Web')
})

it.each(['human','planner:claude-code'])('shows live %s map approval attribution',async agent => {
  const by = {principal:'user:local',agent,request:agent==='human' ? null :
    {thread_id:'t2',entry_id:'request',ordinal:7}}
  const a = app = await startApp('/projects/p1/threads/t1',{...answers({entries:[entry()]}),
    'GET /api/projects/p1/map-proposals':[{...pending(),waivers:agent==='human' ? pending().waivers : [],
      status:'Approved',approved_by:by}],
  })
  await until(() => a.text().includes(agent==='human' ? 'Approved by you' : 'Approved by the Planner'))
  if (agent!=='human') expect(a.container.querySelector('a[href="/projects/p1/threads/t2#entry-7"]')).not.toBeNull()
})

it('shows waiver consent and posts one replayable approval', async () => {
  expect(MapProposalCard).toBeTypeOf('function')
  let live = pending()
  const a = app = await startApp('/projects/p1/threads/t1',{...answers({entries:[entry()]}),
    'GET /api/projects/p1/map-proposals':() => Response.json([live]),
    'POST /api/projects/p1/map-proposals/m1/approve':() => {live={...live,status:'Approved'};return Response.json({revision:1})},
  })
  await until(() => a.button('Approve map and waivers') !== undefined)
  expect(a.text()).toContain('Without frontend')
  expect(a.text()).toContain('Terminal tool')
  act(() => a.button('Approve map and waivers')?.click())
  await until(() => a.text().includes('Approved'))
  expect(a.bodies.at(-1)).toMatchObject({command_id:expect.any(String)})
  expect(a.button('Approve map and waivers')).toBeUndefined()
})

it.each([['Pending',true],['Superseded',false],['Approved',false]] as const)(
  'hides approval for live %s state (outdated %s)',async (status,outdated) => {
    const a = app = await startApp('/projects/p1/threads/t1',{...answers({entries:[entry()]}),
      'GET /api/projects/p1/map-proposals':[{...pending(),status,outdated}],
    })
    await until(() => a.text().includes(outdated ? 'Outdated' : status))
    expect(a.button('Approve map and waivers')).toBeUndefined()
  })

it('keeps an unchanged command on refusal and opens a contract beside the card',async () => {
  const a = app = await startApp('/projects/p1/threads/t1',{...answers({entries:[entry()]}),
    'GET /api/projects/p1/map-proposals':[pending()],
    'POST /api/projects/p1/map-proposals/m1/approve':() => Response.json({code:'STAGE_BLOCKED',message:'unserved goals: G4'},{status:409}),
  })
  await until(() => a.button('Approve map and waivers') !== undefined)
  act(() => a.button('Approve map and waivers')?.click())
  await until(() => a.text().includes('unserved goals: G4'))
  act(() => a.button('Approve map and waivers')?.click())
  await until(() => a.bodies.length===2)
  expect(a.bodies[1]).toEqual(a.bodies[0])
  act(() => a.button('View contract')?.click())
  await until(() => a.container.querySelector('[role="dialog"]') !== null)
  expect(a.container.querySelector('[role="dialog"]')?.textContent).toContain('Backend contract')
})

it('shows a move when a change card has a new parent, and none when unchanged',async () => {
  const change = {...pending(),action:'change' as const,part_id:'backend',old:pending().parts[0].content,old_parent:null,
    parts:[{...pending().parts[0],parent:'core'}]}
  const part = {part:{content:{title:'Core'}},ancestors:[],plans:[],revision:1}
  const a = app = await startApp('/projects/p1/threads/t1',{...answers({entries:[{...entry(),card:{...change}}]}),
    'GET /api/projects/p1/map-proposals':[change],'GET /api/projects/p1/design/parts/core':part})
  await until(() => a.text().includes('Moves under: Core'))
  app.unmount()
  const top = {...change,old_parent:'core',parts:[{...change.parts[0],parent:null}]}
  const b = app = await startApp('/projects/p1/threads/t1',{...answers({entries:[{...entry(),card:{...top}}]}),
    'GET /api/projects/p1/map-proposals':[top]})
  await until(() => b.text().includes('Moves to the top level'))
  app.unmount()
  const same = {...change,old_parent:'core'}
  const c = app = await startApp('/projects/p1/threads/t1',{...answers({entries:[{...entry(),card:{...same}}]}),
    'GET /api/projects/p1/map-proposals':[same],'GET /api/projects/p1/design/parts/core':part})
  await until(() => c.text().includes('Change part'))
  expect(c.text()).not.toContain('Moves')
})
