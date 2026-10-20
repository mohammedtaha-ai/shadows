param([Parameter(Mandatory)][string]$Config)
$ErrorActionPreference='Stop'
$settings=Get-Content -LiteralPath $Config -Raw | ConvertFrom-Json
& $settings.node "$PSScriptRoot/desktop-origin-probe.mjs" $Config
if($LASTEXITCODE -ne 0){throw 'Packaged-origin assertions failed'}
