// Test-owned development WebView2 only. No debug endpoint or globals are installed by product code.
import {chromium} from '@playwright/test';
import assert from 'node:assert/strict';
import {writeFileSync} from 'node:fs';
import {execFileSync} from 'node:child_process';
const args=Object.fromEntries(process.argv.slice(2).reduce((a,v,i,all)=>i%2===0?[...a,[v.replace(/^--/,''),all[i+1]]]:a,[]));
const report={focusCycles:[],hiddenFrames:null,observations:[],profiles:[]};
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
const shell=code=>execFileSync('pwsh',['-NoProfile','-Command',code],{windowsHide:true,encoding:'utf8',timeout:20000}).trim();
const foreground=()=>Number(shell("Add-Type 'using System;using System.Runtime.InteropServices;public class FG{[DllImport(\"user32.dll\")]public static extern IntPtr GetForegroundWindow();}';[FG]::GetForegroundWindow().ToInt64()"));
const cpu=()=>JSON.parse(shell(`$rootProcess=${Number(args.pid)};$all=Get-CimInstance Win32_Process;$ids=[Collections.Generic.HashSet[int]]::new();$null=$ids.Add($rootProcess);do{$count=$ids.Count;foreach($item in $all){if($ids.Contains([int]$item.ParentProcessId)){$null=$ids.Add([int]$item.ProcessId)}}}while($ids.Count -ne $count);$rows=@(foreach($id in $ids){$item=Get-Process -Id $id -ErrorAction SilentlyContinue;if($item){[pscustomobject]@{id=$id;start=$item.StartTime.ToUniversalTime().Ticks.ToString();cpu=$item.CPU;name=$item.ProcessName}}});ConvertTo-Json -InputObject $rows -Compress`));
let browser;
try {
  for(let i=0;i<40;i++){try{browser=await chromium.connectOverCDP(`http://127.0.0.1:${Number(args.port)}`);break}catch{await sleep(250)}}
  assert.ok(browser,'owned WebView2 endpoint unavailable');
  const context=browser.contexts()[0];
  let main,companion;
  for(let attempt=0;attempt<40;attempt++){
    for(const page of context.pages()){
      const label=await page.evaluate(()=>window.__TAURI_INTERNALS__?.metadata?.currentWindow?.label).catch(()=>null);
      if(label==='main')main=page;
      if(label==='companion')companion=page;
    }
    if(main&&companion)break;
    await sleep(100);
  }
  assert.ok(main&&companion,'both native surfaces must exist');
  const invoke=(page,command,payload={})=>page.evaluate(({command,payload})=>window.__TAURI_INTERNALS__.invoke(command,payload),{command,payload});
  const waitState=async(predicate,attempts=60)=>{for(let i=0;i<attempts;i++){const state=await invoke(main,'companion_state');if(predicate(state))return state;await sleep(100)}throw Error('companion state deadline')};
  await companion.addInitScript(()=>{window.nativeDraws=0;const clear=CanvasRenderingContext2D.prototype.clearRect;CanvasRenderingContext2D.prototype.clearRect=function(...args){window.nativeDraws++;return clear.apply(this,args)}});
  await companion.reload();await waitState(s=>s.visible&&s.renderAllowed);await sleep(500);
  await companion.screenshot({path:args.output.replace(/\.json$/,'.png'),omitBackground:true});
  assert.ok(await companion.evaluate(()=>window.nativeDraws)>0);
  report.hideReply=await invoke(main,'set_companion_visible',{visible:false});await sleep(150);
  report.actualStateAfterHide=await invoke(main,'companion_state');
  report.companionDocument=await companion.evaluate(()=>({url:location.href,scripts:[...document.scripts].map(s=>s.src),metadata:window.__TAURI_INTERNALS__.metadata}));
  const before=await companion.evaluate(()=>window.nativeDraws);
  const documentVisibility=await companion.evaluate(()=>document.visibilityState);
  await sleep(1000);const after=await companion.evaluate(()=>window.nativeDraws);
  report.hiddenFrames={before,after,documentVisibility};assert.equal(after,before);
  await invoke(main,'set_companion_visible',{visible:true});await sleep(300);
  assert.ok(await companion.evaluate(()=>window.nativeDraws)>after);
  for(const command of ['auth_action','cancel_login','hide_main','set_companion_visible','set_companion_paused']){
    let denied=false;try{await invoke(companion,command,{action:'status',visible:false,paused:true})}catch{denied=true}assert.ok(denied,`${command} must reject companion`);
  }
  for(const command of ['companion_send','companion_input','companion_ready','companion_failed','drag_companion','open_main_from_companion']){
    let denied=false;try{await invoke(main,command)}catch{denied=true}assert.ok(denied,`${command} must reject main`);
  }
  report.observations.push({scenario:'actual-native-ipc-scope',passed:true});
  const cycles=args.cycles===undefined?10:Number(args.cycles);
  for(let i=0;i<cycles;i++){
    await invoke(main,'set_companion_visible',{visible:false});
    const before=foreground();assert.equal(before,Number(args.foreground),'owned foreground test window required');
    await invoke(main,'set_companion_visible',{visible:true});await sleep(100);
    const after=foreground();report.focusCycles.push({before,after});assert.equal(after,before);
  }
  report.focusCyclesRequested=cycles;
  // Explicit input alone may activate the existing native surface.
  const info=()=>JSON.parse(shell(`Add-Type 'using System;using System.Runtime.InteropServices;public class InputWin{[StructLayout(LayoutKind.Sequential)]public struct Rect{public int L,T,R,B;}[DllImport("user32.dll")]public static extern bool GetWindowRect(IntPtr h,out Rect r);[DllImport("user32.dll")]public static extern int GetWindowLongW(IntPtr h,int n);[DllImport("user32.dll")]public static extern bool IsWindowVisible(IntPtr h);}';$r=[InputWin+Rect]::new();$h=[IntPtr]${Number(args['companion-hwnd'])};[InputWin]::GetWindowRect($h,[ref]$r)|Out-Null;@{x=$r.L;y=$r.T;width=$r.R-$r.L;height=$r.B-$r.T;style=[InputWin]::GetWindowLongW($h,-20);main=[InputWin]::IsWindowVisible([IntPtr]${Number(args['main-hwnd'])})}|ConvertTo-Json -Compress`));
  const witness=()=>shell(`Add-Type '[System.Runtime.InteropServices.StructLayout(System.Runtime.InteropServices.LayoutKind.Sequential)]public struct Unused{public int X;}public class InputFocus{[System.Runtime.InteropServices.DllImport("user32.dll")]public static extern bool SetForegroundWindow(System.IntPtr h);}';[InputFocus]::SetForegroundWindow([IntPtr]${Number(args.foreground)})|Out-Null`);
  const closed=info();
  const opened=await invoke(companion,'companion_input',{open:true});
  assert.equal(opened.composer.open,true);
  await sleep(200);
  const openInfo=info(),dpr=await companion.evaluate(()=>devicePixelRatio);
  report.inputProbe={closed,opened,openInfo,foreground:foreground()};
  assert.equal(foreground(),Number(args['companion-hwnd']));
  assert.equal(openInfo.style&0x08000000,0,'open input must clear WS_EX_NOACTIVATE');
  assert.equal(openInfo.width,Math.round(296*dpr));assert.equal(openInfo.height,Math.round(320*dpr));
  assert.equal(openInfo.main,false);
  assert.ok(openInfo.x>=Number(args['area-left'])&&openInfo.y>=Number(args['area-top']));
  assert.ok(openInfo.x+openInfo.width<=Number(args['area-right'])&&openInfo.y+openInfo.height<=Number(args['area-bottom']));
  if(args.typing==='true'){
    await invoke(companion,'companion_input',{open:false});witness();
    await companion.getByRole('button',{name:'Talk to AYLUL',exact:true}).click();
    const field=companion.getByRole('textbox',{name:'Message AYLUL',exact:true});
    await field.waitFor({state:'visible'});await sleep(150);
    shell("Add-Type -AssemblyName System.Windows.Forms;[Windows.Forms.SendKeys]::SendWait('ab')");
    assert.equal(await field.inputValue(),'ab','physical keyboard must reach the actual composer');
    await field.fill('');
    shell("Add-Type -AssemblyName System.Windows.Forms;[Windows.Forms.SendKeys]::SendWait('{ESC}')");
    await waitState(s=>!s.composer.open);
    await sleep(100);assert.equal(foreground(),Number(args.foreground),'Escape restores the prior external app');
    report.observations.push({scenario:'actual-composer-physical-keyboard-and-escape',passed:true});
  }
  await invoke(companion,'companion_input',{open:false});await sleep(200);
  const restored=info();
  assert.notEqual(restored.style&0x08000000,0,'closed companion must be non-activating');
  for(const key of ['x','y','width','height'])assert.equal(restored[key],closed[key]);
  assert.equal(restored.main,false);witness();await sleep(100);
  assert.equal(foreground(),Number(args.foreground));
  await invoke(companion,'companion_input',{open:true});
  await invoke(main,'set_companion_visible',{visible:false});
  witness();await sleep(100);const beforeShow=foreground();
  const shown=await invoke(main,'set_companion_visible',{visible:true});
  assert.equal(shown.composer.open,false);assert.equal(foreground(),beforeShow);
  report.observations.push({scenario:'explicit-input-focus-and-restored-bounds',passed:true,closed,openInfo,restored});
  if(args.message){
    const bound=await invoke(main,'companion_binding');
    assert.ok(bound.agent&&bound.target,'real send requires the existing explicit binding');
    const prior=await companion.locator('canvas').count();assert.equal(prior,1);
    await companion.getByRole('button',{name:'Talk to AYLUL',exact:true}).click();
    const field=companion.getByRole('textbox',{name:'Message AYLUL',exact:true});
    await field.fill(args.message);
    await companion.getByRole('button',{name:'Send message',exact:true}).click();
    await field.waitFor({state:'visible'});
    for(let i=0;i<400;i++){
      const state=await invoke(main,'companion_state');
      const value=await field.inputValue();
      if(!value&&state.presence.preview){
        report.realSend={passed:true,agent:bound.agent,threadId:bound.target.threadId,previewId:state.presence.preview.id,previewScalars:[...state.presence.preview.text].length};break;
      }
      const error=await companion.getByRole('status',{name:'Message status',exact:true}).innerText();
      if(error&&!['Sending…','AYLUL is answering.'].includes(error))throw Error('real send refused: '+error);
      await sleep(300);
    }
    assert.equal(report.realSend?.passed,true,'real provider completion preview deadline');
    const afterSend=await invoke(main,'companion_binding');assert.deepEqual(afterSend.target,bound.target);assert.equal(afterSend.agent,bound.agent);
    assert.equal(info().main,false);await companion.screenshot({path:args.output.replace(/\.json$/, '-real-send.png'),omitBackground:true});
    await invoke(companion,'companion_input',{open:false});
  }
  // A page-local Canvas failure must be recoverable without restarting the app.
  await companion.evaluate(()=>{CanvasRenderingContext2D.prototype.clearRect=()=>{throw Error('controlled render failure')}});
  await waitState(s=>s.error==='companion_render_failed');
  await invoke(main,'set_companion_visible',{visible:true});
  for(let i=0;i<60;i++){companion=context.pages().find(p=>!p.isClosed()&&p.url().includes('companion.html'));if(companion){const state=await invoke(main,'companion_state');if(state.visible&&state.renderAllowed)break;}await sleep(150)}
  await waitState(s=>s.visible&&s.renderAllowed&&!s.error);
  assert.equal(context.pages().filter(p=>p.url().includes('companion.html')).length,1);
  report.observations.push({scenario:'canvas-failure-reconstruction',passed:true});
  if(args.profile==='true')for(const mode of ['visible-idle','paused','hidden']){
    await invoke(main,'set_companion_visible',{visible:mode!=='hidden'});await invoke(main,'set_companion_paused',{paused:mode==='paused'});await sleep(500);
    for(let i=0;i<5;i++){
      const start=cpu();const now=performance.now();await sleep(10000);const end=cpu();const seconds=(performance.now()-now)/1000;
      const matches=start.map(row=>[row,end.find(item=>item.id===row.id&&item.start===row.start)]);
      const stable=matches.every(([,item])=>item)&&start.length===end.length;
      const cpuSeconds=matches.reduce((sum,[a,b])=>sum+(b?b.cpu-a.cpu:0),0);
      report.profiles.push({mode,seconds,complete:stable,cpuOneCorePercent:100*cpuSeconds/seconds,processes:end.map(({id,name})=>({id,name}))});
    }
  }
  report.passed=true;
}catch(error){report.passed=false;report.error=String(error);report.stack=error.stack;process.exitCode=1;}
finally{writeFileSync(args.output,JSON.stringify(report,null,2));await browser?.close();}
