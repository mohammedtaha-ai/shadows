// @vitest-environment happy-dom
import {act} from 'react'
import {afterEach,it,expect,vi} from 'vitest'
import * as companion from '@/desktop/companion-send'
import * as api from '@/api/threads'
import * as presence from '@/desktop/presence'
import {threadFixture,projectFixture,claudeHarness,runningOperation} from '@/test/contract-fixtures'
import {answers} from '@/test/fake-daemon'
import {startApp,typeInto,until,type TestApp} from '../test-app'
let app:TestApp|null=null
afterEach(()=>{app?.unmount();app=null;vi.restoreAllMocks();delete (window as unknown as Record<string,unknown>).__TAURI_INTERNALS__})
const bound={revision:1,generation:7,configurationId:'00000000-0000-4000-8000-000000000001',agent:'agent-a',target:null,previews:true}
it('binding failure is reported separately and never rejects an ordinary send',async()=>{
 Object.assign(window,{__TAURI_INTERNALS__:{}})
 vi.spyOn(presence,'companionBinding').mockResolvedValue(bound)
 vi.spyOn(api,'getThread').mockResolvedValue({...threadFixture,harness:'aylul',profile:'agent-a'})
 vi.spyOn(presence,'targetCompanionThread').mockRejectedValue('companion_binding_changed')
 await expect(companion.noteCompanionSend('t1')).resolves.toBeUndefined()
 expect(companion.companionSendProblem('t1')).toContain('Companion')
})
it('other agents and browser sends cannot replace the selected binding',async()=>{
 Object.assign(window,{__TAURI_INTERNALS__:{}})
 vi.spyOn(presence,'companionBinding').mockResolvedValue(bound)
 vi.spyOn(api,'getThread').mockResolvedValue({...threadFixture,harness:'aylul',profile:'agent-b'})
 const target=vi.spyOn(presence,'targetCompanionThread')
 await companion.noteCompanionSend('t1');expect(target).not.toHaveBeenCalled()
 delete (window as unknown as Record<string,unknown>).__TAURI_INTERNALS__
 await companion.noteCompanionSend('t1');expect(target).not.toHaveBeenCalled()
})
it('the selected agent uses the captured binding generation',async()=>{
 Object.assign(window,{__TAURI_INTERNALS__:{}})
 vi.spyOn(presence,'companionBinding').mockResolvedValue(bound)
 vi.spyOn(api,'getThread').mockResolvedValue({...threadFixture,harness:'aylul',profile:'agent-a'})
 const target=vi.spyOn(presence,'targetCompanionThread').mockResolvedValue({...bound,revision:2,generation:8})
 await companion.noteCompanionSend('t1')
 expect(target).toHaveBeenCalledWith('t1',7)
})
function routes(trace:string[],failSession=false){
 const thread={...threadFixture,harness:'aylul',profile:'agent-a'}
 const table=answers({start:async(request:Request)=>{trace.push('turn');const body=await request.json();expect(body.model).toBe('fake-small');return Response.json({operation_id:'op1'},{status:202})}})
 table['GET /api/harnesses']=[{...claudeHarness,kind:'aylul',label:'AYLUL'}]
 table['GET /api/projects']=[{...projectFixture,assistant:true,allowed_modes:{aylul:['auto','acceptEdits']}}]
 table['GET /api/aylul/providers']=[{slug:'provider-a',name:'Fixture',base_url:'http://127.0.0.1:1',models: [{ id: 'fake-large', context_window: 200000 }, { id: 'fake-small', context_window: 200000 }],key_set:true}]
 table['GET /api/aylul/agents']={project_id:'p1',agents:[{slug:'agent-a',name:'Agent A',provider:'provider-a',default_model:'fake-small',deleted:false}]}
 table['GET /api/aylul/agents/agent-a/threads']=[thread]
 table['GET /api/projects/p1/threads']=[thread];table['GET /api/threads/t1']=thread
 table['POST /api/projects/p1/threads']=async(request:Request)=>{trace.push('create');expect((await request.json()).profile).toBe('agent-a');return Response.json(thread,{status:201})}
 if(failSession)table['POST /api/threads/t1/session']=()=>{trace.push('session');throw Error('session fixture unavailable')}
 return table
}
it('first-send retains profile/model and binds its created thread before turn admission',async()=>{
 const trace:string[]=[]
 vi.spyOn(await import('@/desktop/companion-send'),'noteCompanionSend').mockImplementation(async(id)=>{expect(id).toBe('t1');trace.push('binding')})
 app=await startApp('/aylul/agents/agent-a/new',routes(trace))
 await until(()=>document.querySelector('textarea')!==null)
 typeInto(document.querySelector('textarea')!,'First agent message')
 await until(()=>app?.button('Send')?.disabled===false)
 await act(async()=>app?.button('Send')?.click())
 await until(()=>app?.path()==='/projects/p1/threads/t1')
 expect(trace).toEqual(['create','binding','turn'])
 expect(app.calls.filter(call=>call==='POST /api/threads/t1/turns')).toHaveLength(1)
})
it('a created thread remains bound when subsequent session opening fails',async()=>{
 const trace:string[]=[]
 vi.spyOn(await import('@/desktop/companion-send'),'noteCompanionSend').mockImplementation(async()=>{trace.push('binding')})
 app=await startApp('/aylul/agents/agent-a/new',routes(trace,true))
 await until(()=>document.querySelector('textarea')!==null)
 typeInto(document.querySelector('textarea')!,'Keep the created target')
 await until(()=>app?.button('Send')?.disabled===false)
 await act(async()=>app?.button('Send')?.click())
 await until(()=>app?.path()==='/projects/p1/threads/t1')
 expect(trace.slice(0,3)).toEqual(['create','binding','session'])
 expect(trace).not.toContain('turn')
})

