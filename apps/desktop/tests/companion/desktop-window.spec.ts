import {test,expect} from '@playwright/test';
test.beforeEach(async({page})=>{
  await page.setViewportSize({width:240,height:240});
  await page.addInitScript(()=>{
    const callbacks=new Map<number,Function>();let next=1;
    const state={revision:1,visible:true,renderAllowed:true,paused:false,error:null};
    Object.assign(window,{nativeCalls:[],__TAURI_EVENT_PLUGIN_INTERNALS__:{unregisterListener(){}},__TAURI_INTERNALS__:{
      metadata:{currentWindow:{label:'companion'},currentWebview:{label:'companion'}},
      transformCallback(fn:Function){const id=next++;callbacks.set(id,fn);return id},unregisterCallback(id:number){callbacks.delete(id)},
      invoke:async(command:string,args:any)=>{
        (window as any).nativeCalls.push(command);
        if(command==='plugin:event|listen')return args.handler;
        if(command==='plugin:event|unlisten')return;
        if(command==='companion_ready'||command==='companion_state')return state;
        if(command==='companion_failed')return {...state,revision:2,renderAllowed:false,error:'companion_render_failed'};
        if(command==='drag_companion'||command==='open_main_from_companion')return;
        throw Error('Unexpected native call '+command);
      }
    }});
  });
});
test('desktop entry draws transparent character without account or studio',async({page})=>{
  await page.goto('/companion.html');await expect(page.locator('canvas')).toBeVisible();
  await expect.poll(()=>page.locator('canvas').evaluate((el:HTMLCanvasElement)=>el.getContext('2d')!.getImageData(0,0,el.width,el.height).data.some((v,i)=>i%4===3&&v>0))).toBe(true);
  expect(await page.locator('canvas').evaluate((el:HTMLCanvasElement)=>el.getContext('2d')!.getImageData(0,0,1,1).data[3])).toBe(0);
  expect(await page.evaluate(()=>({x:document.documentElement.scrollWidth,y:document.documentElement.scrollHeight}))).toEqual({x:240,y:240});
  await expect(page.getByRole('heading',{name:'Account',exact:true})).toHaveCount(0);
  expect(await page.evaluate(()=>(window as any).nativeCalls)).not.toContain('auth_action');
  await page.mouse.click(120,120);await expect.poll(()=>page.evaluate(()=>(window as any).nativeCalls.includes('open_main_from_companion'))).toBe(true);
  await page.screenshot({path:'test-results/desktop-240.png'});
});
test('desktop Canvas failure stays local and does not report readiness',async({page})=>{
  await page.addInitScript(()=>{HTMLCanvasElement.prototype.getContext=(()=>null) as any});
  await page.goto('/companion.html');await expect(page.getByRole('alert')).toContainText('unavailable');
  expect(await page.evaluate(()=>(window as any).nativeCalls)).not.toContain('companion_ready');
});
test('transient drag failure leaves the independent canvas mounted',async({page})=>{
  await page.goto('/companion.html');await expect(page.locator('canvas')).toBeVisible();
  await page.evaluate(()=>{const internal=(window as any).__TAURI_INTERNALS__;const original=internal.invoke;internal.invoke=(command:string,args:any)=>command==='drag_companion'?Promise.reject(Error('refused')):original(command,args)});
  await page.mouse.move(120,120);await page.mouse.down();await page.mouse.move(145,120);await page.mouse.up();
  await expect(page.getByRole('alert')).toContainText('Unable to move');
  await expect(page.locator('canvas')).toBeVisible();
});
test('drawing failure after readiness reports to native recovery owner',async({page})=>{
  await page.goto('/companion.html');await expect(page.locator('canvas')).toBeVisible();
  await expect.poll(()=>page.evaluate(()=>(window as any).nativeCalls.includes('companion_ready'))).toBe(true);
  await page.evaluate(()=>{CanvasRenderingContext2D.prototype.clearRect=()=>{throw Error('controlled render failure')}});
  await expect(page.getByRole('alert')).toContainText('unavailable');
  await expect.poll(()=>page.evaluate(()=>(window as any).nativeCalls.includes('companion_failed'))).toBe(true);
});
