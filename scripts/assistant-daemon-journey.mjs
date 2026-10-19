// Synthetic composed acceptance. No personal config, provider call or GUI launch.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {mkdtempSync,writeFileSync,readFileSync,rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {resolve,join,dirname,basename} from 'node:path';
import {fileURLToPath} from 'node:url';
import {randomUUID} from 'node:crypto';
import {once} from 'node:events';
import {setTimeout as delay} from 'node:timers/promises';
import {hermesFixture} from '../services/assistant/tests/fixture.mjs';
const workspace=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const root=mkdtempSync(join(tmpdir(),'shadows-daemon-journey-'));
const descriptorPath=join(root,'connection.json'),config=join(root,'assistant.json'),keyFile=join(root,'key');
const fixture=await hermesFixture();
let child,descriptor,diagnostics='';
writeFileSync(keyFile,'synthetic-service-key',{mode:0o600});
writeFileSync(config,JSON.stringify({database:join(root,'assistant.sqlite'),hermes:{origin:fixture.origin,network:'development-host',keyFile}}));
async function start(){
  child=spawn(resolve(workspace,process.platform==='win32'?'target/debug/shadows.exe':'target/debug/shadows'),['daemon','--node',process.execPath,'--entry',join(workspace,'services/assistant/dist/main.js'),'--config',config,'--descriptor',descriptorPath,'--bind','127.0.0.1:0'],{stdio:['ignore','pipe','pipe']});
  child.stderr.on('data',bytes=>diagnostics+=bytes);
  let ready=false;child.stdout.on('data',data=>{if(String(data).includes('daemon listening'))ready=true;});
  for(let attempt=0;attempt<400;attempt++){
    if(child.exitCode!==null)throw new Error('Daemon exited before readiness: '+diagnostics.slice(-1500).replaceAll('synthetic-service-key','[redacted]'));
    if(ready){descriptor=JSON.parse(readFileSync(descriptorPath,'utf8'));return;}
    await delay(25);
  }
  throw new Error('Daemon readiness deadline');
}
async function stop(){if(child&&child.exitCode===null){const exit=once(child,'exit');child.kill();await exit;await delay(100);}}
async function call(request,headers={}){
  const response=await fetch(descriptor.origin+'/assistant',{method:'POST',headers:{'content-type':'application/json',authorization:'Bearer '+descriptor.token,...headers},body:JSON.stringify(request)});
  assert.equal(response.status,200);return response.json();
}
try{
  await start();const installation=descriptor.installationId;
  let refused=await fetch(descriptor.origin+'/assistant',{method:'POST',headers:{'content-type':'application/json'},body:'{"action":"status"}'});assert.equal(refused.status,403);
  refused=await fetch(descriptor.origin+'/assistant',{method:'POST',headers:{'content-type':'application/json',authorization:'Bearer '+descriptor.token,origin:'https://untrusted.test'},body:'{"action":"status"}'});assert.equal(refused.status,403);
  assert.equal((await call({action:'status'})).state,'ready');
  const {conversation}=await call({action:'create',commandId:randomUUID()});
  fixture.loseReply();const sent=await call({action:'send',conversationId:conversation.id,commandId:randomUUID(),text:'Synthetic local chat'});
  assert.equal(sent.turns[0].state,'unknown');assert.equal(fixture.executions,1);
  await stop();await start();assert.equal(descriptor.installationId,installation);
  const before=await call({action:'snapshot',conversationId:conversation.id});assert.equal(before.turns[0].state,'unknown');assert.equal(fixture.submits,1);
  const recovered=await call({action:'recover',conversationId:conversation.id,turnId:sent.turns[0].id});assert.equal(recovered.turns[0].output,'LOCAL_REPLY');assert.equal(fixture.executions,1);
  assert.equal((await call({action:'list'})).conversations[0].id,conversation.id);
  fixture.running();const ongoing=await call({action:'send',conversationId:conversation.id,commandId:randomUUID(),text:'Synthetic stop check'});
  const active=ongoing.turns.find(turn=>turn.state==='running');assert.ok(active);
  const stopping=await call({action:'stop',conversationId:conversation.id,turnId:active.id});assert.equal(stopping.turns.find(turn=>turn.id===active.id).state,'stopping');
  fixture.cancelRuns();const cancelled=await call({action:'snapshot',conversationId:conversation.id});assert.equal(cancelled.turns.find(turn=>turn.id===active.id).state,'cancelled');assert.equal(fixture.executions,2);
  assert.ok(!diagnostics.includes('synthetic-service-key'));
  console.log(JSON.stringify({result:'PASS',scope:'real daemon + Node + owned Hermes HTTP fixture',restart:true,explicitRecovery:true,stopAcknowledgement:true,executions:fixture.executions,expectedExecutions:2,modelCalls:0}));
}finally{
  await stop();await fixture.close();
  assert.equal(dirname(root),tmpdir());assert.ok(basename(root).startsWith('shadows-daemon-journey-'));
  rmSync(root,{recursive:true,force:true,maxRetries:20,retryDelay:50});
}
