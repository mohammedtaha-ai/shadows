import {invoke} from '@tauri-apps/api/core';
import type {components} from '../api/generated/nylra-v1';
export type AssistantSnapshot=components['schemas']['AssistantSnapshot'];
export type AssistantConversation=components['schemas']['AssistantConversation'];
export type AssistantStatus=components['schemas']['AssistantStatus'];
const call=<T>(request:object)=>invoke<T>('assistant_call',{request});
export const assistant={
 status:()=>call<AssistantStatus>({action:'status'}),
 list:()=>call<{conversations:AssistantConversation[]}>({action:'list'}),
 create:(commandId:string)=>call<AssistantSnapshot>({action:'create',commandId}),
 snapshot:(conversationId:string)=>call<AssistantSnapshot>({action:'snapshot',conversationId}),
 send:(conversationId:string,commandId:string,text:string)=>call<AssistantSnapshot>({action:'send',conversationId,commandId,text}),
 stop:(conversationId:string,turnId:string)=>call<AssistantSnapshot>({action:'stop',conversationId,turnId}),
 recover:(conversationId:string,turnId:string)=>call<AssistantSnapshot>({action:'recover',conversationId,turnId}),
};
