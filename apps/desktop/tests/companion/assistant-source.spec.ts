import {test,expect} from '@playwright/test';
test('signed_out_local_chat_and_explicit_remote_switch_clear_the_draft',async({page})=>{
  await page.goto('/tests/companion/fixture.html?shell&localAssistant');
  const source=page.getByLabel('Assistant location',{exact:true});
  await expect(source).toHaveValue('local');
  await expect(page.getByText('Connected',{exact:true})).toBeVisible();
  await expect(page.getByText('Local Hermes · Model managed by Hermes',{exact:true})).toBeVisible();
  const input=page.getByLabel('Message',{exact:true});
  await input.fill('Local draft');
  await source.selectOption('remote');
  await expect(input).toBeHidden();
  await expect(page.getByText('Sign in below to start a conversation.')).toBeVisible();
  await source.selectOption('local');
  await expect(page.getByText('Connected',{exact:true})).toBeVisible();
  await expect(input).toHaveValue('');
});
test('late_local_snapshot_cannot_replace_remote_history_with_colliding_ids',async({page})=>{
  await page.goto('/tests/companion/fixture.html?shell&localAssistant&signedIn');
  await expect(page.getByText('Local answer',{exact:true})).toBeVisible();
  await page.evaluate(()=>{
    const fx=(window as any).__fx;
    fx.gate=new Promise<void>(resolve=>{fx.release=resolve;});
  });
  await page.getByRole('button',{name:'Refresh connections',exact:true}).click();
  await page.waitForTimeout(1700);
  await page.getByLabel('Assistant location',{exact:true}).selectOption('remote');
  await page.evaluate(()=>{const fx=(window as any).__fx;fx.gate=null;fx.release();});
  await expect(page.getByText('Alpha answer',{exact:true})).toBeVisible();
  await expect(page.getByText('Local answer',{exact:true})).toBeHidden();
});
test('remote_operations_capture_the_displayed_account_and_session',async({page})=>{
  await page.goto('/tests/companion/fixture.html?shell&signedIn');
  await expect(page.getByText('Alpha answer',{exact:true})).toBeVisible();
  const calls=await page.getByTestId('fixture-ipc-calls').textContent();
  const remote=calls!.split('\n').filter(line=>line.startsWith('assistant_call ')).map(line=>JSON.parse(line.slice('assistant_call '.length)));
  expect(remote.length).toBeGreaterThan(0);
  for(const args of remote){expect(args.expectedAccountId).toBe('acct-a');expect(args.expectedSessionId).toBe('sess-a');}
});
test('returning_to_a_source_does_not_accept_an_older_send_reply_or_clear_a_new_draft',async({page})=>{
  await page.goto('/tests/companion/fixture.html?shell&localAssistant');
  await expect(page.getByText('Connected',{exact:true})).toBeVisible();
  await page.evaluate(()=>{const fx=(window as any).__fx;fx.sendGate=new Promise<void>(resolve=>{fx.releaseSend=resolve;});});
  const source=page.getByLabel('Assistant location',{exact:true}),input=page.getByLabel('Message',{exact:true});
  await input.fill('Old local message');await page.getByRole('button',{name:'Send message',exact:true}).click();
  await expect.poll(()=>page.evaluate(()=>(window as any).__fx.calls.some((call:any)=>call.action==='send'))).toBe(true);
  await source.selectOption('remote');await expect(input).toBeHidden();
  await source.selectOption('local');await expect(page.getByText('Connected',{exact:true})).toBeVisible();
  await input.fill('New local draft');
  await page.evaluate(async()=>{(window as any).__fx.releaseSend();await new Promise(requestAnimationFrame);await new Promise(requestAnimationFrame);});
  await expect(input).toHaveValue('New local draft');
});
