// Test-owned native acceptance. This driver never sends launchAtLogin.
import {chromium} from '@playwright/test';
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
const args=Object.fromEntries(process.argv.slice(2).reduce((a,v,i,all)=>i%2===0?[...a,[v.replace(/^--/,''),all[i+1]]]:a,[]));
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
const psQuote=value=>"'"+String(value).replaceAll("'","''")+"'";
const shell=code=>execFileSync('pwsh',['-NoProfile','-Command',"$ErrorActionPreference='Stop';"+code],{windowsHide:true,encoding:'utf8',timeout:30000}).trim();
let pid=Number(args.pid);
assert.ok(Number.isSafeInteger(pid)&&pid>0,'a test-owned desktop PID is required');
const info=JSON.parse(shell(`Get-CimInstance Win32_Process -Filter "ProcessId=${pid}" | Select-Object ExecutablePath,CommandLine | ConvertTo-Json`));
assert.match(info.CommandLine,/--acceptance-root/,'refuse an ordinary desktop process');
const root=args.root??info.CommandLine.match(/--acceptance-root\s+(?:"([^"]+)"|(\S+))/)?.slice(1).find(Boolean);
assert.ok(root,'isolated acceptance root is required');
const port=Number(args['main-port']), companionPort=Number(args['companion-port']);
assert.equal(companionPort,port+1);
const support=fileURLToPath(new URL('windows-support.ps1',import.meta.url));
const native=`. ${psQuote(support)}; Add-Type -AssemblyName UIAutomationClient; Add-Type -AssemblyName UIAutomationTypes; Add-Type @'
using System;using System.Text;using System.Runtime.InteropServices;
public class PrefWin {
 [StructLayout(LayoutKind.Sequential)]public struct Rect{public int L,T,R,B;}
 public delegate bool Visitor(IntPtr h,IntPtr l);
 [DllImport("user32.dll")]public static extern bool EnumWindows(Visitor v,IntPtr l);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)]public static extern int GetWindowText(IntPtr h,StringBuilder t,int n);
 [DllImport("user32.dll",EntryPoint="GetWindowLongPtrW")]public static extern IntPtr GetWindowLongPtr(IntPtr h,int n);
 [DllImport("user32.dll")]public static extern bool PostMessageW(IntPtr h,uint m,IntPtr w,IntPtr l);
 [DllImport("user32.dll")]public static extern bool IsWindowVisible(IntPtr h);
 [DllImport("user32.dll")]public static extern bool GetWindowRect(IntPtr h,out Rect r);
 [DllImport("user32.dll")]public static extern uint GetWindowThreadProcessId(IntPtr h,out uint p);
 public static long Find(uint pid,bool main) {
  long result=0;
  EnumWindows((h,l)=>{uint p;GetWindowThreadProcessId(h,out p);var t=new StringBuilder(256);GetWindowText(h,t,256);
   bool decorated=(GetWindowLongPtr(h,-16).ToInt64()&0x00C00000)==0x00C00000;
   bool companion=(GetWindowLongPtr(h,-20).ToInt64()&0x08000000)!=0;
   Rect r;GetWindowRect(h,out r);
   bool sized=main?(r.R-r.L>=640 && r.B-r.T>=400):(r.R>r.L && r.B>r.T);
   if(p==pid && t.ToString()=="AYLUL" && sized && (main?decorated:companion)){result=h.ToInt64();return false;}return true;
  },IntPtr.Zero);return result;
 }
}
'@;
`;
function ownedHandle(surface){const handle=Number(shell(native+`[PrefWin]::Find(${pid},$${surface==='main'?'true':'false'})`));assert.ok(handle,`owned ${surface} window missing for PID ${pid}`);return handle;}
const alive=()=>shell(`$p=Get-Process -Id ${pid} -ErrorAction SilentlyContinue;if($p){'yes'}else{'no'}`)==='yes';
async function exited(){for(let i=0;i<40&&alive();i++)await sleep(100);assert.ok(!alive(),'application must exit including its companion');}
function menu(item){shell(native+`
$icon=Find-NylraTrayIcon
if(!$icon){
 $c=[Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::NameProperty,'Show Hidden Icons')
 $overflow=[Windows.Automation.AutomationElement]::RootElement.FindFirst([Windows.Automation.TreeScope]::Descendants,$c)
 if(!$overflow){throw 'Tray overflow unavailable'}
 $r=$overflow.Current.BoundingRectangle;Click-N0 ([int]($r.Left+$r.Width/2)) ([int]($r.Top+$r.Height/2));$icon=Find-NylraTrayIcon
}
if(!$icon){throw 'Owned tray icon unavailable'}
$r=$icon.Current.BoundingRectangle;[N0Win]::SetCursorPos([int]($r.Left+$r.Width/2),[int]($r.Top+$r.Height/2))|Out-Null
[N0Win]::mouse_event(8,0,0,0,[UIntPtr]::Zero);[N0Win]::mouse_event(16,0,0,0,[UIntPtr]::Zero);Start-Sleep -Milliseconds 400
$c=[Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::NameProperty,${psQuote(item)})
$m=[Windows.Automation.AutomationElement]::RootElement.FindFirst([Windows.Automation.TreeScope]::Descendants,$c)
if(!$m -or $m.Current.ProcessId -ne ${pid}){throw 'Owned menu item unavailable'}
$r=$m.Current.BoundingRectangle;Click-N0 ([int]($r.Left+$r.Width/2)) ([int]($r.Top+$r.Height/2))
`);}
let browsers=[];
async function connect(endpoint){
 for(let i=0;i<60;i++){
  try {const browser=await chromium.connectOverCDP(`http://127.0.0.1:${endpoint}`);browsers.push(browser);const page=browser.contexts()[0].pages()[0];await page.waitForFunction(()=>!!window.__TAURI_INTERNALS__,null,{timeout:5000});return page;}catch{await sleep(200);}
 }
 throw Error('test-owned WebView2 endpoint unavailable');
}
const invoke=(page,command,payload={})=>page.evaluate(({command,payload})=>window.__TAURI_INTERNALS__.invoke(command,payload),{command,payload});
async function pages(){return [await connect(port),await connect(companionPort)];}
async function disconnect(){for(const browser of browsers)await browser.close().catch(()=>{});browsers=[];}
async function restart(){
 await disconnect();
 pid=Number(shell(`$p=Start-Process -FilePath ${psQuote(info.ExecutablePath)} -ArgumentList @('--acceptance-root',${psQuote('"'+root+'"')},'--acceptance-debug-port','${port}') -PassThru -WindowStyle Hidden;$p.Id`));
 startedPids.push(pid);console.log(`STARTED acceptance PID ${pid}`);
 return pages();
}
const startedPids=[pid];
const forcedCleanup=[];
const results=[];
const pass=name=>{results.push(name);console.log('PASS '+name);};
try {
 let [main,companion]=await pages();
 const initial=await invoke(main,'desktop_preferences');
 assert.equal(typeof initial.launchAtLogin,'boolean');assert.equal(initial.closeToTray,true);
 assert.deepEqual(await invoke(main,'set_desktop_preferences',{}),initial);
 const patch=await invoke(main,'set_desktop_preferences',{closeToTray:true});
 assert.deepEqual(patch,initial);pass('main can read and patch preferences');
 for(const command of ['desktop_preferences','set_desktop_preferences','refresh_companion_appearance']) {
  await assert.rejects(invoke(companion,command,{}));
 }
 pass('companion cannot invoke preferences');
 // The unbound refresh also exercises N-6 through the real command/ACL/publication path.
 assert.deepEqual(await invoke(main,'refresh_companion_appearance'),{ok:true});
 pass('N-6 startup unbound refresh acknowledges native publication');
 menu('Quit NYLRA');await exited();
 [main,companion]=await restart();
 assert.equal((await invoke(main,'desktop_preferences')).closeToTray,true);
 const mainHandle=ownedHandle('main');
 shell(native+`[PrefWin]::PostMessageW([IntPtr]${mainHandle},0x10,[IntPtr]::Zero,[IntPtr]::Zero)|Out-Null`);
 await sleep(300);assert.ok(alive());
 assert.equal(shell(native+`[PrefWin]::IsWindowVisible([IntPtr]${mainHandle})`),'False');
 menu('Open NYLRA');
 assert.equal(shell(native+`[PrefWin]::IsWindowVisible([IntPtr]${mainHandle})`),'True');
 pass('close to tray survives restart');
 await invoke(main,'set_companion_visible',{visible:true});
 assert.equal(shell(native+`[PrefWin]::IsWindowVisible([IntPtr]${ownedHandle('companion')})`),'True');
 await invoke(main,'set_desktop_preferences',{closeToTray:false});
 shell(native+`[PrefWin]::PostMessageW([IntPtr]${mainHandle},0x10,[IntPtr]::Zero,[IntPtr]::Zero)|Out-Null`);
 await exited();pass('close without tray exits with companion visible');
 [main,companion]=await restart();
 assert.equal((await invoke(main,'desktop_preferences')).closeToTray,false);
 assert.equal((await invoke(main,'desktop_preferences')).launchAtLogin,initial.launchAtLogin);
 menu('Quit NYLRA');await exited();pass('tray Quit ignores close preference');
 console.log(JSON.stringify({passed:results.length,results,loginItem:'unmodified; registration and sign-in reserved for Mohammed'}));
} finally {
 if(alive()) {
  try {const handle=ownedHandle('main');shell(native+`[PrefWin]::PostMessageW([IntPtr]${handle},0x10,[IntPtr]::Zero,[IntPtr]::Zero)|Out-Null`);}catch(error){console.error('Normal cleanup close:',error.message);}
  await sleep(1000);
  if(alive()) {assert.ok(startedPids.includes(pid));shell(`Stop-Process -Id ${pid} -ErrorAction Stop`);forcedCleanup.push(pid);console.log(`FORCED CLEANUP of test-owned PID ${pid}`);}
 }
 await disconnect();
 console.log(JSON.stringify({startedPids,forcedCleanup}));
}
