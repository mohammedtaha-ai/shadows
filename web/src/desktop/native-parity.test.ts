import {it,expect,vi} from 'vitest'
import fixture from '../test/native-presence.json'
const {navigate,listen,nativeCall}=vi.hoisted(()=>({navigate:vi.fn(),listen:vi.fn(),nativeCall:vi.fn()}))
vi.mock('@tauri-apps/api/event',()=>({listen}))
vi.mock('./native',()=>({nativeCall}))
import {parseBinding,parsePresence} from './presence'
import {parseNavigation,connectNavigation,panelNavigationRoute} from './navigation'
it('parses panel destinations and maps them to settings routes',()=>{
  const cases=[
    [{kind:'agentSettings',agent:'my-agent'},{to:'/aylul',search:{settings:'agents',agent:'my-agent'}}],
    [{kind:'newAgent'},{to:'/aylul',search:{settings:'agents',agent:'new'}}],
    [{kind:'settings'},{to:'/aylul',search:{settings:'general'}}],
  ] as const
  for(const [destination,route] of cases){
    expect(parseNavigation({revision:1,destination}).destination).toEqual(destination)
    expect(panelNavigationRoute(destination)).toEqual(route)
  }
  for(const agent of ['', '../bad', 'Bad', 'a'.repeat(129)]){
    expect(()=>parseNavigation({revision:1,destination:{kind:'agentSettings',agent}})).toThrow('navigation_invalid')
  }
  expect(()=>parseNavigation({revision:1,destination:{kind:'unknown'}})).toThrow('navigation_invalid')
})
it('consumes actual Rust-serialized binding and closed navigation projections',()=>{
  for(const value of [fixture.unbound,fixture.binding,fixture.target])expect(parseBinding(value)).toEqual(value)
  for(const value of fixture.destinations)expect(parseNavigation(value)).toEqual(value)
  expect(parsePresence(fixture.companion.presence)).toEqual(fixture.companion.presence)
  expect(parsePresence(fixture.activePresence)).toEqual(fixture.activePresence)
  expect(fixture.target.target?.projectId).toBeTruthy()
  expect(fixture.target.target?.threadId).toBeTruthy()
})
it('refuses endpoint selectors or invalid identities instead of treating them as navigation',()=>{
  expect(()=>parseNavigation({revision:1,destination:{kind:'url',url:'https://example.com'}})).toThrow()
  expect(()=>parseNavigation({revision:1,destination:{kind:'conversation',projectId:'../escape',threadId:'invalid'}})).toThrow()
  expect(()=>parseBinding({...fixture.binding,configurationId:'foreign'})).toThrow()
  expect(()=>parsePresence({...fixture.activePresence,expression:'listening'})).toThrow()
  expect(()=>parsePresence({...fixture.activePresence,preview:{...fixture.activePresence.preview,text:'x'.repeat(241)}})).toThrow()
})

it('new navigation events win over a delayed snapshot and only the applied revision is acknowledged',async()=>{
  let snapshot!:(value:unknown)=>void
  const snapshotPromise=new Promise(resolve=>{snapshot=resolve})
  const off=vi.fn()
  listen.mockReset().mockResolvedValue(off)
  navigate.mockReset().mockResolvedValue(undefined)
  nativeCall.mockReset().mockImplementation(async(command:string)=>command==='navigation_snapshot'?snapshotPromise:undefined)
  const connected=connectNavigation(navigate,async()=>{})
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

it('acknowledges navigation only after the injected navigation has applied',async()=>{
  let applied!:()=>void
  const applying=new Promise<void>(resolve=>{applied=resolve})
  listen.mockReset().mockResolvedValue(vi.fn())
  navigate.mockReset().mockReturnValue(applying)
  nativeCall.mockReset().mockImplementation(async(command:string)=>command==='navigation_snapshot'?fixture.destinations[0]:undefined)
  const dispose=await connectNavigation(navigate,async()=>{})
  expect(navigate).toHaveBeenCalledWith(fixture.destinations[0].destination)
  expect(nativeCall).not.toHaveBeenCalledWith('ack_navigation',expect.anything())
  applied()
  await Promise.resolve();await Promise.resolve()
  expect(nativeCall).toHaveBeenCalledWith('ack_navigation',{revision:fixture.destinations[0].revision})
  dispose()
})

it('waits for injected router readiness before reading the navigation snapshot',async()=>{
  let ready!:()=>void
  const loading=new Promise<void>(resolve=>{ready=resolve})
  listen.mockReset().mockResolvedValue(vi.fn())
  navigate.mockReset().mockResolvedValue(undefined)
  nativeCall.mockReset().mockResolvedValue(null)
  const connected=connectNavigation(navigate,()=>loading)
  await Promise.resolve();await Promise.resolve()
  expect(listen).toHaveBeenCalledTimes(1)
  expect(nativeCall).not.toHaveBeenCalled()
  ready()
  const dispose=await connected
  expect(nativeCall).toHaveBeenCalledWith('navigation_snapshot')
  dispose()
})
