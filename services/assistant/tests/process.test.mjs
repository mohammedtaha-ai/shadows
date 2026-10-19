import {test} from 'node:test';
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {mkdtempSync,writeFileSync,rmSync,existsSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {createInterface} from 'node:readline';
import {once} from 'node:events';
import {hermesFixture} from './fixture.mjs';
test('actual_stdio_process_has_closed_requests_and_does_not_print_credentials',async()=>{
  assert.ok(existsSync(new URL('../dist/main.js',import.meta.url)), 'Assistant process entry is missing');
  const root=mkdtempSync(join(tmpdir(),'shadows-assistant-process-')),fixture=await hermesFixture();
  const keyFile=join(root,'key'),config=join(root,'config.json');
  writeFileSync(keyFile,'synthetic-service-key',{mode:0o600});
  writeFileSync(config,JSON.stringify({database:join(root,'assistant.sqlite'),hermes:{origin:fixture.origin,network:'development-host',keyFile}}));
  const child=spawn(process.execPath,[new URL('../dist/main.js',import.meta.url).pathname.replace(/^\/(?=[A-Za-z]:)/,''),'--config',config],{stdio:['pipe','pipe','pipe']});
  let stderr='';child.stderr.on('data',data=>stderr+=data);
  const reader=createInterface({input:child.stdout})[Symbol.asyncIterator]();
  try{
    const ready=JSON.parse((await reader.next()).value);assert.match(ready.result.installationId,/^[0-9a-f-]{36}$/);
    child.stdin.write(JSON.stringify({id:1,request:{action:'status',url:'http://forged'}})+'\n');
    const refused=JSON.parse((await reader.next()).value);assert.equal(refused.error,'request_invalid');
    child.stdin.write(JSON.stringify({id:2,request:{action:'status'}})+'\n');
    const value=JSON.parse((await reader.next()).value);assert.equal(value.result.state,'ready');
    assert.ok(!JSON.stringify([ready,refused,value]).includes('synthetic-service-key'));
    const exited=once(child,'exit');child.stdin.end();assert.equal((await exited)[0],0);
    assert.ok(!stderr.includes('synthetic-service-key'));
  }finally{if(child.exitCode===null)child.kill();await fixture.close();rmSync(root,{recursive:true,force:true});}
});
