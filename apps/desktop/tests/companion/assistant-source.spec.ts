import {test,expect} from '@playwright/test';
test('signed_out_local_chat_and_explicit_remote_switch_clear_the_draft',async({page})=>{
  await page.goto('/tests/companion/fixture.html?shell&localAssistant');
  const source=page.getByLabel('Assistant location',{exact:true});
  await expect(source).toHaveValue('local');
  await expect(page.getByText('Connected',{exact:true})).toBeVisible();
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
