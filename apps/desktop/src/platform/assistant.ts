import {invoke} from '@tauri-apps/api/core';
import type {components} from '../api/generated/nylra-v1';
export type AssistantSnapshot=components['schemas']['AssistantSnapshot'];
export type AssistantConversation=components['schemas']['AssistantConversation'];
export type AssistantStatus=components['schemas']['AssistantStatus'];
export type AssistantSelection={source:'remote'}|{source:'local';installationId:string};
export const localInstallation=()=>invoke<string|null>('assistant_sources');
export function assistantFor(selection:AssistantSelection={source:'remote'},principal?:{accountId:string;sessionId:string}){
 const call=<T>(request:object)=>selection.source==='local'
  ?invoke<T>('assistant_local_call',{request,installationId:selection.installationId})
  :principal?invoke<T>('assistant_call',{request,expectedAccountId:principal.accountId,expectedSessionId:principal.sessionId})
  :Promise.reject<T>(new Error('verification_refused'));
 return {
 status:()=>call<AssistantStatus>({action:'status'}),
 list:()=>call<{conversations:AssistantConversation[]}>({action:'list'}),
 create:(commandId:string)=>call<AssistantSnapshot>({action:'create',commandId}),
 snapshot:(conversationId:string)=>call<AssistantSnapshot>({action:'snapshot',conversationId}),
 send:(conversationId:string,commandId:string,text:string)=>call<AssistantSnapshot>({action:'send',conversationId,commandId,text}),
 stop:(conversationId:string,turnId:string)=>call<AssistantSnapshot>({action:'stop',conversationId,turnId}),
 recover:(conversationId:string,turnId:string)=>call<AssistantSnapshot>({action:'recover',conversationId,turnId}),
 };
}
export const assistant=assistantFor();
