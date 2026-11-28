import { expect, it } from 'vitest'
import { hookSetup } from './external-hooks'

const base = { waitUrl: 'http://127.0.0.1:4318/api/external/task-wait', projectId: 'p1' }
const win = 'C:\\work\\demo'
const unix = '/home/me/demo'

it('the_claude_code_hook_wakes_with_async_rewake', () => {
  const setup = hookSetup({ ...base, client: 'claude', platform: 'unix', directory: unix })
  expect(setup.entryPath).toBe('/home/me/demo/.claude/settings.local.json')
  const hook = JSON.parse(setup.entry).hooks.Stop[0].hooks[0]
  expect(hook).toMatchObject({ type: 'command', asyncRewake: true, timeout: 600 })
  expect(setup.script).toContain('>&2')
  expect(setup.script.trimEnd().endsWith('exit 2')).toBe(true)
  const ps = hookSetup({ ...base, client: 'claude', platform: 'windows', directory: win })
  expect(ps.script).toContain('[Console]::Error.WriteLine($text)')
  expect(ps.script.trimEnd().endsWith('exit 2')).toBe(true)
})

it('the_codex_hook_blocks_with_the_reports_as_reason', () => {
  const setup = hookSetup({ ...base, client: 'codex', platform: 'unix', directory: unix })
  expect(setup.entryPath).toBe('/home/me/demo/.codex/hooks.json')
  const hook = JSON.parse(setup.entry).hooks.Stop[0].hooks[0]
  expect(hook.asyncRewake).toBeUndefined()
  expect(hook.timeout).toBe(600)
  expect(setup.script).toContain('jq -n --arg reason "$text"')
  expect(setup.script).toContain('decision:"block"')
  expect(setup.script.trimEnd().endsWith('exit 0')).toBe(true)
  const ps = hookSetup({ ...base, client: 'codex', platform: 'windows', directory: win })
  expect(ps.script).toContain("decision = 'block'; reason = $text } | ConvertTo-Json")
})

it('each_script_acknowledges_only_after_writing_the_reports', () => {
  for (const client of ['claude', 'codex'] as const) {
    const ps = hookSetup({ ...base, client, platform: 'windows', directory: win }).script
    const handed = ps.indexOf(client === 'claude' ? 'Console]::Error' : 'Write-Output (@{ decision')
    expect(handed).toBeGreaterThan(0)
    expect(ps.lastIndexOf('Set-Content -Path $ackFile -Value @($answer')).toBeGreaterThan(handed)
    const sh = hookSetup({ ...base, client, platform: 'unix', directory: unix }).script
    const out = sh.indexOf(client === 'claude' ? '>&2' : 'jq -n --arg reason')
    expect(out).toBeGreaterThan(0)
    expect(sh.indexOf('"$reported" > "$ack"')).toBeGreaterThan(out)
  }
})

it('the_script_sends_540_seconds_and_the_project_and_reads_the_token_from_the_environment', () => {
  for (const platform of ['windows', 'unix'] as const) {
    const setup = hookSetup({ ...base, client: 'claude', platform, directory: win })
    expect(setup.script).toContain('540')
    expect(setup.script).toContain('570')
    expect(setup.script).toContain("'p1'")
    expect(setup.script).toContain(base.waitUrl)
    expect(setup.script).toContain('SHADOWS_TOKEN')
    expect(setup.script).toContain('Shadows: SHADOWS_TOKEN is not set')
    expect(setup.script + setup.entry).not.toMatch(/Bearer [A-Za-z0-9_-]{10,}/)
  }
})

it('a_report_with_no_task_is_headed_by_its_kind', () => {
  const ps = hookSetup({ ...base, client: 'claude', platform: 'windows', directory: win }).script
  expect(ps).toContain('if ($null -ne $_.task) { "task $($_.task), " }')
  expect(ps).toContain('($(if')
  expect(ps).not.toContain('(task $($_.task)')
  const sh = hookSetup({ ...base, client: 'claude', platform: 'unix', directory: unix }).script
  expect(sh).toContain('if .task == null then "" else "task \\(.task), " end')
  expect(sh).toContain('Shadows report \\(.report_id) (')
  expect(sh).not.toContain('(task \\(.task)')
})

it('windows_paths_and_the_powershell_run_command', () => {
  const setup = hookSetup({ ...base, client: 'claude', platform: 'windows', directory: win + '\\' })
  expect(setup.scriptPath).toBe('C:\\work\\demo\\.claude\\hooks\\shadows-wait.ps1')
  expect(setup.entryPath).toBe('C:\\work\\demo\\.claude\\settings.local.json')
  expect(JSON.parse(setup.entry).hooks.Stop[0].hooks[0].command).toBe(
    'powershell -NoProfile -ExecutionPolicy Bypass -File "C:\\work\\demo\\.claude\\hooks\\shadows-wait.ps1"')
  const codex = hookSetup({ ...base, client: 'codex', platform: 'unix', directory: unix })
  expect(JSON.parse(codex.entry).hooks.Stop[0].hooks[0].command).toBe(
    'sh "/home/me/demo/.codex/hooks/shadows-wait.sh"')
})
