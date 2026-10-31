// @vitest-environment happy-dom
import {act} from 'react'
import {afterEach,it,expect,vi} from 'vitest'
import {threadFixture,projectFixture,claudeHarness,runningOperation} from '@/test/contract-fixtures'
import {answers} from '@/test/fake-daemon'
import {startApp,typeInto,until,type TestApp} from '@/test/test-app'
let app:TestApp|null=null
afterEach(()=>{app?.unmount();app=null;vi.restoreAllMocks()})
function routes(){
 const table=answers({start:async()=>Response.json({operation_id:'op1'},{status:202})})
 table['GET /api/harnesses']=[claudeHarness]
 table['GET /api/projects']=[projectFixture]
 table['GET /api/projects/p1/threads']=[threadFixture];table['GET /api/threads/t1']=threadFixture
 return table
}
async function enter(box:HTMLTextAreaElement){
 box.focus()
 await act(async()=>{box.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true}))})
}
it('the message box keeps focus through a send and is read-only while it is in flight',async()=>{
 let release:()=>void=()=>{}
 const table=routes()
 table['POST /api/threads/t1/turns']=()=>new Promise<Response>(resolve=>{release=()=>resolve(Response.json({operation_id:'op1'},{status:202}))})
 app=await startApp('/projects/p1/threads/t1',table)
 await until(()=>app?.calls.includes('POST /api/threads/t1/session')===true)
 const box=document.querySelector('textarea')!
 await act(async()=>typeInto(box,'Hello'))
 await until(()=>app?.button('Send')?.disabled===false)
 await enter(box)
 await until(()=>app?.calls.includes('POST /api/threads/t1/turns')===true)
 expect(box.readOnly).toBe(true)
 expect(box.disabled).toBe(false)
 expect(document.activeElement).toBe(box)
 await act(async()=>release())
 await until(()=>!box.readOnly)
 expect(document.activeElement).toBe(box)
})
it('the message box keeps focus through a queued message',async()=>{
 let release:()=>void=()=>{}
 const table=routes()
 table['GET /api/threads/t1/operations']=[runningOperation()]
 table['POST /api/threads/t1/queue']=()=>new Promise<Response>(resolve=>{release=()=>resolve(Response.json({
  status:'waiting',message:{id:'q1',thread_id:'t1',position:1,prompt:'Queue',model:'fake-large',mode:'acceptEdits',effort:'high',focus:null,plan:null,last_error:null,created_at:'x'}
 },{status:202}))})
 app=await startApp('/projects/p1/threads/t1',table)
 await until(()=>app?.button('Stop')!==undefined)
 await until(()=>app?.calls.includes('POST /api/threads/t1/session')===true)
 const box=document.querySelector('textarea')!
 await act(async()=>typeInto(box,'Queue'))
 await enter(box)
 await until(()=>app?.calls.includes('POST /api/threads/t1/queue')===true)
 expect(box.readOnly).toBe(true)
 expect(document.activeElement).toBe(box)
 await act(async()=>release())
 await until(()=>!box.readOnly)
 expect(document.activeElement).toBe(box)
})
