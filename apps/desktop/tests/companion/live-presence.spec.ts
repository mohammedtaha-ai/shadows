import {test,expect} from '@playwright/test';
test.beforeEach(async({page})=>{
 await page.setViewportSize({width:240,height:240});
 await page.clock.install({time:new Date('2026-10-20T12:00:00Z')});
 await page.addInitScript(()=>{
  const callbacks=new Map<number,Function>();let next=1,listener=0;
  let state:any={revision:1,visible:true,renderAllowed:true,paused:false,error:null,presence:{expression:'idle',status:'Ready',preview:null}};
  const calls:string[]=[];let draws=0;
  const clear=CanvasRenderingContext2D.prototype.clearRect;
  CanvasRenderingContext2D.prototype.clearRect=function(...args){draws++;return clear.apply(this,args)};
  const publish=(change:any)=>{state={...state,...change,revision:state.revision+1};callbacks.get(listener)?.({event:'companion-state',id:listener,payload:state});};
  Object.assign(window,{__TAURI_EVENT_PLUGIN_INTERNALS__:{unregisterListener(){}},__TAURI_INTERNALS__:{
   metadata:{currentWindow:{label:'companion'},currentWebview:{label:'companion'}},
   transformCallback(fn:Function){const id=next++;callbacks.set(id,fn);return id;},unregisterCallback(id:number){callbacks.delete(id)},
   async invoke(command:string,args:any){
    calls.push(command);
    if(command==='plugin:event|listen'){listener=args.handler;return listener;}
    if(command==='plugin:event|unlisten')return;
    if(command==='companion_ready'||command==='companion_state')return state;
    if(command==='companion_failed')return {...state,renderAllowed:false,error:'companion_render_failed'};
    if(command==='open_main_from_companion'||command==='drag_companion')return;
    throw Error('Unexpected fixture command '+command);
   },
  },__fx:{publish,calls,draws:()=>draws,success(text:string,id='reply-1'){
   const preview={id,text,expiresAtMs:Date.now()+8000};publish({presence:{expression:'success',status:'Reply ready',preview}});
   setTimeout(()=>publish({presence:{expression:'idle',status:'Ready',preview}}),2000);
  }}});
 });
});
test('native status and a compact plain reply retain their deadlines across repeated events',async({page})=>{
 await page.goto('/companion.html');
 await expect(page.getByRole('status',{name:'AYLUL status',exact:true})).toHaveText('Ready');
 await page.evaluate(()=>(window as any).__fx.success('تم تجهيز الرد 🙂 '+ 'كلام طويل '.repeat(18)));
 const bubble=page.getByRole('status',{name:'AYLUL message',exact:true});
 await expect(bubble).toBeVisible();
 await expect(page.getByRole('status',{name:'AYLUL status',exact:true})).toHaveText('Reply ready');
 const bounds=await bubble.boundingBox();expect(bounds!.x).toBeGreaterThanOrEqual(0);expect(bounds!.x+bounds!.width).toBeLessThanOrEqual(240);
 expect(bounds!.height).toBeLessThanOrEqual(52);
 await page.screenshot({path:'../../output/playwright/companion-live-presence.png',omitBackground:true});
 await page.clock.fastForward(2000);
 await expect(page.getByRole('status',{name:'AYLUL status',exact:true})).toHaveText('Ready');
 await page.evaluate(()=>(window as any).__fx.success('تم تجهيز الرد 🙂','reply-1'));
 await page.clock.fastForward(6000);
 await expect(bubble).toHaveCount(0);
});
test('native hide stops drawing and showing cannot revive an expired preview',async({page})=>{
 await page.goto('/companion.html');await expect(page.locator('canvas')).toBeVisible();
 await page.evaluate(()=>(window as any).__fx.success('A short reply'));
 await expect(page.getByRole('status',{name:'AYLUL message',exact:true})).toBeVisible();
 await page.evaluate(()=>(window as any).__fx.publish({visible:false,renderAllowed:false}));
 await page.clock.runFor(50);const before=await page.evaluate(()=>(window as any).__fx.draws());
 await page.clock.fastForward(8100);expect(await page.evaluate(()=>(window as any).__fx.draws())).toBe(before);
 await page.evaluate(()=>(window as any).__fx.publish({visible:true,renderAllowed:true}));
 await expect(page.getByRole('status',{name:'AYLUL message',exact:true})).toHaveCount(0);
});
test('paused and reduced-motion status remains readable, keyboard opens and drag consumes the click',async({page})=>{
 await page.emulateMedia({reducedMotion:'reduce'});
 await page.goto('/companion.html');await expect(page.locator('canvas')).toBeVisible();
 await page.evaluate(()=>(window as any).__fx.publish({paused:true,presence:{expression:'working',status:'Working',preview:null}}));
 await expect(page.getByRole('status',{name:'AYLUL status',exact:true})).toHaveText('Working');
 const action=page.getByRole('button',{name:'Open NYLRA or drag AYLUL',exact:true});
 await action.focus();await action.press('Enter');await action.press('Space');
 expect(await page.evaluate(()=>(window as any).__fx.calls.filter((name:string)=>name==='open_main_from_companion').length)).toBe(2);
 await page.mouse.move(120,120);await page.mouse.down();await page.mouse.move(145,120);await page.mouse.up();
 expect(await page.evaluate(()=>(window as any).__fx.calls.filter((name:string)=>name==='open_main_from_companion').length)).toBe(2);
 expect(await page.evaluate(()=>(window as any).__fx.calls)).toContain('drag_companion');
});
