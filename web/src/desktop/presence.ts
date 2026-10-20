import {useEffect,useState} from 'react'
import {isDesktop} from './runtime'
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

export interface PresencePreview {id:string;text:string;expiresAtMs:number}
export interface PresenceProjection {expression:'idle'|'thinking'|'working'|'speaking'|'success'|'blocked';status:string;preview:PresencePreview|null}
export function parsePresence(raw:unknown):PresenceProjection{
  if(!raw||typeof raw!=='object')throw Error('companion_presence_invalid')
  const value=raw as PresenceProjection
  if(!['idle','thinking','working','speaking','success','blocked'].includes(value.expression)||
    typeof value.status!=='string'||[...value.status].length>160)throw Error('companion_presence_invalid')
  if(value.preview!==null){
    const preview=value.preview
    if(!preview||typeof preview!=='object'||typeof preview.id!=='string'||preview.id.length>512||
      typeof preview.text!=='string'||[...preview.text].length>240||
      !Number.isSafeInteger(preview.expiresAtMs)||preview.expiresAtMs<0)throw Error('companion_presence_invalid')
  }
  return value
}

export function useCompanionBinding():BindingView|null{
  const [binding,setBinding]=useState<BindingView|null>(null)
  useEffect(()=>{
    if(!isDesktop())return
    return connectBinding(view=>setBinding(previous=>previous&&previous.revision>=view.revision?previous:view),()=>{})
  },[])
  return binding
}
