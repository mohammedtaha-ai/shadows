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
