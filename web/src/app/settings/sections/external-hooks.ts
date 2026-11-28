export type HookClient = 'claude' | 'codex'
export type HookPlatform = 'windows' | 'unix'

export interface HookSetup {
  scriptPath: string
  script: string
  entryPath: string
  entry: string
}

const WAIT_SECONDS = 540
const REQUEST_TIMEOUT_SECONDS = 570
const HOOK_TIMEOUT_SECONDS = 600

// The text of one client's Stop hook: it waits for the connection's reports once per run and
// carries no token, so it reads SHADOWS_TOKEN from the environment that starts the client.
export function hookSetup({ client, platform, waitUrl, projectId, directory }: {
  client: HookClient
  platform: HookPlatform
  waitUrl: string
  projectId: string
  directory: string
}): HookSetup {
  const windows = platform === 'windows'
  const sep = windows ? '\\' : '/'
  const base = directory.replace(/[\\/]+$/, '')
  const folder = client === 'claude' ? '.claude' : '.codex'
  const join = (...parts: string[]) => [base, ...parts].join(sep)
  const scriptPath = join(folder, 'hooks', `shadows-wait.${windows ? 'ps1' : 'sh'}`)
  const entryPath = client === 'claude' ? join('.claude', 'settings.local.json') : join('.codex', 'hooks.json')
  const command = windows
    ? `powershell -NoProfile -ExecutionPolicy Bypass -File "${scriptPath}"`
    : `sh "${scriptPath}"`
  const hook = client === 'claude'
    ? { type: 'command', command, asyncRewake: true, timeout: HOOK_TIMEOUT_SECONDS }
    : { type: 'command', command, timeout: HOOK_TIMEOUT_SECONDS }
  const entry = JSON.stringify({ hooks: { Stop: [{ hooks: [hook] }] } }, null, 2)
  const script = windows ? powershell(client, waitUrl, projectId) : shell(client, waitUrl, projectId)
  return { scriptPath, script, entryPath, entry }
}

function powershell(client: HookClient, waitUrl: string, projectId: string): string {
  const quote = (value: string) => `'${value.replaceAll("'", "''")}'`
  const hand = client === 'claude'
    ? '[Console]::Error.WriteLine($text)'
    : "Write-Output (@{ decision = 'block'; reason = $text } | ConvertTo-Json -Compress)"
  return [
    "if (-not $env:SHADOWS_TOKEN) { Write-Output 'Shadows: SHADOWS_TOKEN is not set'; exit 0 }",
    "$ackFile = Join-Path $PSScriptRoot 'shadows-wait.ack'",
    '$ids = @()',
    'if (Test-Path $ackFile) { $ids = @(Get-Content $ackFile | Where-Object { $_.Trim() }) }',
    `$body = @{ project_id = ${quote(projectId)}; seconds = ${WAIT_SECONDS}; ack = [string[]]$ids } | ConvertTo-Json -Compress`,
    'try {',
    `  $response = Invoke-WebRequest -UseBasicParsing -Method Post -Uri ${quote(waitUrl)} -Headers @{ Authorization = "Bearer $env:SHADOWS_TOKEN" } -ContentType 'application/json' -Body $body -TimeoutSec ${REQUEST_TIMEOUT_SECONDS}`,
    '} catch {',
    '  if ($_.Exception.Response) { $status = [int]$_.Exception.Response.StatusCode } else { $status = $_.Exception.Message }',
    '  Write-Output "Shadows wait: $status"',
    '  exit 0',
    '}',
    'if ($response.StatusCode -ne 200) { Write-Output "Shadows wait: $($response.StatusCode)"; exit 0 }',
    'Set-Content -Path $ackFile -Value @()',
    '$answer = $response.Content | ConvertFrom-Json',
    "if ($answer.outcome -ne 'ended') { exit 0 }",
    '$text = ($answer.reports | ForEach-Object { "Shadows report $($_.report_id) (task $($_.task), $($_.kind))`n$($_.report)" }) -join "`n`n"',
    hand,
    'Set-Content -Path $ackFile -Value @($answer.reports | ForEach-Object { $_.report_id })',
    `exit ${client === 'claude' ? 2 : 0}`,
    '',
  ].join('\n')
}

function shell(client: HookClient, waitUrl: string, projectId: string): string {
  const quote = (value: string) => `'${value.replaceAll("'", "'\\''")}'`
  const hand = client === 'claude'
    ? "printf '%s\\n' \"$text\" >&2"
    : "jq -n --arg reason \"$text\" '{decision:\"block\",reason:$reason}'"
  return [
    '#!/bin/sh',
    '# Needs curl and jq.',
    'if [ -z "$SHADOWS_TOKEN" ]; then echo \'Shadows: SHADOWS_TOKEN is not set\'; exit 0; fi',
    'ack="$(dirname "$0")/shadows-wait.ack"',
    "ids='[]'",
    'if [ -f "$ack" ]; then ids=$(jq -R -s -c \'split("\\n") | map(select(length > 0))\' "$ack"); fi',
    `body=$(jq -n -c --arg project ${quote(projectId)} --argjson ack "$ids" '{project_id:$project,seconds:${WAIT_SECONDS},ack:$ack}')`,
    'out=$(mktemp)',
    `status=$(curl -sS -m ${REQUEST_TIMEOUT_SECONDS} -o "$out" -w '%{http_code}' -X POST -H "Authorization: Bearer $SHADOWS_TOKEN" -H 'Content-Type: application/json' --data "$body" ${quote(waitUrl)}) || status=failed`,
    'if [ "$status" != 200 ]; then echo "Shadows wait: $status"; rm -f "$out"; exit 0; fi',
    ': > "$ack"',
    'if [ "$(jq -r .outcome "$out")" != ended ]; then rm -f "$out"; exit 0; fi',
    'text=$(jq -r \'.reports | map("Shadows report \\(.report_id) (task \\(.task), \\(.kind))\\n\\(.report)") | join("\\n\\n")\' "$out")',
    'reported=$(jq -r \'.reports[].report_id\' "$out")',
    'rm -f "$out"',
    hand,
    'printf \'%s\\n\' "$reported" > "$ack"',
    `exit ${client === 'claude' ? 2 : 0}`,
    '',
  ].join('\n')
}
