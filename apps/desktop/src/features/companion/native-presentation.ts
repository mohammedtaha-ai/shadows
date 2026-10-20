import type {PresenceProjection} from '../../platform/companion-state.ts';
import type {CompanionPresentation} from './types.ts';
import {createMessageSlot,receiveMessage,visibleMessage} from './animation/messages.ts';
export function createNativePresentation(){
 const slot=createMessageSlot();
 let lastId:string|null=null;
 let projection:PresenceProjection={expression:'idle',status:'Choose an agent',preview:null};
 return {
  receive(next:PresenceProjection|undefined,now:number){
   projection=next??{expression:'idle',status:'Choose an agent',preview:null};
   const preview=projection.preview;
   if(!preview){receiveMessage(slot,null,now);return;}
   if(preview.id===lastId)return;
   lastId=preview.id;
   const deadline=Math.min(preview.expiresAtMs,now+8000);
   receiveMessage(slot,deadline>now?{id:preview.id,text:preview.text,durationMs:deadline-now}:null,now);
  },
  read(now:number):CompanionPresentation{
   return {expression:projection.expression,statusText:projection.status,message:visibleMessage(slot,now)};
  },
  deadline:()=>slot.deadline,
 };
}
