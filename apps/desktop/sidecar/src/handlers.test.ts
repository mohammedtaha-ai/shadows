import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, writeFileSync, readFileSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { makeHandlers, settingsFor } from './handlers.ts';
import { Store } from './store/store.ts';
import { Approvals } from './approvals/approvals.ts';
import { expectBefore, planAllowRule, writePlan } from './agents/settings-file.ts';

function setup(withExe = true) {
  const home = mkdtempSync(join(tmpdir(), 'aylul-h-'));
  const exe = join(home, 'aylul-hook.exe');
  if (withExe) writeFileSync(exe, '');
  const store = new Store(':memory:');
  const now = () => new Date('2026-10-13T19:00:00.000Z');
  const approvals = new Approvals({ now, newId: () => 'id1', record: () => {}, requestHoldMs: 500, refusalHoldMs: 500,
    writeRule: (project, rule, seen) => ({ ruleId: 'r',
      backupPath: writePlan(expectBefore(planAllowRule(settingsFor(project), rule), seen), join(home, 'b'), now()) }) });
  const handlers = makeHandlers({ store, approvals, claudeSettings: join(home, '.claude', 'settings.json'),
    hookExe: withExe ? exe : undefined, backupDir: join(home, 'b'), now });
  return { home, exe, approvals, handlers };
}

test('state reports hooks not installed, then installed after hooks.install', async () => {
  const { handlers, home } = setup();
  assert.equal((await handlers.state({}) as { hooksInstalled: boolean }).hooksInstalled, false);
  const preview = await handlers['hooks.preview']({}) as { path: string; lines: string[]; before: string };
  assert.equal(preview.path, join(home, '.claude', 'settings.json'));
  assert.ok(preview.lines.some(l => l.includes('aylul-hook.exe')));
  await handlers['hooks.install']({ before: preview.before });
  assert.equal((await handlers.state({}) as { hooksInstalled: boolean }).hooksInstalled, true);
});

test('without the hook executable, install is refused', () => {
  const { handlers } = setup(false);
  assert.throws(() => handlers['hooks.install']({ before: 'x' }), { message: 'hook_exe_missing' });
});

test('a refusal is previewed and approved into the project settings', async () => {
  const { handlers, approvals, home } = setup();
  const project = join(home, 'proj');
  void approvals.refusal({ session_id: 's1', hook_event_name: 'PermissionDenied', cwd: join(project, 'src'), project_dir: project,
    tool_name: 'Bash', tool_input: { command: 'npm run deploy' }, reason: '[X]' }, new AbortController().signal);
  const id = approvals.pending()[0].id;
  const preview = await handlers['refusal.preview']({ id, rule: 'Bash(npm run deploy)' }) as { lines: string[]; before: string };
  assert.ok(preview.lines.some(l => l.includes('Bash(npm run deploy)')));
  await handlers['refusal.approve']({ id, rule: 'Bash(npm run deploy)', before: preview.before });
  assert.deepEqual(JSON.parse(readFileSync(settingsFor(project), 'utf8')).permissions.allow, ['Bash(npm run deploy)']);
});

test('an approval after the settings file changed since the preview writes nothing', async () => {
  const { handlers, approvals, home } = setup();
  const project = join(home, 'proj');
  void approvals.refusal({ session_id: 's1', hook_event_name: 'PermissionDenied', project_dir: project, tool_name: 'Bash',
    tool_input: { command: 'npm run deploy' }, reason: '[X]' }, new AbortController().signal);
  const id = approvals.pending()[0].id;
  const preview = await handlers['refusal.preview']({ id, rule: 'Bash(npm run deploy)' }) as { before: string };
  mkdirSync(join(project, '.claude'), { recursive: true });
  writeFileSync(settingsFor(project), '{"model": "x"}');
  assert.throws(() => handlers['refusal.approve']({ id, rule: 'Bash(npm run deploy)', before: preview.before }),
    { message: 'settings_changed' });
  assert.equal(readFileSync(settingsFor(project), 'utf8'), '{"model": "x"}');
  assert.equal(approvals.pending().length, 1);
});

test('a preview for one rule cannot approve a different rule', async () => {
  const { handlers, approvals, home } = setup();
  const project = join(home, 'proj');
  void approvals.refusal({ session_id: 's1', hook_event_name: 'PermissionDenied', project_dir: project,
    tool_name: 'Bash', tool_input: { command: 'npm run deploy' } }, new AbortController().signal);
  const id = approvals.pending()[0].id;
  const preview = handlers['refusal.preview']({ id, rule: 'Bash(npm run deploy)' }) as { before: string };
  assert.throws(() => handlers['refusal.approve']({ id, rule: 'Bash(npm test)', before: preview.before }),
    { message: 'preview_mismatch' });
  assert.equal(existsSync(settingsFor(project)), false);
  assert.equal(approvals.pending().length, 1);
});

test('a newer rule preview invalidates the earlier one', () => {
  const { handlers, approvals, home } = setup();
  const project = join(home, 'proj');
  void approvals.refusal({ session_id: 's1', hook_event_name: 'PermissionDenied', project_dir: project,
    tool_name: 'Bash', tool_input: { command: 'npm run deploy' } }, new AbortController().signal);
  const id = approvals.pending()[0].id;
  const first = handlers['refusal.preview']({ id, rule: 'Bash(npm run deploy)' }) as { before: string };
  handlers['refusal.preview']({ id, rule: 'Bash(npm test)' });
  assert.throws(() => handlers['refusal.approve']({ id, rule: 'Bash(npm run deploy)', before: first.before }),
    { message: 'preview_mismatch' });
  assert.equal(existsSync(settingsFor(project)), false);
});

test('missing or wrong parameters are bad_params', () => {
  const { handlers } = setup();
  assert.throws(() => handlers['request.answer']({ id: 'x' }), { message: 'bad_params' });
  assert.throws(() => handlers['request.answer']({ id: 'x', behavior: 'maybe' }), { message: 'bad_params' });
  assert.throws(() => handlers.steps({}), { message: 'bad_params' });
});
