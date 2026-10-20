import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import path from 'node:path';
import {assembleDesktop} from './assemble-desktop.mjs';
const desktop=path.resolve(fileURLToPath(new URL('..',import.meta.url)));
const npm=process.env.npm_execpath;
if(!npm)throw new Error('Run build:desktop through npm');
for(const [cwd,args] of [
 [path.resolve(desktop,'../../web'),['run','build','--','--mode','desktop']],
 [desktop,['exec','--','vite','build','--mode','companion']],
]){
 const result=spawnSync(process.execPath,[npm,...args],{cwd,stdio:'inherit'});
 if(result.status!==0)process.exit(result.status??1);
}
await assembleDesktop(path.resolve(desktop,'../../web/dist'),path.join(desktop,'companion-dist'),path.join(desktop,'dist'));
