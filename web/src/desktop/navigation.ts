import {listen} from '@tauri-apps/api/event'
import {nativeCall} from './native'
export type MainDestination=
  | {kind:'companionSettings'}
  | {kind:'agentDraft';agent:string}
  | {kind:'conversation';projectId:string;threadId:string}
  | {kind:'agentSettings';agent:string}
  | {kind:'newAgent'}
  | {kind:'settings'}
export interface PendingNavigation {revision:number;destination:MainDestination}
type PanelRoute={to:'/aylul';search:{settings:'agents';agent:string}|{settings:'general'}}
export function panelNavigationRoute(destination:Extract<MainDestination,{kind:'agentSettings'|'newAgent'|'settings'}>):PanelRoute{
  switch(destination.kind){
    case 'agentSettings':return {to:'/aylul',search:{settings:'agents',agent:destination.agent}}
    case 'newAgent':return {to:'/aylul',search:{settings:'agents',agent:'new'}}
    case 'settings':return {to:'/aylul',search:{settings:'general'}}
  }
}
export function parseNavigation(raw:unknown):PendingNavigation{
  if(!raw||typeof raw!=='object')throw Error('navigation_invalid')
  const value=raw as PendingNavigation,destination=value.destination
  if(!Number.isSafeInteger(value.revision)||value.revision<1||!destination||typeof destination!=='object')throw Error('navigation_invalid')
  const uuid=(text:unknown)=>typeof text==='string'&&/^[0-9a-f]{8}(-[0-9a-f]{4}){3}-[0-9a-f]{12}$/i.test(text)
  const valid=destination.kind==='companionSettings'||destination.kind==='newAgent'||destination.kind==='settings'||
    (destination.kind==='agentDraft'||destination.kind==='agentSettings')&&typeof destination.agent==='string'&&/^[a-z0-9][a-z0-9-]{0,127}$/.test(destination.agent)||
    destination.kind==='conversation'&&uuid(destination.projectId)&&uuid(destination.threadId)
  if(!valid)throw Error('navigation_invalid')
  return value
}
export async function connectNavigation(
  navigateDestination:(destination:MainDestination,panelRoute?:PanelRoute)=>Promise<void>,
  waitUntilReady:()=>Promise<void>,
):Promise<()=>void>{
  let disposed=false,revision=0,pending:PendingNavigation|null=null,running=false
  async function drain(){
    if(running)return
    running=true
    try{
      while(!disposed&&pending){
        const request=pending;pending=null
        const destination=request.destination
        if(destination.kind==='agentSettings'||destination.kind==='newAgent'||destination.kind==='settings'){
          await navigateDestination(destination,panelNavigationRoute(destination))
        }else await navigateDestination(destination)
        if(!disposed)await nativeCall('ack_navigation',{revision:request.revision})
      }
    }finally{running=false}
  }
  const accept=(raw:unknown)=>{
    if(disposed||raw===null)return
    let request:PendingNavigation
    try{request=parseNavigation(raw)}catch{return}
    if(request.revision<=revision)return
    revision=request.revision;pending=request
    void drain().catch(()=>{/* Pending request remains native-owned until a successful acknowledgement. */})
  }
  const off=await listen<unknown>('main-navigation',event=>accept(event.payload))
  try{await waitUntilReady();accept(await nativeCall('navigation_snapshot'))}
  catch(error){disposed=true;off();throw error}
  return ()=>{disposed=true;off()}
}
