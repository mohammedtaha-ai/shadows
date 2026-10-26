import {useSyncExternalStore} from 'react'
import { getThread } from '@/api/threads'
import {isDesktop} from './runtime'
import {companionBinding,targetCompanionThread} from './presence'
let problem:{threadId:string;message:string}|null=null
const listeners=new Set<()=>void>()
const subscribe=(listener:()=>void)=>{listeners.add(listener);return()=>{listeners.delete(listener)}}
export function companionSendProblem(threadId:string|null):string|null{return problem?.threadId===threadId?problem.message:null}
export function useCompanionSendProblem(threadId:string|null):string|null{
  return useSyncExternalStore(subscribe,()=>companionSendProblem(threadId),()=>null)
}
/** Companion targeting is independent of the ordinary command and never rejects it. */
export async function noteCompanionSend(threadId:string):Promise<void>{
  if(!isDesktop())return
  try{
    const binding=await companionBinding()
    if(!binding.agent)return
    const thread=await getThread(threadId)
    if(thread.harness!=='aylul'||thread.profile!==binding.agent)return
    await targetCompanionThread(threadId,binding.generation)
    if(problem?.threadId===threadId){problem=null;listeners.forEach(listener=>listener())}
  }catch{
    problem={threadId,message:'Companion was not updated. Use with companion to retry, or retry Local Core in Settings.'}
    listeners.forEach(listener=>listener())
  }
}
