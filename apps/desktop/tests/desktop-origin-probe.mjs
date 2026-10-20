import http from 'node:http';
import {readFile,writeFile,mkdir,open} from 'node:fs/promises';
import {spawn} from 'node:child_process';
import path from 'node:path';
import assert from 'node:assert/strict';

const config = JSON.parse(await readFile(process.argv[2], 'utf8'));
const run = config.run;
await mkdir(path.join(run,'project'),{recursive:true});
const results = { origins:{}, results:{}, coreOrigin:config.coreOrigin };
const children = [];
const logs = [];
const allowedEnv = ['SystemRoot','SystemDrive','ComSpec','PATHEXT','PATH','USERPROFILE','HOME','APPDATA','LOCALAPPDATA','TEMP','TMP'];
const env = Object.fromEntries(allowedEnv.filter(k => process.env[k]).map(k => [k,process.env[k]]));
let coreReady = false, starting = false;
async function startCore() {
  if(starting || !results.origins.main) return;
  starting = true;
  const log = await open(path.join(run,'core.log'),'w');
  logs.push(log);
  const child = spawn(config.coreExe,['serve','--db',path.join(run,'core.sqlite3'),
    '--bind','127.0.0.1:43381','--node',config.node,'--adapter',config.fake,
    '--harness',config.fake,'--allow-origin',results.origins.main],
    {env,windowsHide:true,stdio:['ignore',log.fd,log.fd]});
  children.push(child);
  for(let n=0;n<100;n++){
    try { if((await fetch(config.coreOrigin+'/api/projects')).ok){coreReady=true;return;} } catch {}
    await new Promise(r=>setTimeout(r,200));
  }
}
const server = http.createServer(async (req,res) => {
  const origin = req.headers.origin;
  if(origin) res.setHeader('Access-Control-Allow-Origin',origin);
  const url = new URL(req.url,config.collector);
  if(url.pathname.startsWith('/origin/')){
    results.origins[url.pathname.split('/').at(-1)] = origin;
    res.end('ok'); void startCore(); return;
  }
  if(url.pathname==='/ready'){
    res.statusCode=coreReady?200:503;
    res.end(JSON.stringify({directory:path.join(run,'project')})); return;
  }
  if(url.pathname.startsWith('/result/')){
    let body=''; for await (const chunk of req) body+=chunk;
    results.results[url.pathname.split('/').at(-1)] = JSON.parse(body);
    res.end('ok');return;
  }
  res.statusCode=404;res.end();
});
await new Promise((resolve,reject)=>server.listen(43382,'127.0.0.1',resolve).on('error',reject));
const nativeLog=await open(path.join(run,'native.log'),'w');
logs.push(nativeLog);
const native=spawn(config.nativeExe,[config.collector,config.coreOrigin],
  {env:process.env,windowsHide:true,stdio:['ignore',nativeLog.fd,nativeLog.fd]});
children.push(native);
console.log("probe.native.pid",native.pid);
native.on("exit",(code,signal)=>console.log("probe.native.exit",code,signal));
native.on("error",error=>console.log("probe.native.error",error.code));
let failure;
try {
  for(let n=0;n<180;n++){
    if(results.results.main && results.results.companion) break;
    if(native.exitCode !== null) break;
    await new Promise(r=>setTimeout(r,500));
  }
  await writeFile(path.join(run,'result.json'),JSON.stringify(results,null,2)+'\n');
  const main=results.results.main;
  assert.ok(main,'main result missing');
  assert.ok(results.origins.main && results.origins.main!=='null','actual request Origin missing');
  assert.equal(main.get,200);
  assert.equal(main.jsonMutation,200);
  assert.equal(main.sseCaughtUp,true);
  assert.equal(main.ipcAllowed,true);
  assert.equal(results.results.companion?.ipcAllowed,false);
  const foreign=await fetch(config.coreOrigin+'/api/projects',{headers:{Origin:'https://foreign.invalid'}});
  results.foreignOriginStatus=foreign.status;
  assert.equal(foreign.status,403);
  assert.equal(main.fonts,'loaded');
  assert.ok(main.rootWidth>0);
  await writeFile(path.join(run,'result.json'),JSON.stringify(results,null,2)+'\n');
  console.log('PASS packaged origin: GET, JSON, SSE, window ACL, foreign origin and layout');
} catch(e) {failure=e;} finally {
  for(const child of children) child.kill();
  server.close();
  await Promise.all(logs.map(log=>log.close()));
}
if(failure) throw failure;
