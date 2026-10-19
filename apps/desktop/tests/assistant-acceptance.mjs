// Explicit local account/provider journey; ordinary npm test never invokes it.
import {chromium} from '@playwright/test';
import {readFileSync,mkdirSync} from 'node:fs';
import assert from 'node:assert/strict';
const grant=JSON.parse(readFileSync('E:/Globalprojects/shadows-backend/.local-runtime/assistant-browser-grant.json','utf8'));
const browser=await chromium.launch({channel:'chrome',headless:true});
try{
 const context=await browser.newContext();await context.addInitScript(value=>sessionStorage.setItem('assistant-fixture-grant',JSON.stringify(value)),grant);
 const page=await context.newPage();let sends=0;page.on('request',request=>{if(request.method()==='POST'&&/\/turns$/.test(request.url()))sends++;});
 await page.goto('http://127.0.0.1:4180/tests/assistant.html');
 await page.getByRole('status').filter({hasText:'Connected'}).waitFor({timeout:30000});
 await page.getByLabel('Message',{exact:true}).fill('Please reply with exactly UI_CHAT_OK.');
 await page.getByRole('button',{name:'Send message',exact:true}).click();
 await page.getByText('UI_CHAT_OK',{exact:true}).waitFor({timeout:120000});assert.equal(sends,1);
 await page.reload();await page.getByText('UI_CHAT_OK',{exact:true}).waitFor({timeout:30000});assert.equal(sends,1);
 await page.getByLabel('Message',{exact:true}).fill('What exact marker did you reply with before? Reply only with it.');
 await page.getByRole('button',{name:'Send message',exact:true}).click();
 await page.waitForFunction(()=>[...document.querySelectorAll('.assistant-message.reply p')].filter(p=>p.textContent.trim()==='UI_CHAT_OK').length===2,{},{timeout:120000});
 assert.equal(sends,2);mkdirSync('tests/.runs/assistant',{recursive:true});await page.screenshot({path:'tests/.runs/assistant/conversation.png',fullPage:true});
 console.log(JSON.stringify({liveBackend:true,liveMiniMax:true,browserNativeBoundary:'substitute',messages:2,reloadRecovered:true,duplicateSubmissions:0}));
}finally{await browser.close();}
