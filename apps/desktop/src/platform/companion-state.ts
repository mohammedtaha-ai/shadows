export type PresenceProjection={
  expression:'idle'|'thinking'|'working'|'speaking'|'success'|'blocked';
  status:string;preview:{id:string;text:string;expiresAtMs:number}|null;
};
export type ComposerView={open:boolean;revision:number;scope:string|null;context:string|null;agent:string|null};
export type CompanionState = {composer:ComposerView;revision:number;visible:boolean;renderAllowed:boolean;paused:boolean;error:string|null;presence?:PresenceProjection};
function acceptPresence(raw:unknown):PresenceProjection{
 if(!raw||typeof raw!=='object')throw Error('companion_presence_invalid');
 const value=raw as PresenceProjection;
 if(!['idle','thinking','working','speaking','success','blocked'].includes(value.expression)||typeof value.status!=='string'||[...value.status].length>160)throw Error('companion_presence_invalid');
 if(value.preview!==null){
  const preview=value.preview;
  if(!preview||typeof preview!=='object'||typeof preview.id!=='string'||preview.id.length>512||typeof preview.text!=='string'||[...preview.text].length>240||!Number.isSafeInteger(preview.expiresAtMs)||preview.expiresAtMs<0)throw Error('companion_presence_invalid');
 }
 return value;
}
export type CompanionTransport = {
  listen:(handler:(value:unknown)=>void)=>Promise<()=>void>;
  snapshot:()=>Promise<unknown>;
  ready:()=>Promise<unknown>;
};
export function acceptState(current:CompanionState|null,raw:unknown):CompanionState {
  if(!raw || typeof raw!=='object') throw Error('companion_state_invalid');
  const value=raw as Record<string,unknown>;
  if(!Number.isSafeInteger(value.revision)|| (value.revision as number)<0 || typeof value.visible!=='boolean'||typeof value.renderAllowed!=='boolean'||typeof value.paused!=='boolean'||(value.error!==null&&typeof value.error!=='string')) throw Error('companion_state_invalid');
  const composer=value.composer as ComposerView;
  if(!composer||typeof composer!=='object'||typeof composer.open!=='boolean'||!Number.isSafeInteger(composer.revision)||composer.revision<0||[composer.scope,composer.context,composer.agent].some(x=>x!==null&&(typeof x!=='string'||x.length>128)))throw Error('companion_state_invalid');
  if(current && (value.revision as number)<=current.revision) return current;
  const presence=value.presence===undefined?{}:{presence:acceptPresence(value.presence)};
  return {composer:{...composer},revision:value.revision as number,visible:value.visible,renderAllowed:value.renderAllowed,paused:value.paused,error:value.error as string|null,...presence};
}
export function connectState(transport:CompanionTransport,onState:(state:CompanionState)=>void,onError:(reason:string)=>void,ready=false):()=>void {
  let disposed=false, unlisten:(()=>void)|undefined, current:CompanionState|null=null;
  const receive=(value:unknown)=>{
    if(disposed)return;
    try {const next=acceptState(current,value);if(next!==current){current=next;onState(next);}}
    catch {onError('Unable to synchronize AYLUL.');}
  };
  void (async()=>{
    try {
      const cleanup=await transport.listen(receive);
      if(disposed){cleanup();return;}
      unlisten=cleanup;
      receive(await (ready?transport.ready():transport.snapshot()));
    }catch{if(!disposed)onError('Unable to synchronize AYLUL.');}
  })();
  return ()=>{if(disposed)return;disposed=true;unlisten?.();unlisten=undefined;};
}
