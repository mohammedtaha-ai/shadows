import {test,expect} from '@playwright/test';
test.beforeEach(async({page})=>{
  await page.addInitScript(()=>{
    (window as any).draws=0;
    const original=CanvasRenderingContext2D.prototype.clearRect;
    CanvasRenderingContext2D.prototype.clearRect=function(...args){(window as any).draws++;return original.apply(this,args)};
  });
  await page.goto('/tests/companion/fixture.html?nativeVisibility');
});
test('native denial stops all draws while document remains visible',async({page})=>{
  await expect(page.getByText('Canvas ready',{exact:true})).toBeVisible();
  await expect.poll(()=>page.evaluate(()=>(window as any).draws)).toBeGreaterThan(0);
  await page.getByRole('button',{name:'Deny drawing',exact:true}).click();
  await page.clock.install();await page.clock.runFor(32);
  const before=await page.evaluate(()=>(window as any).draws);
  await page.getByRole('button',{name:'Change input',exact:true}).click();
  await page.setViewportSize({width:630,height:500});await page.clock.runFor(1000);
  expect(await page.evaluate(()=>document.visibilityState)).toBe('visible');
  expect(await page.evaluate(()=>(window as any).draws)).toBe(before);
  await page.getByRole('button',{name:'Pause fixture',exact:true}).click();
  await page.getByRole('button',{name:'Allow drawing',exact:true}).click();
  await page.clock.runFor(32);expect(await page.evaluate(()=>(window as any).draws)).toBe(before+1);
  await page.clock.runFor(1000);expect(await page.evaluate(()=>(window as any).draws)).toBe(before+1);
});
test('initialization reports ready without a draw under denied permission',async({page})=>{
  await page.goto('/tests/companion/fixture.html?nativeVisibility&denied');
  await expect(page.getByText('Canvas ready',{exact:true})).toBeVisible();
  expect(await page.evaluate(()=>(window as any).draws)).toBe(0);
});
test('reduced motion restore paints once and unmount leaves no frames',async({page})=>{
  await page.emulateMedia({reducedMotion:'reduce'});
  await page.getByRole('button',{name:'Deny drawing',exact:true}).click();
  await page.clock.install();await page.clock.runFor(32);
  const before=await page.evaluate(()=>(window as any).draws);
  await page.getByRole('button',{name:'Allow drawing',exact:true}).click();await page.clock.runFor(32);
  expect(await page.evaluate(()=>(window as any).draws)).toBe(before+1);
  await page.getByRole('button',{name:'Unmount canvas',exact:true}).click();await page.clock.runFor(1000);
  expect(await page.evaluate(()=>(window as any).draws)).toBe(before+1);
});
