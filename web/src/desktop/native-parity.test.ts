import {it,expect,vi} from 'vitest'
import fixture from '../test/native-presence.json'
const {navigate,listen,nativeCall}=vi.hoisted(()=>({navigate:vi.fn(),listen:vi.fn(),nativeCall:vi.fn()}))
vi.mock('@tauri-apps/api/event',()=>({listen}))
vi.mock('./native',()=>({nativeCall}))
import {parseBinding} from './presence'
vi.mock('../router',()=>({router:{load:async()=>{},navigate}}))
import {parseNavigation,connectNavigation} from './navigation'
it('consumes actual Rust-serialized binding and closed navigation projections',()=>{
  for(const value of [fixture.unbound,fixture.binding,fixture.target])expect(parseBinding(value)).toEqual(value)
  for(const value of fixture.destinations)expect(parseNavigation(value)).toEqual(value)
  expect(fixture.target.target?.projectId).toBeTruthy()
  expect(fixture.target.target?.threadId).toBeTruthy()
})
it('refuses endpoint selectors or invalid identities instead of treating them as navigation',()=>{
  expect(()=>parseNavigation({revision:1,destination:{kind:'url',url:'https://example.com'}})).toThrow()
  expect(()=>parseNavigation({revision:1,destination:{kind:'conversation',projectId:'../escape',threadId:'invalid'}})).toThrow()
  expect(()=>parseBinding({...fixture.binding,configurationId:'foreign'})).toThrow()
})

it('new navigation events win over a delayed snapshot and only the applied revision is acknowledged',async()=>{
  let snapshot!:(value:unknown)=>void
  const snapshotPromise=new Promise(resolve=>{snapshot=resolve})
  const off=vi.fn()
  listen.mockReset().mockResolvedValue(off)
  navigate.mockReset().mockResolvedValue(undefined)
  nativeCall.mockReset().mockImplementation(async(command:string)=>command==='navigation_snapshot'?snapshotPromise:undefined)
  const connected=connectNavigation()
  await Promise.resolve();await Promise.resolve()
  listen.mock.calls[0][1]({payload:fixture.destinations[2]})
  snapshot(fixture.destinations[0])
  const dispose=await connected
  await Promise.resolve();await Promise.resolve()
  expect(navigate).toHaveBeenCalledTimes(1)
  expect(nativeCall).toHaveBeenCalledWith('ack_navigation',{revision:fixture.destinations[2].revision})
  expect(()=>listen.mock.calls[0][1]({payload:{revision:99,destination:{kind:'url',url:'https://example.com'}}})).not.toThrow()
  dispose();expect(off).toHaveBeenCalledTimes(1)
})
