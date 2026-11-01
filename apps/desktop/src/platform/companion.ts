import {invoke} from '@tauri-apps/api/core';
import {getCurrentWebviewWindow} from '@tauri-apps/api/webviewWindow';
import {acceptState,connectState,type CompanionState,type CompanionTransport} from './companion-state.ts';
export type {CompanionState} from './companion-state.ts';
const nativeTransport:CompanionTransport={
  listen:handler=>getCurrentWebviewWindow().listen<unknown>('companion-state',event=>handler(event.payload)),
  snapshot:()=>invoke('companion_state'),ready:()=>invoke('companion_ready'),
};
export function connectCompanion(onState:(state:CompanionState)=>void,onError:(reason:string)=>void,ready=false,transport=nativeTransport):()=>void {
  return connectState(transport,onState,onError,ready);
}
export async function setCompanionInput(open:boolean):Promise<CompanionState>{return acceptState(null,await invoke('companion_input',{open}));}
export async function setCompanionPanel(open:boolean):Promise<CompanionState>{return acceptState(null,await invoke('companion_panel',{open}));}
export async function selectCompanionAgent(agent:string):Promise<void>{await invoke('select_companion_agent',{agent});}
export async function sendFromCompanion(text:string,commandId:string,context:string):Promise<{threadId:string;operationId:string}>{
 const value=await invoke<{threadId:string;operationId:string}>('companion_send',{submission:{text,commandId,context}});
 if(!value||typeof value.threadId!=='string'||typeof value.operationId!=='string')throw Error('companion_send_uncertain');
 return value;
}
export function dragCompanion():Promise<void>{return invoke('drag_companion');}
export function reportCompanionFailure():Promise<CompanionState>{return invoke('companion_failed');}
export function openMainFromCompanion():Promise<void>{return invoke('open_main_from_companion');}
