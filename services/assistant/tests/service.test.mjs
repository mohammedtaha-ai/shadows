import {test} from 'node:test';
import assert from 'node:assert/strict';
import {existsSync, mkdtempSync, rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {randomUUID} from 'node:crypto';
import {hermesFixture} from './fixture.mjs';
import {DatabaseSync} from 'node:sqlite';

async function setup() {
  assert.ok(existsSync(new URL('../dist/service.js',import.meta.url)), 'Local assistant implementation is missing');
  const {openService}=await import('../dist/service.js');
  const {createHermesClient}=await import('../../../hermes-client/dist/index.js').catch(()=>import('@shadows/hermes-client'));
  const root=mkdtempSync(join(tmpdir(),'shadows-assistant-')), fixture=await hermesFixture();
  const client=createHermesClient({origin:fixture.origin,network:'development-host',key:'synthetic-service-key',timeoutMs:300});
  let now=Date.now(), service;
  const open=()=>service=openService({database:join(root,'assistant.sqlite'),client,clock:()=>new Date(now)});
  open();
  return {fixture,database:join(root,'assistant.sqlite'),get service(){return service;},open,advance(ms){now+=ms;},close(){service.close();return fixture.close().finally(()=>rmSync(root,{recursive:true,force:true}));}};
}

test('local_chat_reopens_without_account_or_planner_and_keeps_identity',async()=>{
  const f=await setup();try{
    const commandId=randomUUID(), created=await f.service.call({action:'create',commandId});
    const id=created.conversation.id;
    const sent=await f.service.call({action:'send',conversationId:id,commandId:randomUUID(),text:'Hello'});
    assert.equal(sent.turns[0].output,'LOCAL_REPLY');assert.equal(sent.turns[0].state,'done');
    const identity=f.service.installationId;f.service.close();f.open();
    assert.equal(f.service.installationId,identity);
    assert.equal((await f.service.call({action:'create',commandId})).conversation.id,id);
    assert.equal((await f.service.call({action:'snapshot',conversationId:id})).turns[0].output,'LOCAL_REPLY');
    assert.equal(f.fixture.executions,1);
  }finally{await f.close();}
});
test('lost_admission_reply_is_only_recovered_explicitly_under_the_same_key',async()=>{
  const f=await setup();try{
    const {conversation}=await f.service.call({action:'create',commandId:randomUUID()});
    const request={action:'send',conversationId:conversation.id,commandId:randomUUID(),text:'Hello'};
    f.fixture.loseReply();const unknown=await f.service.call(request);assert.equal(unknown.turns[0].state,'unknown');
    f.service.close();f.open();await f.service.call({action:'snapshot',conversationId:conversation.id});await f.service.call(request);
    assert.equal(f.fixture.submits,1);
    await assert.rejects(f.service.call({...request,text:'Changed'}),/command_conflict/);
    await assert.rejects(f.service.call({...request,commandId:randomUUID()}),/generation_busy/);
    const recovered=await f.service.call({action:'recover',conversationId:conversation.id,turnId:unknown.turns[0].id});
    assert.equal(recovered.turns[0].state,'done');assert.equal(f.fixture.submits,2);assert.equal(f.fixture.executions,1);
  }finally{await f.close();}
});
test('expired_recovery_does_not_resubmit_and_unknown_fields_refuse',async()=>{
  const f=await setup();try{
    const {conversation}=await f.service.call({action:'create',commandId:randomUUID()});
    f.fixture.loseReply();const value=await f.service.call({action:'send',conversationId:conversation.id,commandId:randomUUID(),text:'Hello'});
    f.advance(61000);const recovered=await f.service.call({action:'recover',conversationId:conversation.id,turnId:value.turns[0].id});
    assert.equal(recovered.turns[0].state,'unknown');assert.equal(f.fixture.submits,1);
    for(const request of [{action:'status',url:'http://other'},{action:'snapshot',conversationId:'../escape'},{action:'send',conversationId:conversation.id,commandId:randomUUID(),text:'x'.repeat(4097)}])
      await assert.rejects(f.service.call(request),/request_invalid/);
  }finally{await f.close();}
});
test('turn_order_is_explicit_even_when_timestamps_are_equal',async()=>{
  const f=await setup();try{
    const {conversation}=await f.service.call({action:'create',commandId:randomUUID()});
    for(let index=0;index<35;index++)await f.service.call({action:'send',conversationId:conversation.id,commandId:randomUUID(),text:'Message '+index});
    const snapshot=await f.service.call({action:'snapshot',conversationId:conversation.id});
    assert.equal(snapshot.truncated,true);
    assert.deepEqual(snapshot.turns.map(turn=>turn.input),Array.from({length:30},(_,index)=>'Message '+(index+5)));
  }finally{await f.close();}
});
test('recovery_never_extends_the_guarantee_captured_before_submission',async()=>{
  const f=await setup();try{
    const {conversation}=await f.service.call({action:'create',commandId:randomUUID()});
    f.fixture.loseReply();const value=await f.service.call({action:'send',conversationId:conversation.id,commandId:randomUUID(),text:'Hello'});
    f.service.close();f.open();f.advance(61000);f.fixture.retention(120);
    const recovered=await f.service.call({action:'recover',conversationId:conversation.id,turnId:value.turns[0].id});
    assert.equal(recovered.turns[0].state,'unknown');
    assert.equal(f.fixture.submits,1,'Changed capabilities must not extend an already expired command guarantee');
  }finally{await f.close();}
});
test('prepared_intents_reopen_as_unknown_without_automatic_dispatch',async()=>{
  const f=await setup();try{
    const {conversation}=await f.service.call({action:'create',commandId:randomUUID()});
    f.fixture.loseReply();const value=await f.service.call({action:'send',conversationId:conversation.id,commandId:randomUUID(),text:'Hello'});
    f.service.close();const db=new DatabaseSync(f.database);
    db.prepare("UPDATE turns SET state='submitting' WHERE id=?").run(value.turns[0].id);db.close();f.open();
    assert.equal((await f.service.call({action:'snapshot',conversationId:conversation.id})).turns[0].state,'unknown');
    assert.equal(f.fixture.submits,1);
    assert.equal((await f.service.call({action:'recover',conversationId:conversation.id,turnId:value.turns[0].id})).turns[0].state,'done');
    assert.equal(f.fixture.executions,1);
  }finally{await f.close();}
});
test('version_one_unknown_intents_migrate_without_inventing_a_recovery_deadline',async()=>{
  const f=await setup();try{
    const {conversation}=await f.service.call({action:'create',commandId:randomUUID()});
    f.fixture.loseReply();const value=await f.service.call({action:'send',conversationId:conversation.id,commandId:randomUUID(),text:'Hello'});
    const installation=f.service.installationId;f.service.close();const db=new DatabaseSync(f.database);
    db.exec('ALTER TABLE turns DROP COLUMN recovery_until; PRAGMA user_version=1;');db.close();f.open();
    assert.equal(f.service.installationId,installation);
    assert.equal((await f.service.call({action:'recover',conversationId:conversation.id,turnId:value.turns[0].id})).turns[0].state,'unknown');
    assert.equal(f.fixture.submits,1);
  }finally{await f.close();}
});
test('stop_ack_remains_stopping_across_restart_until_the_upstream_confirms_cancellation',async()=>{
  const f=await setup();try{
    const {conversation}=await f.service.call({action:'create',commandId:randomUUID()});f.fixture.running();
    const value=await f.service.call({action:'send',conversationId:conversation.id,commandId:randomUUID(),text:'Hello'});
    assert.equal(value.turns[0].state,'running');
    assert.equal((await f.service.call({action:'stop',conversationId:conversation.id,turnId:value.turns[0].id})).turns[0].state,'stopping');
    f.service.close();f.open();
    assert.equal((await f.service.call({action:'snapshot',conversationId:conversation.id})).turns[0].state,'stopping');
    await assert.rejects(f.service.call({action:'send',conversationId:conversation.id,commandId:randomUUID(),text:'Another'}),/generation_busy/);
    f.fixture.cancelRuns();
    assert.equal((await f.service.call({action:'snapshot',conversationId:conversation.id})).turns[0].state,'cancelled');
    assert.equal(f.fixture.submits,1);
  }finally{await f.close();}
});
