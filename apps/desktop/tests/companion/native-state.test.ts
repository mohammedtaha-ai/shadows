import {test} from 'node:test';
import assert from 'node:assert/strict';
import {acceptState, connectState} from '../../src/platform/companion-state.ts';
const hidden={revision:8,visible:false,renderAllowed:false,paused:false,error:null};
const flush=()=>new Promise<void>(resolve=>setImmediate(resolve));
test('older ready reply cannot reverse newer native hide',async()=>{
  const values:any[]=[]; let listener:(value:unknown)=>void=()=>{}; let resolveReady:(v:unknown)=>void=()=>{};
  const cleanup=connectState({listen:async(fn)=>{listener=fn;return()=>{}},snapshot:async()=>hidden,ready:()=>new Promise(r=>{resolveReady=r})},v=>values.push(v),assert.fail,true);
  await flush(); listener(hidden); resolveReady({...hidden,revision:7,visible:true,renderAllowed:true}); await flush();
  assert.deepEqual(values,[hidden]); cleanup();
  assert.strictEqual(acceptState(hidden,{...hidden,revision:7}),hidden);
  assert.throws(()=>acceptState(hidden,{...hidden,revision:NaN}));
  assert.throws(()=>acceptState(hidden,{...hidden,revision:Number.MAX_SAFE_INTEGER+1}));
  assert.throws(()=>acceptState(hidden,{...hidden,visible:'yes'}));
});
test('late subscription after disposal cleans up without readiness or callbacks',async()=>{
  let subscribe:(cleanup:()=>void)=>void=()=>{};let cleanups=0,ready=0,callbacks=0;
  const dispose=connectState({listen:()=>new Promise(r=>{subscribe=r}),snapshot:async()=>hidden,ready:async()=>{ready++;return hidden}},()=>callbacks++,assert.fail,true);
  dispose();subscribe(()=>cleanups++);await flush();dispose();
  assert.equal(cleanups,1);assert.equal(ready,0);assert.equal(callbacks,0);
});
test('listener failure and malformed events never enable drawing',async()=>{
  let errors=0,callbacks=0;
  connectState({listen:async()=>{throw Error('refused')},snapshot:async()=>hidden,ready:async()=>hidden},()=>callbacks++,()=>errors++);
  await flush();assert.equal(callbacks,0);assert.equal(errors,1);
  let receive:(raw:unknown)=>void=()=>{};
  const dispose=connectState({listen:async(fn)=>{receive=fn;return()=>{}},snapshot:async()=>hidden,ready:async()=>hidden},()=>callbacks++,()=>errors++);
  await flush();receive({...hidden,revision:9,renderAllowed:1});assert.equal(callbacks,1);assert.equal(errors,2);dispose();
});
