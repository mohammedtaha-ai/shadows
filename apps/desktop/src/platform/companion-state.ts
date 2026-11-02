export type PresenceProjection={
  expression:'idle'|'thinking'|'working'|'speaking'|'success'|'blocked';
  status:string;preview:{id:string;text:string;expiresAtMs:number}|null;
};
export type ComposerView={open:boolean;revision:number;scope:string|null;context:string|null;agent:string|null};
export type PanelAgent={slug:string;name:string;description:string;color:string;outfit:string;bound:boolean};
export type WaitingOption={optionId:string;name:string;kind:string};
export type WaitingRequest={threadId:string;requestId:string;title:string;options:WaitingOption[]};
export type WaitingView={count:number;bound:WaitingRequest[]};
export type PanelView={open:boolean;agents:PanelAgent[];waiting:WaitingView};
export type CompanionState = {composer:ComposerView;revision:number;visible:boolean;renderAllowed:boolean;paused:boolean;error:string|null;presence?:PresenceProjection;color:string;outfit:string;panel:PanelView};
function acceptWaiting(raw:unknown):WaitingView {
  if(raw===undefined)return {count:0,bound:[]};
  if(!raw||typeof raw!=='object')throw Error('companion_panel_invalid');
  const value=raw as Record<string,unknown>;
  if(typeof value.count!=='number'||!Number.isSafeInteger(value.count)||value.count<0||value.count>4294967295||!Array.isArray(value.bound)||value.bound.length>8)throw Error('companion_panel_invalid');
  const text=(item:unknown,max:number):item is string=>typeof item==='string'&&item.length>0&&[...item].length<=max;
  const bound=value.bound.map(entry=>{
    if(!entry||typeof entry!=='object')throw Error('companion_panel_invalid');
    const request=entry as Record<string,unknown>;
    if(!text(request.threadId,512)||!text(request.requestId,512)||!text(request.title,240)||!Array.isArray(request.options)||request.options.length>8)throw Error('companion_panel_invalid');
    const options=request.options.map(choice=>{
      if(!choice||typeof choice!=='object')throw Error('companion_panel_invalid');
      const option=choice as Record<string,unknown>;
      if(!text(option.optionId,512)||!text(option.name,80)||!text(option.kind,64))throw Error('companion_panel_invalid');
      return {optionId:option.optionId,name:option.name,kind:option.kind};
    });
    return {threadId:request.threadId,requestId:request.requestId,title:request.title,options};
  });
  return {count:value.count,bound};
}
function acceptPanel(raw:unknown,waitingRaw:unknown):PanelView {
  const waiting=acceptWaiting(waitingRaw);
  if(raw===undefined)return {open:false,agents:[],waiting};
  if(!raw||typeof raw!=='object')throw Error('companion_panel_invalid');
  const panel=raw as Record<string,unknown>;
  if(typeof panel.open!=='boolean'||!Array.isArray(panel.agents)||panel.agents.length>64)throw Error('companion_panel_invalid');
  const text=(value:unknown,max:number,empty=false):value is string=>typeof value==='string'&&(empty||value.length>0)&&[...value].length<=max;
  const agents=panel.agents.map(raw=>{
    if(!raw||typeof raw!=='object')throw Error('companion_panel_invalid');
    const agent=raw as Record<string,unknown>;
    if(!text(agent.slug,128)||!text(agent.name,128)||!text(agent.description,200,true)||!text(agent.color,32)||!text(agent.outfit,32)||typeof agent.bound!=='boolean')throw Error('companion_panel_invalid');
    return {slug:agent.slug,name:agent.name,description:agent.description,color:agent.color,outfit:agent.outfit,bound:agent.bound};
  });
  return {open:panel.open,agents,waiting};
}
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
  const appearance=value.appearance;
  const candidate=appearance&&typeof appearance==='object'?(appearance as Record<string,unknown>).color:undefined;
  const color=typeof candidate==='string'&&candidate.length>=1&&candidate.length<=32?candidate:'purple';
  const outfitId=appearance&&typeof appearance==='object'?(appearance as Record<string,unknown>).outfit:undefined;
  const outfit=typeof outfitId==='string'&&outfitId.length>=1&&outfitId.length<=32?outfitId:'default';
  return {composer:{...composer},revision:value.revision as number,visible:value.visible,renderAllowed:value.renderAllowed,paused:value.paused,error:value.error as string|null,color,outfit,panel:acceptPanel(value.panel,value.waiting),...presence};
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
