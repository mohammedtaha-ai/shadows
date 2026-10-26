import {test} from 'node:test';
import assert from 'node:assert/strict';
import {acceptState, connectState} from './companion-state.ts';
const composer={open:false,revision:0,scope:null,context:null,agent:null};
const hidden={composer,revision:8,visible:false,renderAllowed:false,paused:false,error:null,color:'purple'};
test('composer view is validated and never optional',()=>{
  const {composer:ignored,...missing}=hidden;
  assert.throws(()=>acceptState(null,missing));
  for(const invalid of [{...composer,open:'yes'},{...composer,scope:'x'.repeat(129)},{...composer,revision:-1},{...composer,context:7}])assert.throws(()=>acceptState(null,{...hidden,composer:invalid}));
  assert.deepEqual(acceptState(null,hidden),hidden);
});
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
test('appearance colour is carried into the state',()=>{
  for(const color of ['blue','teal'])assert.equal(acceptState(null,{...hidden,appearance:{color}}).color,color);
});
test('missing or malformed appearance reads as purple',()=>{
  for(const appearance of [undefined,null,{color:7},{color:''},{color:'x'.repeat(33)}]){
    const state=acceptState(null,{...hidden,appearance});
    assert.equal(state.color,'purple');assert.equal(state.revision,hidden.revision);
  }
});
test('an older revision cannot change the colour',()=>{
  const current=acceptState(null,{...hidden,revision:5,appearance:{color:'blue'}});
  assert.strictEqual(acceptState(current,{...hidden,revision:4,appearance:{color:'pink'}}),current);
  assert.equal(current.color,'blue');
});
