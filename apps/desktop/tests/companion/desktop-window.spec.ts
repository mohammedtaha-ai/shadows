import {test,expect} from '@playwright/test';
import {paletteFor} from '../../src/features/companion/renderer/palette.ts';
test.beforeEach(async({page})=>{
  await page.setViewportSize({width:240,height:240});
  await page.addInitScript(()=>{
    const callbacks=new Map<number,Function>();let next=1;
    const state={composer:{open:false,revision:0,scope:null,context:null,agent:null},revision:1,visible:true,renderAllowed:true,paused:false,error:null};
    Object.assign(window,{nativeCalls:[],nativeState:state,__TAURI_EVENT_PLUGIN_INTERNALS__:{unregisterListener(){}},__TAURI_INTERNALS__:{
      metadata:{currentWindow:{label:'companion'},currentWebview:{label:'companion'}},
      transformCallback(fn:Function){const id=next++;callbacks.set(id,fn);return id},unregisterCallback(id:number){callbacks.delete(id)},
      invoke:async(command:string,args:any)=>{
        (window as any).nativeCalls.push(command);
        if(command==='plugin:event|listen')return args.handler;
        if(command==='plugin:event|unlisten')return;
        if(command==='companion_ready'||command==='companion_state')return state;
        if(command==='companion_panel')return state;
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
  await page.mouse.click(120,120);await expect.poll(()=>page.evaluate(()=>(window as any).nativeCalls.includes('companion_panel'))).toBe(true);
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
test("the companion window draws the state's colour",async({page})=>{
  function hue(rgb:number[]){
    const [r,g,b]=rgb,max=Math.max(r,g,b),min=Math.min(r,g,b),d=max-min;
    return d===0?0:((max===r?(g-b)/d:max===g?(b-r)/d+2:(r-g)/d+4)*60+360)%360;
  }
  for(const color of ['blue',undefined]){
    await page.addInitScript(color=>{const state=(window as any).nativeState;if(color)state.appearance={color};else delete state.appearance;},color);
    await page.goto('/companion.html');await expect(page.locator('canvas')).toBeVisible();
    const hex=paletteFor(color).glow;
    const expected=hue([1,3,5].map(start=>parseInt(hex.slice(start,start+2),16)));
    await expect.poll(async()=>{
      const pixel=await page.locator('canvas').evaluate((canvas:HTMLCanvasElement)=>Array.from(canvas.getContext('2d')!.getImageData(Math.floor(canvas.width*0.2),Math.floor(canvas.height*0.5),1,1).data));
      if(pixel[3]<12)return 360;
      const delta=Math.abs(hue(pixel.slice(0,3))-expected);return Math.min(delta,360-delta);
    }).toBeLessThanOrEqual(4);
  }
});
test("the companion window draws the state's outfit",async({page})=>{
  await page.emulateMedia({reducedMotion:'reduce'});
  await page.addInitScript(()=>{Math.random=()=>0.5;});
  const images:string[]=[];
  for(const outfit of ['default','headphones','unknown-outfit']){
    await page.addInitScript(outfit=>{(window as any).nativeState.appearance={outfit};},outfit);
    await page.goto('/companion.html');
    await expect.poll(()=>page.locator('canvas').evaluate((canvas:HTMLCanvasElement)=>canvas.getContext('2d')!.getImageData(0,0,canvas.width,canvas.height).data.some((value,index)=>index%4===3&&value>0))).toBe(true);
    images.push(await page.locator('canvas').evaluate(async(canvas:HTMLCanvasElement)=>{
      const pixels=canvas.getContext('2d')!.getImageData(0,0,canvas.width,canvas.height).data;
      return Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',pixels))).map(value=>value.toString(16).padStart(2,'0')).join('');
    }));
  }
  expect(images[1]).not.toBe(images[0]);
  expect(images[2]).toBe(images[0]);
});
