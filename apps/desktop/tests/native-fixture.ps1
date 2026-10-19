param([string]$RunName='native-fixture')
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'windows-support.ps1')
$out=New-NylraRunDir $RunName
$exe=Join-Path (Get-NylraTarget) 'release/examples/native_fixture.exe'
# Every operation is a fresh native process. Fixture material is deliberately fake.
foreach($action in @('save','restart','logout','cleared','browser')) {
 & $exe $action $out
 if($LASTEXITCODE -ne 0){throw "Native fixture $action failed"}
}
