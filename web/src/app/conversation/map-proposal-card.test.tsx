// @vitest-environment happy-dom
import {act} from 'react'
import {afterEach,expect,it} from 'vitest'
import type {MapProposal} from '@/api/design'
import {agentEntry} from '@/test/contract-fixtures'
import {answers} from '@/test/fake-daemon'
import {type TestApp,startApp,until} from '../test-app'
import {MapProposalCard} from './map-proposal-card'

let app: TestApp | null = null
afterEach(() => {app?.unmount();app=null})
const pending = (): MapProposal => ({id:'m1',thread_id:'t1',action:'first',part_id:null,old:null,
  status:'Pending',outdated:false,created_at:'2026-10-12',waivers:[{part:'frontend',reason:'Terminal tool'}],
  parts:[{key:'backend',parent_key:null,parent:null,content:{title:'Backend',responsibility:'Logic',design:'',
    kind:'backend',serves:['G1'],folders:['backend'],files:[{path:'backend/main.rs',content:'fn main() {}'}],
    contract:{header:'Backend contract',rules:['S1']}}}]})
const entry = () => ({...agentEntry('e1','Project map proposal'),kind:'MapProposal' as const,card:{...pending()}})

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
