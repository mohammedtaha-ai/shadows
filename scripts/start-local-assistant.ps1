param(
    [Parameter(Mandatory = $true)][string]$HermesKeyFile,
    [string]$HermesOrigin = 'http://127.0.0.1:8642',
    [string]$RuntimeDirectory = (Join-Path $env:LOCALAPPDATA 'Shadows\assistant-runtime'),
    [switch]$SkipBuild,
    [switch]$OpenDesktop
)
$ErrorActionPreference = 'Stop'
$repository = Split-Path $PSScriptRoot -Parent
$runtime = [IO.Path]::GetFullPath($RuntimeDirectory)
$keyPath = (Resolve-Path -LiteralPath $HermesKeyFile).Path
if ((Test-Path -LiteralPath $runtime) -and -not (Test-Path -LiteralPath (Join-Path $runtime '.shadows-assistant-runtime')) -and @(Get-ChildItem -LiteralPath $runtime -Force).Count) {
    throw 'Choose an empty runtime directory; existing unrelated permissions will not be changed.'
}
New-Item -ItemType Directory -Force -Path $runtime | Out-Null
if ([Environment]::OSVersion.Platform -eq [PlatformID]::Win32NT) {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent().User
    $permissions = [Security.AccessControl.DirectorySecurity]::new()
    $permissions.SetOwner($identity)
    $permissions.SetAccessRuleProtection($true, $false)
    $permissions.AddAccessRule([Security.AccessControl.FileSystemAccessRule]::new($identity, 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow'))
    Set-Acl -LiteralPath $runtime -AclObject $permissions
}
Set-Content -LiteralPath (Join-Path $runtime '.shadows-assistant-runtime') -Value 'Shadows local assistant runtime' -Encoding utf8
if (-not $SkipBuild) {
    Push-Location (Join-Path $repository 'packages/hermes-client')
    try { npm ci --no-audit --no-fund; if ($LASTEXITCODE) { throw 'Client install failed' }; npm run build; if ($LASTEXITCODE) { throw 'Client build failed' } } finally { Pop-Location }
    Push-Location (Join-Path $repository 'services/assistant')
    try { npm ci --no-audit --no-fund; if ($LASTEXITCODE) { throw 'Service install failed' }; npm run build; if ($LASTEXITCODE) { throw 'Service build failed' } } finally { Pop-Location }
    Push-Location $repository
    try { cargo build -p shadows --bin shadows; if ($LASTEXITCODE) { throw 'Daemon build failed' } } finally { Pop-Location }
    if ($OpenDesktop) {
        Push-Location (Join-Path $repository 'apps/desktop')
        try { npm ci --no-audit --no-fund; if ($LASTEXITCODE) { throw 'Desktop install failed' }; npm run build; if ($LASTEXITCODE) { throw 'Desktop UI build failed' } } finally { Pop-Location }
        cargo build --manifest-path (Join-Path $repository 'apps/desktop/src-tauri/Cargo.toml'); if ($LASTEXITCODE) { throw 'Desktop build failed' }
    }
}
$descriptor = Join-Path $runtime 'connection.json'
$configuration = Join-Path $runtime 'assistant.json'
@{database=(Join-Path $runtime 'assistant.sqlite');hermes=@{origin=$HermesOrigin;network='development-host';keyFile=$keyPath}} |
    ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $configuration -Encoding utf8
$daemon = Join-Path $repository 'target/debug/shadows.exe'
$entry = Join-Path $repository 'services/assistant/dist/main.js'
$nodePath = (Get-Command node.exe -ErrorAction Stop).Source
# PowerShell passes a single argument line on Windows. Quote paths, never secrets.
$daemonArguments = @('daemon','--node',('"'+$nodePath+'"'),'--entry',('"'+$entry+'"'),'--config',('"'+$configuration+'"'),'--descriptor',('"'+$descriptor+'"'),'--bind','127.0.0.1:0')
$requestedAt = [DateTime]::UtcNow
$daemonProcess = Start-Process -FilePath $daemon -ArgumentList $daemonArguments -WindowStyle Hidden -RedirectStandardOutput (Join-Path $runtime 'daemon.log') -RedirectStandardError (Join-Path $runtime 'daemon-error.log') -PassThru
$ready = $false
for ($attempt = 0; $attempt -lt 150; $attempt++) {
    $daemonProcess.Refresh()
    if ($daemonProcess.HasExited) { throw 'Local assistant daemon exited before readiness. Check the runtime logs.' }
    if ((Test-Path -LiteralPath $descriptor) -and (Get-Item -LiteralPath $descriptor).LastWriteTimeUtc -ge $requestedAt) { $ready = $true; break }
    Start-Sleep -Milliseconds 100
}
if (-not $ready) {
    $daemonProcess.Kill()
    $daemonProcess.WaitForExit(3000) | Out-Null
    throw 'Local assistant daemon readiness timed out. Check the runtime logs.'
}
$env:SHADOWS_DAEMON_DESCRIPTOR = $descriptor
$env:AYLUL_NODE = $nodePath
$env:AYLUL_SIDECAR = Join-Path $repository 'apps/desktop/sidecar/src/main.ts'
Write-Output 'Local assistant daemon ready. Existing Hermes must already be running for chat.'
Write-Output "Desktop launch requires SHADOWS_DAEMON_DESCRIPTOR=$descriptor"
if ($OpenDesktop) { Start-Process -FilePath (Join-Path $repository 'apps/desktop/src-tauri/target/debug/nylra-desktop.exe') -WindowStyle Hidden }
