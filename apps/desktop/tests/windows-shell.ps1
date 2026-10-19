param([string]$RunName='windows-shell')
# Real Windows shell acceptance. No direct invocation of tray callbacks.
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'windows-support.ps1')
$out=New-NylraRunDir $RunName
if(Test-Path (Join-Path $out 'shell-observations.json')){throw 'Use a fresh RunName to preserve evidence'}
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @'
using System;using System.Runtime.InteropServices;
public class NylraShell {
 [StructLayout(LayoutKind.Sequential)]public struct Rect {public int L,T,R,B;}
 [StructLayout(LayoutKind.Sequential)]public struct MonitorInfo {public int Size;public Rect Monitor;public Rect Work;public uint Flags;}
 [DllImport("user32.dll")]public static extern IntPtr MonitorFromWindow(IntPtr h,uint flags);
 [DllImport("user32.dll")]public static extern bool GetMonitorInfoW(IntPtr h,ref MonitorInfo info);
 [DllImport("user32.dll")]public static extern bool IsWindowVisible(IntPtr h);
 [DllImport("user32.dll")]public static extern bool PostMessageW(IntPtr h,uint msg,IntPtr w,IntPtr l);
}
'@
function Find-OwnedButton([string]$Name) {
 if($Name -eq 'NYLRA'){return Find-NylraTrayIcon}
 $nameCondition=New-Object Windows.Automation.PropertyCondition ([Windows.Automation.AutomationElement]::NameProperty),$Name
 $typeCondition=New-Object Windows.Automation.PropertyCondition ([Windows.Automation.AutomationElement]::ControlTypeProperty),([Windows.Automation.ControlType]::Button)
 $condition=New-Object Windows.Automation.AndCondition $nameCondition,$typeCondition
 if($Name -eq 'NYLRA'){$id=[Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::AutomationIdProperty,'NotifyItemIcon');$condition=[Windows.Automation.AndCondition]::new($condition,$id)}
 [Windows.Automation.AutomationElement]::RootElement.FindFirst([Windows.Automation.TreeScope]::Descendants,$condition)
}
function Shortcut {
 foreach($key in @(0x11,0x12,0x10,0x7B)){[N0Win]::keybd_event($key,0,0,[UIntPtr]::Zero)}
 foreach($key in @(0x7B,0x10,0x12,0x11)){[N0Win]::keybd_event($key,0,2,[UIntPtr]::Zero)}
 Start-Sleep -Milliseconds 700
}
$exe=Join-Path (Get-NylraTarget) 'debug/nylra-desktop.exe'
$p=Start-Process $exe -PassThru -WindowStyle Hidden
$results=[Collections.Generic.List[object]]::new()
try {
 Start-Sleep -Seconds 4;$w=Get-N0Rect 'NYLRA';$h=[IntPtr]$w.Handle
 # Verify main controls with the optional companion independently hidden.
 $character=[N0Win]::FindWindow([IntPtr]::Zero,'AYLUL');if($character -ne [IntPtr]::Zero){[NylraShell]::PostMessageW($character,0x10,[IntPtr]::Zero,[IntPtr]::Zero)|Out-Null;Start-Sleep -Milliseconds 300}
 $monitor=New-Object NylraShell+MonitorInfo;$monitor.Size=[Runtime.InteropServices.Marshal]::SizeOf($monitor)
 if(![NylraShell]::GetMonitorInfoW([NylraShell]::MonitorFromWindow($h,2),[ref]$monitor)){throw 'Native work area unavailable'}
 $fits=$w.Left -ge $monitor.Work.L -and $w.Top -ge $monitor.Work.T -and ($w.Left+$w.Width) -le $monitor.Work.R -and ($w.Top+$w.Height) -le $monitor.Work.B
 $results.Add([pscustomobject]@{Scenario='window-fits-native-work-area';Passed=$fits;Window=$w;WorkArea=$monitor.Work})
 if(!$fits){throw 'Main window extends outside native work area'}
 $results.Add([pscustomobject]@{Scenario='native-startup';Window=$w;Visible=[NylraShell]::IsWindowVisible($h);SHA256=(Get-FileHash $exe).Hash})
 Save-N0Window 'NYLRA' (Join-Path $out 'shell-main.png')|Out-Null
 [NylraShell]::PostMessageW($h,0x10,[IntPtr]::Zero,[IntPtr]::Zero)|Out-Null;Start-Sleep -Milliseconds 700
 if([NylraShell]::IsWindowVisible($h)){throw 'Native close did not hide to tray'}
 $icon=Find-OwnedButton 'NYLRA';$overflowOpened=$false
 if(!$icon){foreach($name in @('Show Hidden Icons','Show hidden icons','Hidden icon menu')){$overflow=Find-OwnedButton $name;if($overflow){$r=$overflow.Current.BoundingRectangle;Click-N0 ([int]($r.Left+$r.Width/2)) ([int]($r.Top+$r.Height/2));$overflowOpened=$true;break}};$icon=Find-OwnedButton 'NYLRA'}
 if(!$icon){$results.Add([pscustomobject]@{Scenario='actual-tray-left-click';Passed=$false;Reason='Owned notification-area Button not exposed'});Shortcut}
 else {
  $r=$icon.Current.BoundingRectangle;$x=[int]($r.Left+$r.Width/2);$y=[int]($r.Top+$r.Height/2)
  Click-N0 $x $y;Start-Sleep -Milliseconds 700
  $passed=[NylraShell]::IsWindowVisible($h) -and [N0Win]::GetForegroundWindow().ToInt64() -eq $w.Handle
  $results.Add([pscustomobject]@{Scenario='actual-tray-left-click';IconName=$icon.Current.Name;Click=@($x,$y);Passed=$passed})
  if(!$passed){throw 'Actual tray click failed to restore/focus main'}
 }
 if($overflowOpened){[N0Win]::keybd_event(27,0,0,[UIntPtr]::Zero);[N0Win]::keybd_event(27,0,2,[UIntPtr]::Zero)}
 foreach($i in 1..10){[NylraShell]::PostMessageW($h,0x10,[IntPtr]::Zero,[IntPtr]::Zero)|Out-Null;Start-Sleep -Milliseconds 250;if([NylraShell]::IsWindowVisible($h)){throw 'Hide cycle failed'};Shortcut;if(![NylraShell]::IsWindowVisible($h)){throw 'Shortcut restore failed'}}
 $results.Add([pscustomobject]@{Scenario='close-hide-shortcut-restore';Cycles=10;Passed=$true})
 # Companion/account controls add Tab stops. Focus the actual named UI button;
 # Space must still reach its frontend handler and native hide_main invocation.
 $ownedRoot=[Windows.Automation.AutomationElement]::FromHandle($h)
 $hideName=New-Object Windows.Automation.PropertyCondition ([Windows.Automation.AutomationElement]::NameProperty),'Hide to tray'
 $hideType=New-Object Windows.Automation.PropertyCondition ([Windows.Automation.AutomationElement]::ControlTypeProperty),([Windows.Automation.ControlType]::Button)
 $hideCondition=New-Object Windows.Automation.AndCondition $hideName,$hideType
 $hideButton=$ownedRoot.FindFirst([Windows.Automation.TreeScope]::Descendants,$hideCondition)
 if(!$hideButton){throw 'Owned Hide to tray button not exposed by UI Automation'}
 $hideButton.SetFocus();Start-Sleep -Milliseconds 200
 # UIA focus alone can leave WebView2's keyboard navigation anchor stale. Move
 # through the native Tab route and return to the named action before Space.
 [N0Win]::keybd_event(16,0,0,[UIntPtr]::Zero);[N0Win]::keybd_event(9,0,0,[UIntPtr]::Zero);[N0Win]::keybd_event(9,0,2,[UIntPtr]::Zero);[N0Win]::keybd_event(16,0,2,[UIntPtr]::Zero);Start-Sleep -Milliseconds 100
 foreach($tabAttempt in 1..12){[N0Win]::keybd_event(9,0,0,[UIntPtr]::Zero);[N0Win]::keybd_event(9,0,2,[UIntPtr]::Zero);Start-Sleep -Milliseconds 100;if([Windows.Automation.AutomationElement]::FocusedElement.Current.Name -eq 'Hide to tray'){break}}
 if([Windows.Automation.AutomationElement]::FocusedElement.Current.Name -ne 'Hide to tray'){throw 'Native Tab navigation did not reach Hide to tray'}
 $keyboardFocus=[Windows.Automation.AutomationElement]::FocusedElement.Current.Name
 $keyboardForeground=[N0Win]::GetForegroundWindow().ToInt64()
 [N0Win]::keybd_event(32,0,0,[UIntPtr]::Zero);[N0Win]::keybd_event(32,0,2,[UIntPtr]::Zero);Start-Sleep -Milliseconds 700
 $results.Add([pscustomobject]@{Scenario='keyboard-hide-button';Passed=![NylraShell]::IsWindowVisible($h);FocusedElement=$keyboardFocus;Foreground=$keyboardForeground;Main=$w.Handle})
 if([NylraShell]::IsWindowVisible($h)){throw 'Named Hide to tray button did not hide the native window after Space'}
 Shortcut
 # Main-process-only sample; excludes companion rendering in WebView2 processes.
 $cpuBefore=$p.CPU;$clock=[DateTime]::UtcNow;Start-Sleep -Seconds 5;$p.Refresh()
 $results.Add([pscustomobject]@{Scenario='main-process-only-idle';Seconds=([DateTime]::UtcNow-$clock).TotalSeconds;CPUOneCorePercent=100*($p.CPU-$cpuBefore)/([DateTime]::UtcNow-$clock).TotalSeconds;WorkingSetMiB=$p.WorkingSet64/1MB;Note='Excludes WebView2 descendants; not aggregate renderer acceptance'})
} finally {
 $results|ConvertTo-Json -Depth 6|Set-Content (Join-Path $out 'shell-observations.json')
 # End only this script's owned test app. Production Quit remains available in tray.
 if(!$p.HasExited){Stop-Process -Id $p.Id; $p.WaitForExit(5000)|Out-Null}
}
