import {listen} from '@tauri-apps/api/event'
import {nativeCall} from './native'
export interface ConversationTarget {projectId:string;threadId:string}
export interface BindingView {
  revision:number;generation:number;configurationId:string|null;agent:string|null
  target:ConversationTarget|null;previews:boolean
}
const uuid=(value:unknown):value is string=>typeof value==='string'&&/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(value)
export function parseBinding(value:unknown):BindingView{
  if(!value||typeof value!=='object')throw Error('companion_binding_invalid')
  const item=value as BindingView
  if(!Number.isSafeInteger(item.revision)||item.revision<0||!Number.isSafeInteger(item.generation)||item.generation<0||
    (item.configurationId!==null&&!uuid(item.configurationId))||(item.agent!==null&&(typeof item.agent!=='string'||!item.agent))||
    typeof item.previews!=='boolean'||(item.target!==null&&(!uuid(item.target?.projectId)||!uuid(item.target?.threadId))))
    throw Error('companion_binding_invalid')
  return item
}
export function connectBinding(receive:(view:BindingView)=>void,failed:()=>void):()=>void{
  let disposed=false,revision=-1,off:(()=>void)|undefined
  const accept=(raw:unknown)=>{
    if(disposed)return
    try{const view=parseBinding(raw);if(view.revision>revision){revision=view.revision;receive(view)}}catch{failed()}
  }
  void listen<unknown>('companion-binding',event=>accept(event.payload)).then(dispose=>{
    if(disposed){dispose();return}off=dispose
    return companionBinding().then(accept)
  }).catch(()=>{if(!disposed)failed()})
  return ()=>{disposed=true;off?.()}
}
export const companionBinding=async()=>parseBinding(await nativeCall('companion_binding'))
export const selectCompanionAgent=async(agent:string|null)=>parseBinding(await nativeCall('select_companion_agent',{agent}))
export const targetCompanionThread=async(threadId:string,expectedGeneration:number)=>parseBinding(await nativeCall('target_companion_thread',{threadId,expectedGeneration}))
export const setCompanionPreviews=async(enabled:boolean)=>parseBinding(await nativeCall('set_companion_previews',{enabled}))
