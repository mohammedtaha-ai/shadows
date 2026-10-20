param([string]$RunName='independent-companion',[switch]$Profile,[switch]$WebViewOnly,[int]$FocusCycles=10)
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'windows-support.ps1')
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @'
using System;using System.Runtime.InteropServices;
public class CompanionWin {
 public delegate bool EnumProc(IntPtr h,IntPtr p);
 [DllImport("user32.dll")]public static extern bool EnumWindows(EnumProc f,IntPtr p);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)]public static extern int GetWindowTextW(IntPtr h,System.Text.StringBuilder s,int n);
 [DllImport("user32.dll")]public static extern bool GetWindowRect(IntPtr h,out Rect r);
 [StructLayout(LayoutKind.Sequential)] public struct Rect {public int L,T,R,B;}
 [DllImport("user32.dll")]public static extern bool GetClientRect(IntPtr h,out Rect r);
 [DllImport("user32.dll")]public static extern bool IsWindowVisible(IntPtr h);
 [DllImport("user32.dll")]public static extern bool PostMessageW(IntPtr h,uint m,IntPtr w,IntPtr l);
 [DllImport("user32.dll")]public static extern bool SetForegroundWindow(IntPtr h);
 [DllImport("user32.dll")]public static extern uint GetWindowThreadProcessId(IntPtr h,out uint p);
 [DllImport("user32.dll")]public static extern int GetWindowLongW(IntPtr h,int n);
}
'@
function GetOwnedRect([string]$role){
 $rows=[Collections.Generic.List[object]]::new()
 [CompanionWin]::EnumWindows({param($h,$p)
  $ownerId=0;[CompanionWin]::GetWindowThreadProcessId($h,[ref]$ownerId)|Out-Null
  $title=[Text.StringBuilder]::new(256);[CompanionWin]::GetWindowTextW($h,$title,256)|Out-Null
  if($ownerId -eq $process.Id -and $title.ToString() -eq 'AYLUL'){
   $r=[CompanionWin+Rect]::new();[CompanionWin]::GetWindowRect($h,[ref]$r)|Out-Null
   if($r.R -gt $r.L -and $r.B -gt $r.T){$rows.Add([pscustomobject]@{Handle=$h.ToInt64();Left=$r.L;Top=$r.T;Width=$r.R-$r.L;Height=$r.B-$r.T;Area=($r.R-$r.L)*($r.B-$r.T)})}
  };return $true
 },[IntPtr]::Zero)|Out-Null
 # Main is the ordinary application surface; companion is WS_EX_NOACTIVATE.
 $found=@($rows|Where-Object {(([CompanionWin]::GetWindowLongW([IntPtr]$_.Handle,-20) -band 0x08000000) -ne 0) -eq ($role -eq 'companion')}|Sort-Object Area -Descending)
 if(!$found.Count){throw "Owned window missing: $role"};return $found[0]
}
$out=New-NylraRunDir $RunName
if(Test-Path (Join-Path $out 'native.json')){throw 'Use a fresh RunName'}
$observations=[Collections.Generic.List[object]]::new()
function FindOwnedElement([string]$name,[Windows.Automation.ControlType]$type){
 if($name -eq 'NYLRA' -and $type -eq [Windows.Automation.ControlType]::Button){return Find-NylraTrayIcon}
 $a=[Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::NameProperty,$name)
 $b=[Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::ControlTypeProperty,$type)
 $condition=[Windows.Automation.AndCondition]::new($a,$b)
 if($name -eq 'NYLRA' -and $type -eq [Windows.Automation.ControlType]::Button){$condition=[Windows.Automation.AndCondition]::new($condition,[Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::AutomationIdProperty,'NotifyItemIcon'))}
 [Windows.Automation.AutomationElement]::RootElement.FindFirst([Windows.Automation.TreeScope]::Descendants,$condition)
}
function TrayItem([string]$name){
 foreach($menuAttempt in 1..5){
 [N0Win]::keybd_event(27,0,0,[UIntPtr]::Zero);[N0Win]::keybd_event(27,0,2,[UIntPtr]::Zero)
 $icon=FindOwnedElement 'NYLRA' ([Windows.Automation.ControlType]::Button)
 if(!$icon){$overflow=FindOwnedElement 'Show Hidden Icons' ([Windows.Automation.ControlType]::Button);if(!$overflow){throw 'Owned tray overflow unavailable'};$r=$overflow.Current.BoundingRectangle;Click-N0 ([int]($r.Left+$r.Width/2)) ([int]($r.Top+$r.Height/2));$icon=FindOwnedElement 'NYLRA' ([Windows.Automation.ControlType]::Button)}
 if(!$icon){throw 'Owned NYLRA icon unavailable'}
 $r=$icon.Current.BoundingRectangle;[N0Win]::SetCursorPos([int]($r.Left+$r.Width/2),[int]($r.Top+$r.Height/2))|Out-Null
 Start-Sleep -Milliseconds 500
 $icon=FindOwnedElement 'NYLRA' ([Windows.Automation.ControlType]::Button)
 if(!$icon){continue}
 $r=$icon.Current.BoundingRectangle;[N0Win]::SetCursorPos([int]($r.Left+$r.Width/2),[int]($r.Top+$r.Height/2))|Out-Null
 [N0Win]::mouse_event(8,0,0,0,[UIntPtr]::Zero);[N0Win]::mouse_event(16,0,0,0,[UIntPtr]::Zero);Start-Sleep -Milliseconds 300
 $item=$null;foreach($menuWait in 1..10){$item=FindOwnedElement $name ([Windows.Automation.ControlType]::MenuItem);if($item){break};Start-Sleep -Milliseconds 100}
 if($item){$r=$item.Current.BoundingRectangle;Click-N0 ([int]($r.Left+$r.Width/2)) ([int]($r.Top+$r.Height/2));return}
 }
 $menuCondition=[Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::ControlTypeProperty,[Windows.Automation.ControlType]::MenuItem)
 $menuNames=@([Windows.Automation.AutomationElement]::RootElement.FindAll([Windows.Automation.TreeScope]::Descendants,$menuCondition)|Where-Object {$_.Current.ProcessId -eq $process.Id}|ForEach-Object {$_.Current.Name})
 $observations.Add(@{Scenario='tray-item-diagnostic';Expected=$name;OwnedMenuItems=$menuNames})
 throw "Missing tray item $name after refreshing owned tray icons"
}
$oldArgs=$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
$listener=[Net.Sockets.TcpListener]::new([Net.IPAddress]::Loopback,0);$listener.Start();$port=$listener.LocalEndpoint.Port;$listener.Stop()
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--remote-debugging-port=$port"
$exe=Join-Path (Get-NylraTarget) 'debug/nylra-desktop.exe'
$process=Start-Process $exe -PassThru -WindowStyle Hidden
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=$oldArgs
$form=$null
try {
 Start-Sleep -Seconds 4
 $main=GetOwnedRect 'main';$character=GetOwnedRect 'companion';$mainH=[IntPtr]$main.Handle;$charH=[IntPtr]$character.Handle
 foreach($handle in @($mainH,$charH)){$ownerId=0;[CompanionWin]::GetWindowThreadProcessId($handle,[ref]$ownerId)|Out-Null;if($ownerId -ne $process.Id){throw 'Window is not owned by this test process'}}
 foreach($readyAttempt in 1..100){if([CompanionWin]::IsWindowVisible($charH)){break};Start-Sleep -Milliseconds 200}
 if(![CompanionWin]::IsWindowVisible($charH)){throw 'Companion did not become visible'}
 $style=[CompanionWin]::GetWindowLongW($charH,-16);$extended=[CompanionWin]::GetWindowLongW($charH,-20)
 $observations.Add(@{Scenario='native-window-style';Window=$character;Style=$style;ExtendedStyle=$extended})
 # Tao keeps style bits and removes non-client chrome through WM_NCCALCSIZE.
 $client=[CompanionWin+Rect]::new();if(![CompanionWin]::GetClientRect($charH,[ref]$client)){throw 'Client rectangle unavailable'}
 if(($client.R-$client.L) -ne $character.Width -or ($client.B-$client.T) -ne $character.Height -or ($extended -band 8) -eq 0){throw 'Companion has non-client frame or is not topmost'}
 $observations.Add(@{Scenario='native-companion-start';Passed=$true;Window=$character;Style=$style;ExtendedStyle=$extended;Binary=(Get-FileHash $exe).Hash})
 [CompanionWin]::PostMessageW($mainH,0x10,[IntPtr]::Zero,[IntPtr]::Zero)|Out-Null;Start-Sleep -Milliseconds 300
 if([CompanionWin]::IsWindowVisible($mainH) -or ![CompanionWin]::IsWindowVisible($charH)){throw 'Closing main affected companion'}
 $observations.Add(@{Scenario='close-main-independence';Passed=$true})
 if(!$WebViewOnly){
 $x=$character.Left+[int]($character.Width/2);$y=$character.Top+[int]($character.Height/2)
 [N0Win]::SetCursorPos($x,$y)|Out-Null;[N0Win]::mouse_event(2,0,0,0,[UIntPtr]::Zero)
 foreach($offset in 1..12){[N0Win]::SetCursorPos(($x-40*$offset),($y-4*$offset))|Out-Null;Start-Sleep -Milliseconds 50}
 [N0Win]::mouse_event(4,0,0,0,[UIntPtr]::Zero);Start-Sleep -Milliseconds 400
 $moved=GetOwnedRect 'companion'
 if($moved.Left -eq $character.Left -and $moved.Top -eq $character.Top){throw 'Real native drag did not move companion'}
 if([CompanionWin]::IsWindowVisible($mainH)){throw 'Drag incorrectly opened main'}
 $observations.Add(@{Scenario='actual-native-drag';Passed=$true;Before=$character;After=$moved})
 Click-N0 ($moved.Left+[int]($moved.Width/2)) ($moved.Top+[int]($moved.Height/2));Start-Sleep -Milliseconds 300
 if(![CompanionWin]::IsWindowVisible($mainH)){throw 'Short character click did not open main'}
 $observations.Add(@{Scenario='actual-character-click';Passed=$true})
 TrayItem 'Hide AYLUL';foreach($attempt in 1..20){if(![CompanionWin]::IsWindowVisible($charH)){break};Start-Sleep -Milliseconds 100};if([CompanionWin]::IsWindowVisible($charH)){throw 'Tray hide failed'}
 TrayItem 'Show AYLUL';foreach($attempt in 1..20){if([CompanionWin]::IsWindowVisible($charH)){break};Start-Sleep -Milliseconds 100};if(![CompanionWin]::IsWindowVisible($charH)){throw 'Tray show failed'}
 $observations.Add(@{Scenario='actual-tray-companion-toggle';Passed=$true})
 }
 $form=[Windows.Forms.Form]::new();$form.Text='NYLRA owned focus witness';$form.Width=340;$form.Height=100;$form.Show();[Windows.Forms.Application]::DoEvents();[CompanionWin]::SetForegroundWindow($form.Handle)|Out-Null
 $nodeArgs=@((Join-Path $PSScriptRoot 'companion-native.mjs'),'--port',"$port",'--output',(Join-Path $out 'native.json'),'--foreground',"$($form.Handle.ToInt64())",'--pid',"$($process.Id)",'--profile',"$($Profile.IsPresent.ToString().ToLower())",'--cycles',"$FocusCycles",'--companion-hwnd',"$($charH.ToInt64())",'--main-hwnd',"$($mainH.ToInt64())")
 $nodeStart=[Diagnostics.ProcessStartInfo]::new('node');$nodeStart.UseShellExecute=$false;$nodeStart.CreateNoWindow=$true
 foreach($nodeArg in $nodeArgs){$nodeStart.ArgumentList.Add($nodeArg)}
 $nodeChild=[Diagnostics.Process]::Start($nodeStart)
 # Keep the owned focus witness responsive; a blocked UI thread creates Windows
 # ghost windows and invalidates foreground-HWND comparisons.
 while(!$nodeChild.HasExited){[Windows.Forms.Application]::DoEvents();Start-Sleep -Milliseconds 50}
 if($nodeChild.ExitCode -ne 0){throw 'Native WebView checks failed; inspect native.json'}
 $observations.Add(@{Scenario='native-webview-state-recovery';Passed=$true;FocusCyclesRequested=$FocusCycles})
 TrayItem 'Quit NYLRA';if(!$process.WaitForExit(5000)){throw 'Quit did not exit owned application'}
 $observations.Add(@{Scenario='actual-tray-quit';Passed=$true})
}finally{
 $observations|ConvertTo-Json -Depth 7|Set-Content (Join-Path $out 'windows.json')
 if($form){$form.Close();$form.Dispose()}
 if(!$process.HasExited){Stop-Process -Id $process.Id;$process.WaitForExit(5000)|Out-Null}
}
