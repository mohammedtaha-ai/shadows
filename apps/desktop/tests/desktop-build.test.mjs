import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,mkdir,writeFile,readFile,rm} from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import {assembleDesktop} from '../scripts/assemble-desktop.mjs';
test('assembled desktop uses the shared main beside isolated companion assets',async()=>{
 const root=await mkdtemp(path.join(os.tmpdir(),'desktop-build-'));
 try {
  for(const folder of ['web/assets','companion/assets','out']) await mkdir(path.join(root,folder),{recursive:true});
  await writeFile(path.join(root,'web/index.html'),'<script src="./assets/main.js"></script>');
  await writeFile(path.join(root,'web/assets/main.js'),'shared-main');
  await writeFile(path.join(root,'web/desktop-main-modules.json'),JSON.stringify(['web/src/main.tsx']));
  await writeFile(path.join(root,'companion/companion.html'),'<script src="./assets/companion.js"></script>');
  await writeFile(path.join(root,'companion/assets/companion.js'),'character');
  await writeFile(path.join(root,'out/old-main.js'),'retired');
  await assembleDesktop(path.join(root,'web'),path.join(root,'companion'),path.join(root,'out'));
  assert.equal(await readFile(path.join(root,'out/assets/main.js'),'utf8'),'shared-main');
  assert.equal(await readFile(path.join(root,'out/companion/assets/companion.js'),'utf8'),'character');
  assert.deepEqual(JSON.parse(await readFile(path.join(root,'out/desktop-main-modules.json'),'utf8')),['web/src/main.tsx']);
  await assert.rejects(readFile(path.join(root,'out/old-main.js')));
 } finally {await rm(root,{recursive:true,force:true});}
});

test('assembly rejects an output inside its source before removing staged data',async()=>{
 const root=await mkdtemp(path.join(os.tmpdir(),'desktop-build-'));
 try {
  await mkdir(path.join(root,'web/out.assembling'),{recursive:true});
  await mkdir(path.join(root,'companion'),{recursive:true});
  await writeFile(path.join(root,'web/index.html'),'main');
  await writeFile(path.join(root,'companion/companion.html'),'companion');
  await writeFile(path.join(root,'web/out.assembling/keep.txt'),'keep');
  await assert.rejects(assembleDesktop(path.join(root,'web'),path.join(root,'companion'),path.join(root,'web/out')));
  assert.equal(await readFile(path.join(root,'web/out.assembling/keep.txt'),'utf8'),'keep');
 } finally {await rm(root,{recursive:true,force:true});}
});

test('production main inventory contains shared dashboard without retired chat or studio',async()=>{
 const modules=JSON.parse(await readFile(new URL('../dist/desktop-modules.json',import.meta.url),'utf8'));
 assert.ok(modules.some(file=>file.endsWith('/web/src/main.tsx')));
 assert.ok(modules.some(file=>file.endsWith('/web/src/app/desktop/native-settings.tsx')));
 assert.ok(!modules.some(file=>file.includes('/apps/desktop/src/app/main.tsx')||file.includes('/features/assistant/')||file.includes('/hermes-client/')||file.includes('/companion/studio/')||file.includes('/tests/')));
});
