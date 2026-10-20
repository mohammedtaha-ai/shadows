param([switch]$VerifyOnly)
$ErrorActionPreference = 'Stop'
# Readiness for the actual installed profile. This script never copies a DB or sends a turn.
$repo = (Resolve-Path (Join-Path $PSScriptRoot '../../..')).Path
$desktopExe = Join-Path $repo 'apps/desktop/src-tauri/target/debug/nylra-desktop.exe'
$configPath = Join-Path $env:APPDATA 'app.nylra.desktop/desktop-core.json'
if (!(Test-Path -LiteralPath $desktopExe)) { throw 'Build the actual debug desktop first' }
if (!(Test-Path -LiteralPath $configPath)) { throw 'Save the existing Core connection in desktop Settings first' }
$config = Get-Content -Raw -LiteralPath $configPath | ConvertFrom-Json
function PlainPath([string]$value) { [IO.Path]::GetFullPath($value.Replace('\\?\', '')) }
$database = PlainPath $config.db
if (!(Test-Path -LiteralPath $database -PathType Leaf)) { throw 'Configured existing database is missing' }
$desktop = @(Get-Process nylra-desktop -ErrorAction SilentlyContinue |
    Where-Object { $_.Path -eq $desktopExe })
if ($desktop.Count -gt 1) { throw 'Multiple actual desktop processes require explicit inspection' }
if (!$desktop.Count) {
    if ($VerifyOnly) { throw 'The actual desktop is not running' }
    $desktop = @(Start-Process -FilePath $desktopExe -WorkingDirectory $repo -WindowStyle Hidden -PassThru)
}
$receiptPath = Join-Path (Split-Path $configPath) ('runtimes/' + $config.configurationId + '.json')
$ready = $false
foreach ($attempt in 1..50) {
    try {
        $identity = Invoke-RestMethod ($config.origin + '/api/runtime') -TimeoutSec 2
        if ($identity.configuration_id -eq $config.configurationId) { $ready = $true; break }
    } catch { }
    Start-Sleep -Milliseconds 200
}
if (!$ready) { throw 'Configured Core is not ready; use Retry in desktop Settings' }
$raw = Get-Content -Raw -LiteralPath $receiptPath
$receipt = $raw | ConvertFrom-Json
if ((PlainPath $receipt.database) -ne $database -or $receipt.runtime_id -ne $identity.runtime_id) {
    throw 'Runtime receipt does not match the actual database and HTTP identity'
}
$core = Get-Process -Id $receipt.process.pid
$startedAt = [regex]::Match($raw, '"started_at"\s*:\s*(\d+)').Groups[1].Value
if ($core.StartTime.ToUniversalTime().ToFileTimeUtc().ToString() -ne $startedAt -or
    $core.Path -ne (PlainPath $config.executable)) {
    throw 'Core process instance differs from its receipt'
}
$providers = @(Invoke-RestMethod ($config.origin + '/api/aylul/providers') -TimeoutSec 5)
$workspace = Invoke-RestMethod ($config.origin + '/api/aylul/agents') -TimeoutSec 5
$projects = @(Invoke-RestMethod ($config.origin + '/api/projects') -TimeoutSec 5)
[ordered]@{
    ready = $true
    database = $database
    desktopPid = $desktop[0].Id
    corePid = $core.Id
    runtimeId = $identity.runtime_id
    providers = @($providers | ForEach-Object { @{name = $_.name; keySet = $_.key_set} })
    agents = @($workspace.agents | Where-Object { !$_.deleted } | ForEach-Object { $_.name })
    projects = @($projects | ForEach-Object { $_.name })
    messageSent = $false
    manualAcceptance = 'Provider turn, background completion, focus, tray and binding-switch journey'
} | ConvertTo-Json -Depth 5
