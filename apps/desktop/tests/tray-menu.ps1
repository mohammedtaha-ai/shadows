param([string]$RunName='tray-menu')
# Focused unresolved case only: real right-click menu Open and Quit.
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'windows-support.ps1')
$out=New-NylraRunDir $RunName
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
Add-Type @'
using System;using System.Runtime.InteropServices;
public class NylraMenuWin {
 [DllImport("user32.dll")]public static extern bool IsWindowVisible(IntPtr h);
 [DllImport("user32.dll")]public static extern bool PostMessageW(IntPtr h,uint msg,IntPtr w,IntPtr l);
}
'@
function Button([string]$Name){
 if($Name -eq 'NYLRA'){return Find-NylraTrayIcon}
 $a=New-Object Windows.Automation.PropertyCondition ([Windows.Automation.AutomationElement]::NameProperty),$Name
 $b=New-Object Windows.Automation.PropertyCondition ([Windows.Automation.AutomationElement]::ControlTypeProperty),([Windows.Automation.ControlType]::Button)
 $c=New-Object Windows.Automation.AndCondition $a,$b
 if($Name -eq 'NYLRA'){$id=[Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::AutomationIdProperty,'NotifyItemIcon');$c=[Windows.Automation.AndCondition]::new($c,$id)}
 [Windows.Automation.AutomationElement]::RootElement.FindFirst([Windows.Automation.TreeScope]::Descendants,$c)
}
function OpenOwnedMenu {
 Key 0x1B
 $icon=Button 'NYLRA'
 if(!$icon){$overflow=Button 'Show Hidden Icons';if(!$overflow){throw 'Owned shell overflow unavailable'};$r=$overflow.Current.BoundingRectangle;Click-N0 ([int]($r.Left+$r.Width/2)) ([int]($r.Top+$r.Height/2));$icon=Button 'NYLRA'}
 if(!$icon){foreach($attempt in 1..20){Start-Sleep -Milliseconds 100;$icon=Button 'NYLRA';if($icon){break}}}
 if(!$icon){throw 'Owned NYLRA icon unavailable'}
 $r=$icon.Current.BoundingRectangle;[N0Win]::SetCursorPos([int]($r.Left+$r.Width/2),[int]($r.Top+$r.Height/2))|Out-Null
 [N0Win]::mouse_event(8,0,0,0,[UIntPtr]::Zero);[N0Win]::mouse_event(16,0,0,0,[UIntPtr]::Zero);Start-Sleep -Milliseconds 500
}
function Key([byte]$Code){[N0Win]::keybd_event($Code,0,0,[UIntPtr]::Zero);[N0Win]::keybd_event($Code,0,2,[UIntPtr]::Zero);Start-Sleep -Milliseconds 120}
function SelectOwnedMenuItem([string]$Name){
 $nameCondition=New-Object Windows.Automation.PropertyCondition ([Windows.Automation.AutomationElement]::NameProperty),$Name
 $typeCondition=New-Object Windows.Automation.PropertyCondition ([Windows.Automation.AutomationElement]::ControlTypeProperty),([Windows.Automation.ControlType]::MenuItem)
 $condition=New-Object Windows.Automation.AndCondition $nameCondition,$typeCondition
 $item=[Windows.Automation.AutomationElement]::RootElement.FindFirst([Windows.Automation.TreeScope]::Descendants,$condition)
 if(!$item){throw "Owned visible menu item unavailable: $Name"}
 $r=$item.Current.BoundingRectangle;if($r.IsEmpty -or $r.Width -le 0){throw 'Owned menu rectangle unavailable'}
 Click-N0 ([int]($r.Left+$r.Width/2)) ([int]($r.Top+$r.Height/2))
}
$p=Start-Process (Join-Path (Get-NylraTarget) 'debug/nylra-desktop.exe') -PassThru -WindowStyle Hidden
$results=[Collections.Generic.List[object]]::new()
try{
 Start-Sleep -Seconds 3;$w=Get-N0Rect 'NYLRA';$h=[IntPtr]$w.Handle
 $character=[N0Win]::FindWindow([IntPtr]::Zero,'AYLUL');if($character -ne [IntPtr]::Zero){[NylraMenuWin]::PostMessageW($character,0x10,[IntPtr]::Zero,[IntPtr]::Zero)|Out-Null;Start-Sleep -Milliseconds 300}
 [NylraMenuWin]::PostMessageW($h,0x10,[IntPtr]::Zero,[IntPtr]::Zero)|Out-Null;Start-Sleep -Milliseconds 500
 OpenOwnedMenu;SelectOwnedMenuItem 'Open NYLRA';Start-Sleep -Milliseconds 500
 $opened=[NylraMenuWin]::IsWindowVisible($h) -and [N0Win]::GetForegroundWindow().ToInt64() -eq $w.Handle
 $results.Add([pscustomobject]@{Scenario='actual-tray-menu-open';Passed=$opened;Main=$w.Handle;Foreground=[N0Win]::GetForegroundWindow().ToInt64()})
 if(!$opened){throw 'Menu Open did not restore/focus main'}
 OpenOwnedMenu;SelectOwnedMenuItem 'Quit NYLRA'
 $quit=$p.WaitForExit(5000);$results.Add([pscustomobject]@{Scenario='actual-tray-menu-quit';Passed=$quit})
 if(!$quit){throw 'Menu Quit did not exit app'}
}finally{
 $results|ConvertTo-Json -Depth 4|Set-Content (Join-Path $out 'tray-menu-observations.json')
 if(!$p.HasExited){Stop-Process -Id $p.Id;$p.WaitForExit(5000)|Out-Null}
}