it('an existing-thread send binds once without navigation retargeting or duplicate admission',async()=>{
 const trace:string[]=[]
 vi.spyOn(await import('@/desktop/companion-send'),'noteCompanionSend').mockImplementation(async()=>{trace.push('binding')})
 const table=routes(trace)
 table['POST /api/threads/t1/turns']=()=>{trace.push('turn');return Response.json({operation_id:'op1'},{status:202})}
 app=await startApp('/projects/p1/threads/t1',table)
 await until(()=>app?.button('Send')!==undefined)
 expect(trace).toEqual([])
 typeInto(document.querySelector('textarea')!,'An existing thread message')
 await until(()=>app?.button('Send')?.disabled===false)
 await act(async()=>app?.button('Send')?.click())
 await until(()=>trace.includes('turn'))
 expect(trace).toEqual(['binding','turn'])
 expect(app.calls.filter(call=>call==='POST /api/threads/t1/turns')).toHaveLength(1)
})
it('explicit queue targets the thread but never submits a second turn',async()=>{
 const trace:string[]=[]
 vi.spyOn(await import('@/desktop/companion-send'),'noteCompanionSend').mockImplementation(async()=>{trace.push('binding')})
 const table=routes(trace)
 table['GET /api/threads/t1/operations']=[runningOperation()]
 table['POST /api/threads/t1/queue']=()=>{trace.push('queue');return Response.json({
  status:'waiting',message:{id:'q1',thread_id:'t1',position:1,prompt:'Queue this',model:'fake-large',mode:'acceptEdits',effort:'high',focus:null,plan:null,last_error:null,created_at:'x'}
 },{status:202})}
 app=await startApp('/projects/p1/threads/t1',table)
 await until(()=>app?.button('Stop')!==undefined)
 expect(trace).toEqual([])
 await until(()=>app?.calls.includes('POST /api/threads/t1/session')===true)
 const box=document.querySelector('textarea')!
 await act(async()=>{typeInto(box,'Queue this');box.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true}))})
 await until(()=>trace.includes('queue'))
 expect(trace).toEqual(['binding','queue'])
 expect(app.calls.filter(call=>call==='POST /api/threads/t1/turns')).toHaveLength(0)
})
