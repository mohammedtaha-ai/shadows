// Explicit isolated Windows acceptance. Run with AYLUL_RUN_NATIVE=1 after build:desktop/native build.
import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,mkdir,writeFile,readFile,rm} from 'node:fs/promises';
import {spawn,execFileSync} from 'node:child_process';
import {createHash,randomUUID} from 'node:crypto';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {chromium} from '@playwright/test';

const desktop=path.resolve(fileURLToPath(new URL('..',import.meta.url)));
const repo=path.resolve(desktop,'../..');
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
async function port(){
 const server=net.createServer();await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
 const value=server.address().port;await new Promise(resolve=>server.close(resolve));return value;
}
function configurationId(config){
 const digest=createHash('sha1').update(Buffer.from('6ba7b8119dad11d180b400c04fd430c8','hex')).update(JSON.stringify(config)).digest();
 digest[6]=(digest[6]&15)|80;digest[8]=(digest[8]&63)|128;
 const hex=digest.subarray(0,16).toString('hex');
 return hex.slice(0,8)+'-'+hex.slice(8,12)+'-'+hex.slice(12,16)+'-'+hex.slice(16,20)+'-'+hex.slice(20);
}
const canonical=file=>'\\\\?\\'+path.resolve(file);
const invoke=(page,command,args={})=>page.evaluate(({command,args})=>window.__TAURI_INTERNALS__.invoke(command,args),{command,args});
async function stopCore(root,id){
 try{
  const raw=await readFile(path.join(root,'runtimes',id+'.json'),'utf8').catch(()=>readFile(path.join(root,'runtimes',id+'.pending.json'),'utf8'));
  const receipt=JSON.parse(raw),started=raw.match(/"started_at"\s*:\s*(\d+)/)?.[1];
  assert.ok(started);
  const result=execFileSync('pwsh',['-NoProfile','-Command',
   '$ownedProcess=Get-Process -Id '+Number(receipt.process.pid)+' -ErrorAction SilentlyContinue; if($ownedProcess -and $ownedProcess.StartTime.ToUniversalTime().ToFileTimeUtc().ToString() -eq "'+started+'"){Stop-Process -Id $ownedProcess.Id -Force}'
  ],{windowsHide:true,encoding:'utf8'});
  void result;
 }catch(error){if(error.code!=='ENOENT')throw error}
}
test('packaged native main starts Core, sends once and restores its hash route',{
 skip:process.env.AYLUL_RUN_NATIVE!=='1',timeout:90000,
},async()=>{
 const root=await mkdtemp(path.join(os.tmpdir(),'desktop عربي acceptance '));
 const origin='http://127.0.0.1:'+await port(),debugPort=await port();
 await mkdir(path.join(root,'home'));
 await writeFile(path.join(root,'core.sqlite3'),'');
 await writeFile(path.join(root,'aylul.json'),JSON.stringify({home_root:path.join(root,'home')}));
 const core=path.join(repo,'target/debug/shadows.exe'),fake=path.join(repo,'target/debug/fake-acp.exe');
 const config={
  configurationId:'',executable:canonical(core),db:canonical(path.join(root,'core.sqlite3')),
  node:canonical(fake),adapter:canonical(fake),harness:canonical(fake),
  aylulConfig:canonical(path.join(root,'aylul.json')),origin,
 };
 config.configurationId=configurationId(config);
 await writeFile(path.join(root,'desktop-core.json'),JSON.stringify(config));
 let browser,companionBrowser,app;
 const errors=[],violations=[],requests=[];
 try{
  app=spawn(path.join(desktop,'src-tauri/target/debug/nylra-desktop.exe'),['--acceptance-root',root,'--acceptance-debug-port',String(debugPort)],{
   windowsHide:true,stdio:'ignore',env:{...process.env},
  });
  for(let i=0;i<100;i++){
   try{browser=await chromium.connectOverCDP('http://127.0.0.1:'+debugPort);break}catch{await sleep(200)}
  }
  assert.ok(browser,'owned native CDP endpoint must start');
  const context=browser.contexts()[0];
  const main=context.pages().find(page=>!page.url().includes('companion.html'));
  for(let i=0;i<100;i++){
   try{companionBrowser=await chromium.connectOverCDP('http://127.0.0.1:'+(debugPort+1));break}catch{await sleep(200)}
  }
  const companion=companionBrowser?.contexts()[0]?.pages()[0];
  assert.ok(main&&companion,'both independently packaged surfaces must load');
  main.on('pageerror',error=>errors.push(error.message));
  main.on('request',request=>{if(request.url().startsWith(origin))requests.push(request.url())});
  await main.evaluate(()=>document.addEventListener('securitypolicyviolation',event=>{
   (window.__cspViolations??=[]).push({directive:event.violatedDirective,blocked:event.blockedURI});
  }));
  await main.waitForFunction(()=>document.querySelector('aside')||document.body.textContent.includes('No projects'),{timeout:30000});
  assert.equal((await invoke(main,'desktop_connection')).state,'ready');
  for(const command of ['desktop_connection','desktop_connection_config','save_desktop_connection','retry_desktop_connection','open_external']){
   await assert.rejects(invoke(companion,command,{}),command+' must reject companion');
  }
  const call=async(method,route,body)=>{
   const response=await fetch(origin+route,{method,headers:{'content-type':'application/json'},body:body?JSON.stringify({command_id:randomUUID(),...body}):undefined});
   assert.ok(response.ok,method+' '+route+' '+await response.clone().text());
   return response.status===204?null:response.json();
  };
  const provider=await call('POST','/api/aylul/providers',{name:'Desktop fixture',base_url:'http://127.0.0.1:1',models:['fake-large','fake-small'],context_window:100000});
  await call('PUT','/api/aylul/providers/'+provider.slug+'/key',{value:'synthetic-fixture-key'});
  const agent=await call('POST','/api/aylul/agents',{name:'Native fixture agent',provider:provider.slug,default_model:'fake-large'});
  await main.evaluate(slug=>{location.hash='/aylul/agents/'+slug+'/new'},agent.slug);
  await main.getByRole('textbox',{name:'Message',exact:true}).fill('native fixture message');
  await main.getByRole('button',{name:'Send',exact:true}).click();
  await main.getByText('hello from fake_acp',{exact:true}).waitFor({timeout:30000});
  const route=new URL(main.url()).hash;
  assert.match(route,/\/threads\//);
  await main.reload();
  await main.getByText('hello from fake_acp',{exact:true}).waitFor({timeout:30000});
  assert.equal(new URL(main.url()).hash,route);
  const network=await context.newCDPSession(main);
  await network.send('Network.enable');
  let missingType=null;
  network.on('Network.responseReceived',event=>{
   if(event.response.url.includes('missing-asset-never-exists.js'))missingType=event.response.mimeType;
  });
  const missingLoaded=await main.evaluate(async()=>{
   try{await import('/missing-asset-never-exists.js');return true}catch{return false}
  });
  assert.equal(missingLoaded,false);
  assert.ok(!missingType?.includes('text/html'),'unknown assets must not fall back to main HTML');
  await network.detach();
  violations.push(...await main.evaluate(()=>window.__cspViolations??[]));
  assert.deepEqual(violations,[]);
  assert.deepEqual(errors,[]);
  const before=await call('GET','/api/runtime');
  app.kill();await sleep(300);
  assert.deepEqual(await call('GET','/api/runtime'),before,'desktop exit must preserve Core');
  const modules=JSON.parse(await readFile(path.join(desktop,'dist/desktop-modules.json'),'utf8'));
  assert.ok(modules.some(file=>file.endsWith('/web/src/main.tsx')));
  assert.ok(!modules.some(file=>file.includes('/apps/desktop/src/app/main.tsx')));
  await writeFile(path.join(repo,'.superpowers/sdd/2026-10-20-aylul-desktop-web-companion/task-4-native-acceptance.json'),
   JSON.stringify({passed:true,syntheticAdapter:true,route,requests:requests.length,violations,errors,runtime:before},null,2));
 }catch(error){
  console.error(error);
  throw error;
 }finally{
  await browser?.close();
  await companionBrowser?.close();
  app?.kill();
  await stopCore(root,config.configurationId);
  // Each browser's command line names this test-owned data directory.
  const quoted=root.replaceAll("'","''");
  execFileSync('pwsh',['-NoProfile','-Command',
   "Get-CimInstance Win32_Process -Filter \"Name = 'msedgewebview2.exe'\" | Where-Object { $_.CommandLine -and $_.CommandLine.Contains('"+quoted+"') } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }"
  ],{windowsHide:true,stdio:'ignore'});
  await sleep(500);
  // The root is the exact temporary directory created by this test.
  await rm(root,{recursive:true,force:true,maxRetries:10,retryDelay:100});
 }
});
