// Unsent text and command identity belong to one native conversation scope.
export type Attempt={scope:string;text:string;commandId:string};
export function createDrafts(){
 const values=new Map<string,string>();
 return {read:(scope:string)=>values.get(scope)??'',write:(scope:string,text:string)=>{values.set(scope,text);},drop:(keep:string|null)=>{for(const scope of values.keys())if(scope!==keep)values.delete(scope);}};
}
export function nextAttempt(previous:Attempt|null,scope:string,text:string,newId:()=>string):Attempt{
 return previous?.scope===scope&&previous.text===text?previous:{scope,text,commandId:newId()};
}
export function submitKey(event:{key:string;shiftKey:boolean;isComposing:boolean}):'send'|'newline'|'close'|null{
 if(event.key==='Escape')return 'close';
 if(event.key!=='Enter'||event.isComposing)return null;
 return event.shiftKey?'newline':'send';
}
export function errorMessage(code:string):string{
 const messages:Record<string,string>={
  companion_send_busy:'AYLUL is still answering. Your text is kept.',
  companion_send_uncertain:'Not sure it was sent. Retry sends the same message.',
  companion_send_stale:'AYLUL changed. Check the agent and send again.',
  companion_send_setup:"Finish this agent's setup in Settings.",
  companion_send_settings:"This conversation's mode is not allowed here. Open it in main.",
  companion_thread_invalid:'This conversation was removed. Choose it again in Settings.',
  companion_core_unavailable:'Core unavailable. Retry local Core in Settings.',
 };
 return Object.hasOwn(messages,code)?messages[code]:'Unable to send. Retry.';
}
