import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createServer,type IncomingMessage,type ServerResponse} from 'node:http';
import {createHermesClient} from '../src/client.js';

async function fixture(handler:(req:IncomingMessage,res:ServerResponse)=>void){
 const server=createServer(handler);await new Promise<void>(resolve=>server.listen(0,'127.0.0.1',resolve));
 const port=(server.address() as {port:number}).port;
 const client=()=>createHermesClient({origin:`http://127.0.0.1:${port}`,network:'development-host',key:'synthetic-hermes-service-key',timeoutMs:200});
 return {client,async close(){server.closeAllConnections();await new Promise<void>(resolve=>server.close(()=>resolve()));}};
}
test('hermes_client_preserves_session_run_and_idempotency_identity',async()=>{
 const requests:Array<{method:string;path:string;key?:string;body:unknown}>=[];
 const f=await fixture((req,res)=>{let text='';req.on('data',chunk=>text+=chunk);req.on('end',()=>{
  requests.push({method:req.method!,path:req.url!,key:req.headers['idempotency-key'] as string|undefined,body:text?JSON.parse(text):null});
  assert.equal(req.headers.authorization,'Bearer synthetic-hermes-service-key');res.setHeader('content-type','application/json');
  if(req.method==='POST'&&req.url==='/api/sessions'){res.statusCode=201;res.end(JSON.stringify({object:'hermes.session',session:{id:'session-one'}}));}
  else if(req.url==='/api/sessions/session-one')res.end(JSON.stringify({object:'hermes.session',session:{id:'session-one'}}));
  else if(req.url==='/v1/runs'){res.statusCode=202;res.end(JSON.stringify({run_id:'run_one',status:'queued',replayed:false}));}
  else res.end(JSON.stringify({object:'hermes.run',run_id:'run_one',session_id:'session-one',status:'completed',output:'HERMES_BRIDGE_OK',completed:true}));
 });});
 try{
  const a=f.client();assert.equal((await a.createSession('session-one','Isolated test')).id,'session-one');
  assert.equal((await a.getSession('session-one')).id,'session-one');
  assert.equal((await a.submitRun({sessionId:'session-one',commandId:'command-one',input:'Reply in English.'})).runId,'run_one');
  const b=f.client();const result=await b.getRun('run_one','session-one');assert.equal(result.output,'HERMES_BRIDGE_OK');
  assert.equal(requests[2]!.key,'command-one');assert.deepEqual(requests[2]!.body,{session_id:'session-one',input:'Reply in English.'});
  assert.equal(requests.length,4);assert.ok(!JSON.stringify(b).includes('synthetic-hermes-service-key'));
 }finally{await f.close();}
});
test('hermes_client_refuses_invalid_target_and_path_or_input_before_network',async()=>{
 assert.throws(()=>createHermesClient({origin:'http://public.test',key:'synthetic-key'}),{message:'hermes_config_invalid'});
 let calls=0;const f=await fixture((_req,res)=>{calls++;res.end('{}');});
 try{
  for(const id of ['../private','a/b','a?key=x',''])await assert.rejects(f.client().getRun(id,'session-one'),{message:'bad_request'});
  await assert.rejects(f.client().submitRun({sessionId:'session-one',commandId:'cmd',input:'x'.repeat(16385)}),{message:'bad_request'});
  assert.equal(calls,0);
 }finally{await f.close();}
});
test('hermes_client_uncertain_submit_is_unknown_and_never_automatically_repeated',async()=>{
 let calls=0;const f=await fixture((req)=>{calls++;req.socket.destroy();});
 try{await assert.rejects(f.client().submitRun({sessionId:'session-one',commandId:'stable-command',input:'Reply OK.'}),{message:'outcome_unknown'});assert.equal(calls,1);}finally{await f.close();}
});
test('hermes_client_refuses_wrong_run_or_session_identity',async()=>{
 let wrongSession=false;const f=await fixture((_req,res)=>res.end(JSON.stringify({object:'hermes.run',run_id:wrongSession?'run_one':'run_other',session_id:'session-other',status:'completed',output:'Unrelated response'})));
 try{
  await assert.rejects(f.client().getRun('run_one','session-one'),{message:'invalid_response'});
  wrongSession=true;await assert.rejects(f.client().getRun('run_one','session-one'),{message:'invalid_response'});
 }finally{await f.close();}
});
test('hermes_client_maps_refusals_without_upstream_diagnostics',async()=>{
 let status=401;const f=await fixture((_req,res)=>{res.statusCode=status;res.end('private diagnostic synthetic-hermes-service-key');});
 try{
  await assert.rejects(f.client().getRun('run_one','session-one'),{message:'credential_rejected'});
  status=409;await assert.rejects(f.client().submitRun({sessionId:'session-one',commandId:'same-command',input:'Changed message.'}),{message:'command_conflict'});
  status=404;await assert.rejects(f.client().getRun('run_one','session-one'),{message:'not_found'});
 }finally{await f.close();}
});
test('hermes_client_bounds_reads_and_does_not_follow_redirects',async()=>{
 let redirect=false,calls=0;const f=await fixture((_req,res)=>{calls++;if(redirect){res.statusCode=302;res.setHeader('location','http://127.0.0.1:1/private');res.end();}else res.end('x'.repeat(1048577));});
 try{
  await assert.rejects(f.client().getRun('run_one','session-one'),{message:'invalid_response'});
  redirect=true;await assert.rejects(f.client().getRun('run_one','session-one'),{message:'unavailable'});assert.equal(calls,2);
 }finally{await f.close();}
});
test('hermes_client_stop_ack_is_stopping_not_completion',async()=>{
 const f=await fixture((_req,res)=>res.end(JSON.stringify({run_id:'run_one',status:'stopping'})));
 try{assert.deepEqual(await f.client().stopRun('run_one'),{runId:'run_one',state:'stopping'});}finally{await f.close();}
});
test('hermes_client_accepts_started_admission_without_claiming_run_completion',async()=>{
 const f=await fixture((_req,res)=>{res.statusCode=202;res.end(JSON.stringify({run_id:'run_one',status:'started',replayed:false}));});
 try{assert.deepEqual(await f.client().submitRun({sessionId:'session-one',commandId:'new-command',input:'Reply OK.'}),
  {runId:'run_one',state:'started',replayed:false});}finally{await f.close();}
});

