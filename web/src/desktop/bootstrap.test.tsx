// @vitest-environment happy-dom
import {describe,it,expect} from 'vitest'
import {bootApplication,waitForReady,type ConnectionView} from './runtime'

const view=(revision:number,state:ConnectionView['state'],origin:string|null=null):ConnectionView=>({
  revision,state,configurationId:'00000000-0000-4000-8000-000000000401',origin,error:null,
})
function deferred<T>() {
  let resolve!:(value:T)=>void
  const promise=new Promise<T>(done=>{resolve=done})
  return {promise,resolve}
}
describe('desktop bootstrap',()=>{
  it('does not mount API consumers before desktop readiness',async()=>{
    const ready=deferred<void>()
    let requests=0
    const result=bootApplication(true,()=>ready.promise,async()=>{requests++})
    await Promise.resolve()
    expect(requests).toBe(0)
    ready.resolve()
    await result
    expect(requests).toBe(1)
  })
  it('browser boot never invokes a native boundary',async()=>{
    let nativeCalls=0, mounted=0
    await bootApplication(false,async()=>{nativeCalls++},async()=>{mounted++})
    expect(nativeCalls).toBe(0)
    expect(mounted).toBe(1)
  })
  it('older ready snapshots cannot reverse a newer connection state',async()=>{
    const snapshot=deferred<ConnectionView>()
    let receive!:(state:ConnectionView)=>void
    const states:ConnectionView[]=[]
    let disposed=0
    const ready=waitForReady({
      listen:async(callback)=>{receive=callback;return()=>{disposed++}},
      snapshot:()=>snapshot.promise,
    },state=>states.push(state))
    await Promise.resolve()
    receive(view(3,'starting'))
    snapshot.resolve(view(2,'ready','http://127.0.0.1:43380'))
    await Promise.resolve()
    expect(states.at(-1)?.state).toBe('starting')
    receive(view(4,'ready','http://127.0.0.1:43381'))
    expect((await ready).origin).toBe('http://127.0.0.1:43381')
    expect(disposed).toBe(1)
  })
  it('failed connection stays visible without an automatic retry',async()=>{
    let calls=0
    let receive!:(state:ConnectionView)=>void
    const seen:ConnectionView[]=[]
    const ready=waitForReady({
      listen:async(callback)=>{receive=callback;return()=>{}},
      snapshot:async()=>{calls++;return {...view(1,'failed'),error:'core_start_timeout'}},
    },state=>seen.push(state))
    await Promise.resolve();await Promise.resolve();await Promise.resolve()
    expect(calls).toBe(1)
    expect(seen.at(-1)?.state).toBe('failed')
    receive(view(2,'ready','http://127.0.0.1:43381'))
    await ready
    expect(calls).toBe(1)
  })
})
