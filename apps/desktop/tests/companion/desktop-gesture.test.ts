import {test} from 'node:test';import assert from 'node:assert/strict';
import {createDesktopGesture} from '../../src/features/companion/desktop-gesture.ts';
const event=(x=20,button=0)=>({pointerId:1,button,x,y:20});
test('eight pixel drag consumes panel click and next gesture survives lost up',async()=>{
  let dragCalls=0,openCalls=0;let resolve:()=>void=()=>{};
  const gesture=createDesktopGesture({drag:()=>{dragCalls++;return new Promise<void>(r=>{resolve=r})},open:async()=>{openCalls++},fail:assert.fail});
  gesture.down(event());gesture.move(event(28));gesture.move(event(30));gesture.up(event(30));
  assert.equal(dragCalls,1);assert.equal(openCalls,0);resolve();await Promise.resolve();
  gesture.down(event());gesture.up(event(27.99));assert.equal(openCalls,1);
  gesture.down(event());gesture.move(event(28));resolve();await Promise.resolve();
  gesture.down(event());gesture.up(event());assert.equal(openCalls,2);gesture.dispose();
});
test('secondary cancellation disposal and drag rejection recover safely',async()=>{
  let opens=0,failures=0;
  const gesture=createDesktopGesture({drag:async()=>{throw Error('native refused')},open:async()=>{opens++},fail:()=>failures++});
  gesture.down(event(20,2));gesture.up(event(20,2));assert.equal(opens,0);
  gesture.down(event());gesture.cancel();gesture.up(event());assert.equal(opens,0);
  gesture.down(event());gesture.move(event(28));await Promise.resolve();await Promise.resolve();assert.equal(failures,1);
  gesture.down(event());gesture.up(event());assert.equal(opens,1);
  gesture.dispose();gesture.down(event());gesture.up(event());assert.equal(opens,1);
});
