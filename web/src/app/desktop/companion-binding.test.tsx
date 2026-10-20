// @vitest-environment happy-dom
import {act} from 'react'
import {createRoot,type Root} from 'react-dom/client'
import {QueryClient,QueryClientProvider} from '@tanstack/react-query'
import {beforeEach,afterEach,it,expect,vi} from 'vitest'
import fixture from '../../test/native-presence.json'
import {CompanionSettings} from './companion-settings'
const {invoke,listen}=vi.hoisted(()=>({invoke:vi.fn(),listen:vi.fn()}))
vi.mock('@tauri-apps/api/core',()=>({invoke}))
vi.mock('@tauri-apps/api/event',()=>({listen}))
vi.mock('@/api/aylul-profiles',()=>({
 agentsQuery:{queryKey:['fixture-agents'],queryFn:async()=>({project_id:'project',agents:[
  {slug:'agent-a',name:'AYLUL A',provider:'provider-a',default_model:'fake-large',deleted:false},
  {slug:'deleted',name:'Deleted agent',provider:'provider-a',default_model:'fake-small',deleted:true},
 ]})},
 providersQuery:{queryKey:['fixture-providers'],queryFn:async()=>[{slug:'provider-a',name:'Local provider'}]},
}))
let host:HTMLDivElement,root:Root,client:QueryClient
async function flush(){await act(async()=>{await new Promise(resolve=>setTimeout(resolve,10))})}
beforeEach(()=>{
 Object.assign(globalThis,{IS_REACT_ACT_ENVIRONMENT:true})
 Object.assign(window,{__TAURI_INTERNALS__:{}})
 host=document.createElement('div');document.body.append(host);root=createRoot(host)
 client=new QueryClient({defaultOptions:{queries:{retry:false}}})
 listen.mockReset().mockResolvedValue(()=>{})
 invoke.mockReset().mockImplementation(async(command:string)=>{
  if(command==='companion_binding')return fixture.unbound
  if(command==='select_companion_agent')return fixture.binding
  if(command==='set_companion_previews')return {...fixture.binding,revision:4,previews:false}
 })
})
afterEach(async()=>{await act(async()=>root.unmount());client.clear();host.remove();delete (window as unknown as Record<string,unknown>).__TAURI_INTERNALS__})
async function render(){await act(async()=>root.render(<QueryClientProvider client={client}><CompanionSettings/></QueryClientProvider>));await flush()}
it('lists live agent names with provider and default model metadata',async()=>{
 await render()
 expect(host.textContent).toContain('AYLUL A · Local provider · fake-large')
 expect(host.textContent).not.toContain('Deleted agent')
 expect((host.querySelector('select') as HTMLSelectElement).value).toBe('')
})
it('persists explicit selection through the native owner without changing viewed routes',async()=>{
 await render()
 const select=host.querySelector('select')!
 await act(async()=>{select.value='agent-a';select.dispatchEvent(new Event('change',{bubbles:true}))});await flush()
 expect(invoke).toHaveBeenCalledWith('select_companion_agent',{agent:'agent-a'})
 const before=invoke.mock.calls.length
 window.dispatchEvent(new HashChangeEvent('hashchange'));await flush()
 expect(invoke.mock.calls.length).toBe(before)
 expect(select.value).toBe('agent-a')
})
it('a later event cannot be overwritten by an older selection response',async()=>{
 let resolve!:(value:unknown)=>void
 invoke.mockImplementation(async(command:string)=>{
  if(command==='companion_binding')return fixture.unbound
  return new Promise(done=>{resolve=done})
 })
 await render()
 const select=host.querySelector('select')!
 await act(async()=>{select.value='agent-a';select.dispatchEvent(new Event('change',{bubbles:true}))})
 await act(async()=>{listen.mock.calls[0][1]({payload:{...fixture.binding,revision:8,agent:null}})})
 await act(async()=>resolve({...fixture.binding,revision:2}));await flush()
 expect(select.value).toBe('')
})
it('settings load failure stays local and reports a fixed message',async()=>{
 invoke.mockRejectedValue('companion_preferences_invalid')
 await render();expect(host.textContent).toContain('Unable to load companion settings')
})
