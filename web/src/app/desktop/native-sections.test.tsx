// @vitest-environment happy-dom
import {afterEach,beforeEach,describe,it,expect,vi} from 'vitest'
import {act} from 'react'
import {createRoot,type Root} from 'react-dom/client'
import {NativeSettings} from './native-settings'
import {ApprovalsPanel} from './approvals-panel'
import {SessionsPanel} from './sessions-panel'
import type {AylulState} from '../../desktop/activity-api'
const {invoke,listen}=vi.hoisted(()=>({invoke:vi.fn(),listen:vi.fn()}))
vi.mock('./companion-settings',()=>({CompanionSettings:()=>null}))
vi.mock('@tauri-apps/api/core',()=>({invoke}))
vi.mock('@tauri-apps/api/event',()=>({listen}))
let host:HTMLDivElement,root:Root
const auth={state:'signed_in',hasEnrollment:true,principal:{accountId:'account',deviceId:'device',sessionId:'session'},providerLogout:null}
async function flush(){await act(async()=>{await new Promise(resolve=>setTimeout(resolve,0))})}
async function click(label:string){const button=[...host.querySelectorAll('button')].find(item=>item.textContent===label);expect(button,label).toBeTruthy();await act(async()=>button!.click());await flush()}
beforeEach(()=>{
 Object.assign(globalThis,{IS_REACT_ACT_ENVIRONMENT:true})
 Object.assign(window,{__TAURI_INTERNALS__:{}})
 host=document.createElement('div');document.body.append(host);root=createRoot(host)
 listen.mockReset().mockResolvedValue(()=>{})
 invoke.mockReset().mockImplementation(async(command:string,args?:Record<string,unknown>)=>{
  if(command==='auth_action')return args?.action==='logout'?{...auth,state:'signed_out',principal:null}:auth
  if(command==='companion_state')return {revision:1,visible:false,renderAllowed:false,paused:false,error:'companion_unavailable'}
  if(command==='desktop_connection_config')return null
  if(command==='cancel_login'||command==='hide_main')return
  throw Error('Unexpected command '+command)
 })
})
afterEach(async()=>{await act(async()=>root.unmount());host.remove();vi.restoreAllMocks();delete (window as unknown as Record<string,unknown>).__TAURI_INTERNALS__})
describe('native sections',()=>{
 it('companion failure leaves account actions and native hide usable',async()=>{
  await act(async()=>root.render(<NativeSettings/>));await flush()
  expect(host.textContent).toContain('unavailable')
  expect(host.querySelector('button')?.disabled).toBe(false)
  await click('Sign out')
  expect(invoke).toHaveBeenCalledWith('auth_action',{action:'logout'})
  expect(invoke.mock.calls.some(([command])=>String(command).includes('assistant'))).toBe(false)
 })
 it('enrollment, global logout and device revoke retain explicit confirmation',async()=>{
  await act(async()=>root.render(<NativeSettings/>));await flush()
  const confirmation=vi.fn(()=>false);Object.assign(window,{confirm:confirmation})
  for(const label of ['Register as a new device','Sign out all sessions','Revoke this device'])await click(label)
  expect(invoke.mock.calls.filter(([command])=>command==='auth_action')).toHaveLength(1)
  confirmation.mockReturnValue(true)
  await click('Register as a new device');await click('Sign out all sessions');await click('Revoke this device')
  expect(invoke.mock.calls.filter(([command])=>command==='auth_action').map(([,args])=>args.action)).toEqual(['status','enroll','logout_all','revoke_device'])
 })
 it('sign-in can be cancelled independently of its pending worker',async()=>{
  invoke.mockImplementation(async(command:string,args?:Record<string,unknown>)=>{
   if(command==='auth_action'&&args?.action==='login')return new Promise(()=>{})
   if(command==='auth_action')return {...auth,state:'signed_out',principal:null}
   if(command==='companion_state')return {revision:1,visible:false,renderAllowed:false,paused:false,error:null}
   if(command==='desktop_connection_config')return null
  })
  await act(async()=>root.render(<NativeSettings/>));await flush()
  await click('Sign in');expect(host.textContent).toContain('Complete sign-in')
  await click('Cancel sign-in');expect(invoke).toHaveBeenCalledWith('cancel_login',undefined)
 })
 it('malformed companion updates show a local error without breaking account controls',async()=>{
  await act(async()=>root.render(<NativeSettings/>));await flush()
  await act(async()=>{listen.mock.calls[0][1]({payload:{revision:'invalid'}})})
  expect(host.textContent).toContain('Unable to synchronize AYLUL')
  await click('Check session');expect(invoke).toHaveBeenCalledWith('auth_action',{action:'status'})
 })
 it('browser renders no native controls and invokes nothing',async()=>{
  delete (window as unknown as Record<string,unknown>).__TAURI_INTERNALS__
  await act(async()=>root.render(<NativeSettings/>));await flush()
  expect(host.textContent).toBe('');expect(invoke).not.toHaveBeenCalled()
 })
 it('local activity preserves project and session context before an answer',async()=>{
  const pending=[{id:'one',kind:'request' as const,sessionId:'session-one',project:'C:/work/first',tool:'Read',target:'a.ts',reason:null,proposedRule:null,at:'2026-10-20',held:true},
   {id:'two',kind:'refusal' as const,sessionId:'session-two',project:'C:/work/second',tool:'Bash',target:'npm test',reason:'refused',proposedRule:'Bash(npm test)',at:'2026-10-20',held:true}]
  const state:AylulState={sessions:[],pending,hooksInstalled:true}
  await act(async()=>root.render(<ApprovalsPanel state={state} refresh={async()=>{}}/>))
  for(const text of ['C:/work/first','session-one','C:/work/second','session-two'])expect(host.textContent).toContain(text)
  expect(host.textContent).not.toContain('Approve rule')
 })
 it('a missing sidecar has a local error without requiring account login',async()=>{
  await act(async()=>root.render(<SessionsPanel state={null} refresh={async()=>{}}/>))
  expect(host.textContent).toContain('local service is not running')
  expect(invoke).not.toHaveBeenCalled()
 })
})
