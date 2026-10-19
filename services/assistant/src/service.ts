import {randomUUID,createHash} from 'node:crypto';
import type {createHermesClient} from '@shadows/hermes-client';
import {Store,type Conversation} from './store.js';
import {parseRequest} from './protocol.js';
type Client=ReturnType<typeof createHermesClient>;
const waiting=(state:string)=>['submitting','running','unknown','stopping'].includes(state);
const view=(row:Conversation)=>({id:row.id,title:row.title,createdAt:row.created_at,updatedAt:row.updated_at});
export function openService(options:{database:string;client:Client;clock?:()=>Date}){
  const store=new Store(options.database),client=options.client,clock=options.clock??(()=>new Date());
  const now=()=>clock().toISOString();
  async function snapshot(id:string){
    const conversation=store.conversation(id);let connection:'ready'|'offline'='ready';
    for(const turn of store.turns(id).filter(turn=>waiting(turn.state))){
      if(!turn.run_id)continue;
      try{const run=await client.getRun(turn.run_id,conversation.session_id);
        const state=run.state==='completed'?(run.completed===false?'error':'done'):run.state==='cancelled'?'cancelled':['failed','interrupted'].includes(run.state)?'error':run.state==='stopping'?'stopping':'running';
        store.update(turn.id,state,now(),undefined,run.output);
      }catch{connection='offline';}
    }
    const rows=store.turns(id);let remaining=12000,truncated=rows.length>30;
    const turns=[];
    for(const row of rows.slice(0,30)){
      remaining-=Buffer.byteLength(row.input);if(remaining<0){truncated=true;break;}
      // Slice by code points so truncation cannot manufacture replacement text.
      let output='';for(const char of row.output){const size=Buffer.byteLength(char);if(size>remaining){truncated=true;break;}remaining-=size;output+=char;}
      turns.push({id:row.id,commandId:row.command_id,input:row.input,output,state:row.state,createdAt:row.created_at});
    }
    return {conversation:view(conversation),turns:turns.reverse(),truncated,connection};
  }
  async function dispatch(id:string,turnId:string){
    const conversation=store.conversation(id),turn=store.turn(turnId,id);
    try{
      const capabilities=await client.capabilities();
      const age=clock().getTime()-Date.parse(turn.created_at);
      if(!capabilities.durableIdempotency||age<0||age>=capabilities.retentionSeconds*1000)throw new Error('outcome_unknown');
      try{await client.getSession(conversation.session_id);}catch(error){if(!(error instanceof Error)||error.message!=='not_found')throw error;await client.createSession(conversation.session_id,'AYLUL '+conversation.id);}
      const admission=await client.submitRun({sessionId:conversation.session_id,commandId:turn.id,input:turn.input});
      store.update(turnId,'running',now(),admission.runId);
    }catch{store.update(turnId,'unknown',now());}
    return snapshot(id);
  }
  async function call(value:unknown){
    const request=parseRequest(value);
    if(request.action==='status'){try{await client.capabilities();return {state:'ready'};}catch{return {state:'unavailable'};}}
    if(request.action==='list')return {conversations:store.list().map(view)};
    if(request.action==='create'){
      const id=store.transaction(()=>{const existing=store.byCommand(request.commandId);if(existing)return existing.id;
        const id=randomUUID(),time=now();store.create({id,command_id:request.commandId,session_id:'aylul_'+id.replaceAll('-',''),title:'New conversation',created_at:time,updated_at:time});return id;});
      return snapshot(id);
    }
    const id=request.conversationId;store.conversation(id);
    if(request.action==='snapshot')return snapshot(id);
    if(request.action==='send'){
      const fingerprint=createHash('sha256').update(JSON.stringify({id,text:request.text})).digest('hex');
      const intent=store.transaction(()=>{
        const old=store.turnCommand(request.commandId);
        if(old){if(old.fingerprint!==fingerprint)throw new Error('command_conflict');return {id:old.id,replay:true};}
        if(store.active(id))throw new Error('generation_busy');const turnId=randomUUID(),time=now();
        store.insert({id:turnId,conversation_id:id,command_id:request.commandId,fingerprint,input:request.text,output:'',run_id:null,state:'submitting',created_at:time,updated_at:time});return {id:turnId,replay:false};
      });
      return intent.replay?snapshot(id):dispatch(id,intent.id);
    }
    const turn=store.turn(request.turnId,id);
    if(request.action==='recover'){
      if(turn.state!=='unknown'||turn.run_id)return snapshot(id);
      store.update(turn.id,'submitting',now());return dispatch(id,turn.id);
    }
    if(waiting(turn.state)&&turn.run_id){try{await client.stopRun(turn.run_id);store.update(turn.id,'stopping',now());}catch{/* Poll the exact run; do not invent a cancellation result. */}}
    return snapshot(id);
  }
  return {call,installationId:store.installationId,close:()=>store.close()};
}
