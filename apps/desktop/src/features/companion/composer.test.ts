import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createDrafts,nextAttempt,submitKey,errorMessage} from './composer-model.ts';
test('draft survives close in the same scope and is removed on a new scope',()=>{const drafts=createDrafts();drafts.write('one','مرحبا');drafts.drop('one');assert.equal(drafts.read('one'),'مرحبا');drafts.drop('two');assert.equal(drafts.read('one'),'');});
test('unchanged scope and text retry one command; a changed scope or text creates another',()=>{let n=0;const id=()=>String(++n);const a=nextAttempt(null,'one','text',id);assert.deepEqual(nextAttempt(a,'one','text',id),a);assert.equal(nextAttempt(a,'two','text',id).commandId,'2');assert.equal(nextAttempt(a,'one','other',id).commandId,'3');});
test('Enter sends, Shift Enter inserts newline and composition never sends',()=>{const e={key:'Enter',shiftKey:false,isComposing:false};assert.equal(submitKey(e),'send');assert.equal(submitKey({...e,shiftKey:true}),'newline');assert.equal(submitKey({...e,isComposing:true}),null);assert.equal(submitKey({...e,key:'Escape'}),'close');});
test('unknown transport errors never appear in the editor',()=>{assert.equal(errorMessage('SECRET'),'Unable to send. Retry.');assert.match(errorMessage('companion_send_uncertain'),/same message/);assert.match(errorMessage('companion_send_busy'),/text is kept/);});
