import {readHermesTarget} from './target.js';

export type HermesRunState='queued'|'running'|'waiting_for_approval'|'stopping'|'completed'|'failed'|'cancelled'|'interrupted';
export type HermesRun={runId:string;sessionId:string;state:HermesRunState;output?:string;completed?:boolean};
export type HermesAdmission={runId:string;state:HermesRunState|'started';replayed:boolean};
export type HermesClientOptions={origin:string;key:string;network?:string;timeoutMs?:number};
const states=new Set<HermesRunState>(['queued','running','waiting_for_approval','stopping','completed','failed','cancelled','interrupted']);
const object=(value:unknown):value is Record<string,unknown>=>typeof value==='object'&&value!==null&&!Array.isArray(value);
const identifier=(value:string)=>{if(!/^[A-Za-z0-9_-]{1,255}$/.test(value))throw new Error('bad_request');return value;};
const state=(value:unknown):HermesRunState=>{if(typeof value!=='string'||!states.has(value as HermesRunState))throw new Error('invalid_response');return value as HermesRunState;};
async function json(response:Response):Promise<unknown>{
 const reader=response.body?.getReader();if(!reader)throw new Error('invalid_response');
 const chunks:Uint8Array[]=[];let size=0;
 try{
  for(;;){const next=await reader.read();if(next.done)break;size+=next.value.byteLength;
   if(size>1048576){await reader.cancel();throw 0;}chunks.push(next.value);}
  return JSON.parse(Buffer.concat(chunks).toString('utf8')) as unknown;
 }catch{throw new Error('invalid_response');}finally{reader.releaseLock();}
}
/** Internal, operator-configured native API client. No retries or account authority. */
export function createHermesClient(options:HermesClientOptions){
 const target=readHermesTarget({HERMES_API_URL:options.origin,HERMES_NETWORK:options.network,
  HERMES_PROBE_TIMEOUT_MS:String(options.timeoutMs??3000)});
 const key=options.key;if(!key||key.trim()!==key||/[\s\x00-\x1f\x7f]/.test(key))throw new Error('hermes_config_invalid');
 async function request(method:'GET'|'POST',path:string,body?:unknown,commandId?:string):Promise<unknown>{
  let response:Response;
  try{response=await fetch(target.origin+path,{method,redirect:'error',signal:AbortSignal.timeout(target.timeoutMs),
   headers:{authorization:`Bearer ${key}`,...(body?{'content-type':'application/json'}:{}),...(commandId?{'idempotency-key':commandId}:{})},
   ...(body?{body:JSON.stringify(body)}:{})});}
  catch{throw new Error(method==='POST'?'outcome_unknown':'unavailable');}
  if(!response.ok){await response.body?.cancel();
   throw new Error(response.status===401||response.status===403?'credential_rejected':response.status===409?'command_conflict':response.status===404?'not_found':'unavailable');}
  try{return await json(response);}catch{throw new Error(method==='POST'?'outcome_unknown':'invalid_response');}
 }
 function session(value:unknown,id:string){
  if(!object(value)||value.object!=='hermes.session'||!object(value.session)||value.session.id!==id)throw new Error('invalid_response');
  return {id};
 }
 return {
  async capabilities(){
   const value=await request('GET','/v1/capabilities');
   if(!object(value)||value.object!=='hermes.api_server.capabilities'||value.platform!=='hermes-agent'||!object(value.auth)||value.auth.type!=='bearer'||value.auth.required!==true||!object(value.features)||!object(value.runtime)||value.runtime.mode!=='server_agent')throw new Error('invalid_response');
   const mode=value.features.runs_idempotency;
   if(!object(mode)||mode.supported!==true||typeof mode.durable!=='boolean'||typeof mode.retention_seconds!=='number'||!Number.isSafeInteger(mode.retention_seconds)||mode.retention_seconds<1)throw new Error('invalid_response');
   return {durableIdempotency:mode.durable,retentionSeconds:mode.retention_seconds};
  },
  async createSession(id:string,title:string){identifier(id);if(!title||Buffer.byteLength(title)>200)throw new Error('bad_request');return session(await request('POST','/api/sessions',{id,title,source:'api_server'}),id);},
  async getSession(id:string){identifier(id);return session(await request('GET',`/api/sessions/${id}`),id);},
  async submitRun(input:{sessionId:string;commandId:string;input:string}):Promise<HermesAdmission>{
   identifier(input.sessionId);identifier(input.commandId);
   if(!input.input||Buffer.byteLength(input.input)>16384)throw new Error('bad_request');
   const value=await request('POST','/v1/runs',{session_id:input.sessionId,input:input.input},input.commandId);
   if(!object(value)||typeof value.run_id!=='string'||!/^run_[A-Za-z0-9_-]+$/.test(value.run_id)||typeof value.replayed!=='boolean')throw new Error('outcome_unknown');
   let admitted:HermesAdmission['state'];
   try{admitted=value.status==='started'?'started':state(value.status);}catch{throw new Error('outcome_unknown');}
   return {runId:value.run_id,state:admitted,replayed:value.replayed};
  },
  async getRun(runId:string,sessionId:string):Promise<HermesRun>{
   identifier(runId);identifier(sessionId);const value=await request('GET',`/v1/runs/${runId}`);
   if(!object(value)||value.object!=='hermes.run'||value.run_id!==runId||value.session_id!==sessionId||
    (value.output!==undefined&&(typeof value.output!=='string'||Buffer.byteLength(value.output)>65536))||
    (value.completed!==undefined&&typeof value.completed!=='boolean'))throw new Error('invalid_response');
   return {runId,sessionId,state:state(value.status),...(typeof value.output==='string'?{output:value.output}:{}),
    ...(typeof value.completed==='boolean'?{completed:value.completed}:{})};
  },
  async stopRun(runId:string){
   identifier(runId);const value=await request('POST',`/v1/runs/${runId}/stop`,{});
   if(!object(value)||value.run_id!==runId||value.status!=='stopping')throw new Error('outcome_unknown');
   return {runId,state:'stopping' as const};
  },
 };
}
