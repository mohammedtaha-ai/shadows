// Deterministic browser coverage. Requires assistant.vite.config.ts on port 4180.
// Every backend response and the native IPC boundary are explicit substitutes.
import {chromium,expect} from '@playwright/test';
import assert from 'node:assert/strict';
const browser=await chromium.launch({channel:'chrome',headless:true});
try{
 const context=await browser.newContext();
 const principal={accountId:'fixture-account',sessionId:'fixture-session',deviceId:'fixture-device'};
 await context.addInitScript(value=>sessionStorage.setItem('assistant-fixture-grant',JSON.stringify({handle:'synthetic',principal:value})),principal);
 const page=await context.newPage();
 const conversation={id:'11111111-1111-4111-8111-111111111111',title:'Fixture conversation',createdAt:new Date().toISOString(),updatedAt:new Date().toISOString()};
 const turn={id:'22222222-2222-4222-8222-222222222222',commandId:'33333333-3333-4333-8333-333333333333',input:'Original request',output:'',state:'unknown',createdAt:conversation.createdAt,updatedAt:conversation.updatedAt};
 let state='unknown',sends=0,stops=0,recoveries=0,offline=false;
 await page.route('**/__assistant_backend/**',async route=>{
  const request=route.request(),path=new URL(request.url()).pathname;
  if(offline){await route.fulfill({status:503,json:{}});return;}
  if(request.method()==='POST'&&path.endsWith('/turns'))sends++;
  if(path.endsWith('/stop')){stops++;state='cancelled';}
  if(path.endsWith('/recover')){recoveries++;state='done';}
  const body=path.endsWith('/status')?{state:'ready'}:path.endsWith('/conversations')?{conversations:[conversation]}:{conversation,turns:[{...turn,state,output:state==='done'?'Recovered original reply':''}],truncated:false};
  await route.fulfill({json:body});
 });
 await page.goto('http://127.0.0.1:4180/tests/assistant.html');
 await expect(page.getByRole('button',{name:'Recover original request'})).toBeVisible();
 await page.reload();
 await expect(page.getByRole('button',{name:'Recover original request'})).toBeVisible();
 assert.equal(sends,0,'Reading an unknown outcome must not resubmit');
 await page.getByRole('button',{name:'Recover original request'}).click();
 await expect(page.getByText('Recovered original reply',{exact:true})).toBeVisible();
 assert.equal(recoveries,1,'Recovery must be an explicit addressed operation');
 state='running';
 await expect(page.getByRole('button',{name:'Stop response'})).toBeVisible({timeout:10000});
 await page.getByRole('button',{name:'Stop response'}).click();
 await expect(page.getByText('Response stopped.',{exact:true})).toBeVisible();
 assert.equal(stops,1,'Stop must address the displayed turn');
 offline=true;
 await expect(page.getByRole('alert')).toBeVisible({timeout:10000});
 offline=false;
 await expect(page.getByRole('alert')).toHaveCount(0,{timeout:10000});
 await page.getByLabel('Message',{exact:true}).fill('Private unsent draft');
 await page.evaluate(()=>window.dispatchEvent(new CustomEvent('assistant-fixture-account',{detail:null})));
 await expect(page.getByText('Sign in below to start a conversation.')).toBeVisible();
 await expect(page.getByText('Original request',{exact:true})).toHaveCount(0);
 await page.evaluate(value=>window.dispatchEvent(new CustomEvent('assistant-fixture-account',{detail:{...value,accountId:'second-account',sessionId:'second-session'}})),principal);
 await expect(page.getByLabel('Message',{exact:true})).toHaveValue('');
 assert.equal(sends,0,'Account changes must not dispatch a retained draft');
 console.log(JSON.stringify({unknownNeverAutoResends:true,explicitRecovery:true,addressedStop:true,pollErrorClears:true,signoutClearsHistory:true,accountSwitchClearsDraft:true,liveBackend:false,nativeIPC:'substitute'}));
}finally{await browser.close();}
