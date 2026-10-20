import {invoke} from '@tauri-apps/api/core'
import {listen} from '@tauri-apps/api/event'
import {isDesktop,type ConnectionView} from './runtime'

export interface LaunchConfig {
  configurationId:string
  executable:string
  db:string
  node:string
  adapter:string
  harness:string
  aylulConfig:string
  origin:string
}
export function nativeCall<T>(command:string,args?:Record<string,unknown>):Promise<T>{
  if(!isDesktop())return Promise.reject(new Error('desktop_required'))
  return invoke<T>(command,args)
}
export const connectionBridge={
  listen:(callback:(view:ConnectionView)=>void)=>listen<ConnectionView>('desktop-connection',event=>callback(event.payload)),
  snapshot:()=>nativeCall<ConnectionView>('desktop_connection'),
}
export const connectionConfig=()=>nativeCall<LaunchConfig|null>('desktop_connection_config')
export const retryConnection=()=>nativeCall<ConnectionView>('retry_desktop_connection')
export async function saveConnection(config:LaunchConfig):Promise<void>{
  const changed=await nativeCall<boolean>('save_desktop_connection',{config})
  if(changed){window.location.hash='/settings';window.location.reload()}
}
export const openExternal=(url:string)=>nativeCall<void>('open_external',{url})

export interface CompanionState {revision:number;visible:boolean;renderAllowed:boolean;paused:boolean;error:string|null}
function companionState(value:unknown):CompanionState{
  if(!value||typeof value!=='object')throw Error('companion_state_invalid')
  const state=value as CompanionState
  if(!Number.isSafeInteger(state.revision)||state.revision<0||typeof state.visible!=='boolean'||
    typeof state.renderAllowed!=='boolean'||typeof state.paused!=='boolean'||
    (state.error!==null&&typeof state.error!=='string'))throw Error('companion_state_invalid')
  return state
}
export function connectCompanion(receive:(state:CompanionState)=>void,failed:(message:string)=>void):()=>void{
  let disposed=false,off:(()=>void)|undefined,revision=-1
  const accept=(value:unknown)=>{
    if(disposed)return
    try{
      const state=companionState(value)
      if(state.revision>revision){revision=state.revision;receive(state)}
    }catch{failed('Unable to synchronize AYLUL.')}
  }
  void listen<unknown>('companion-state',event=>accept(event.payload)).then(dispose=>{
    if(disposed){dispose();return}
    off=dispose
    return nativeCall('companion_state').then(accept)
  }).catch(()=>{if(!disposed)failed('Unable to connect to AYLUL.')})
  return ()=>{disposed=true;off?.()}
}
export const setCompanionVisible=async(visible:boolean)=>companionState(await nativeCall('set_companion_visible',{visible}))
export const setCompanionPaused=async(paused:boolean)=>companionState(await nativeCall('set_companion_paused',{paused}))
export const hideToTray=()=>nativeCall<void>('hide_main')
