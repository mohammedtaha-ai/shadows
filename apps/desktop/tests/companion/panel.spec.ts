import {test,expect} from '@playwright/test';
import {installComposerNative} from './fakes.ts';

test.beforeEach(async({page})=>{
  await page.setViewportSize({width:640,height:520});
  await page.addInitScript(installComposerNative);
});
async function openPanel(page:import('@playwright/test').Page){
  await page.goto('/companion.html');
  await expect(page.locator('canvas').first()).toBeVisible();
  await page.locator('.aylul-artwork').click({position:{x:120,y:120}});
  await expect(page.getByRole('region',{name:'AYLUL panel',exact:true})).toBeVisible();
}
test('click and Enter open the panel and Escape closes it',async({page})=>{
  await openPanel(page);
  await page.keyboard.press('Escape');
  await expect(page.getByRole('region',{name:'AYLUL panel',exact:true})).toHaveCount(0);
  await page.getByRole('button',{name:'Open AYLUL panel or drag AYLUL',exact:true}).focus();
  await page.keyboard.press('Enter');
  await expect(page.getByRole('region',{name:'AYLUL panel',exact:true})).toBeVisible();
});
test('the grid draws agents and selection uses the native binding operation',async({page})=>{
  await openPanel(page);
  const grid=page.getByRole('group',{name:'Agents',exact:true});
  await expect(grid.getByRole('button',{name:'AYLUL',exact:true})).toHaveAttribute('aria-pressed','true');
  await expect(grid.locator('canvas')).toHaveCount(2);
  await expect(grid.getByRole('button').last()).toHaveAccessibleName('Create a new agent');
  await expect(page.getByRole('region',{name:'Bound agent',exact:true}).locator('canvas')).toHaveCount(1);
  await grid.getByRole('button',{name:'Researcher',exact:true}).click();
  await expect(grid.getByRole('button',{name:'Researcher',exact:true})).toHaveAttribute('aria-pressed','true');
  expect(await page.evaluate(()=>(window as any).__composerFx.calls.some((call:any)=>call.command==='select_companion_agent'&&call.args.agent==='researcher'))).toBe(true);
  await expect(page.getByRole('region',{name:'Bound agent',exact:true})).toContainText('Finds sources');
  await page.screenshot({path:'test-results/panel-neon.png'});
});
test('nine agents scroll within the grid',async({page})=>{
  await openPanel(page);
  await page.evaluate(()=>{const fx=(window as any).__composerFx;fx.publish({panel:{open:true,agents:Array.from({length:9},(_,index)=>({slug:`agent-${index}`,name:`Agent ${index}`,description:'Helps',color:'blue',outfit:'crown',bound:index===0}))}})});
  const grid=page.getByRole('group',{name:'Agents',exact:true});
  await expect(grid.getByRole('button')).toHaveCount(10);
  expect(await grid.evaluate(el=>el.scrollHeight>el.clientHeight)).toBe(true);
});
test('Settings New agent and Open request their closed native destinations',async({page})=>{
  await openPanel(page);
  for(const [name,target] of [['Settings','settings'],['New agent','newAgent'],['Create a new agent','newAgent'],['Agent settings','agentSettings'],['Open','bound']]){
    await page.getByRole('button',{name,exact:true}).click();
    expect(await page.evaluate(()=>(window as any).__composerFx.calls.at(-1))).toEqual({command:'open_main_from_companion',args:{target}});
  }
});
test('Talk and Chat open the composer and Sound toggles tones',async({page})=>{
  await openPanel(page);
  await page.getByRole('button',{name:'Sound',exact:true}).click();
  await expect(page.getByRole('button',{name:'Sound',exact:true})).toHaveAttribute('aria-pressed','true');
  await page.getByRole('button',{name:'Sound',exact:true}).click();
  await expect(page.getByRole('button',{name:'Sound',exact:true})).toHaveAttribute('aria-pressed','false');
  await page.getByRole('button',{name:'Talk',exact:true}).click();
  await expect(page.getByRole('textbox',{name:'Message AYLUL',exact:true})).toBeVisible();
  await page.keyboard.press('Escape');
  await page.locator('.aylul-artwork').click({position:{x:120,y:120}});
  await page.getByRole('button',{name:'Chat',exact:true}).click();
  await expect(page.getByRole('textbox',{name:'Message AYLUL',exact:true})).toBeVisible();
});
