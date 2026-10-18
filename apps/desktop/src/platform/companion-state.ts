export type CompanionState = {revision:number;visible:boolean;renderAllowed:boolean;paused:boolean;error:string|null};
export type CompanionTransport = {
  listen:(handler:(value:unknown)=>void)=>Promise<()=>void>;
  snapshot:()=>Promise<unknown>;
  ready:()=>Promise<unknown>;
};
export function acceptState(current:CompanionState|null,raw:unknown):CompanionState {
  if(!raw || typeof raw!=='object') throw Error('companion_state_invalid');
  const value=raw as Record<string,unknown>;
  if(!Number.isSafeInteger(value.revision)|| (value.revision as number)<0 || typeof value.visible!=='boolean'||typeof value.renderAllowed!=='boolean'||typeof value.paused!=='boolean'||(value.error!==null&&typeof value.error!=='string')) throw Error('companion_state_invalid');
  if(current && (value.revision as number)<=current.revision) return current;
  return {revision:value.revision as number,visible:value.visible,renderAllowed:value.renderAllowed,paused:value.paused,error:value.error as string|null};
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
