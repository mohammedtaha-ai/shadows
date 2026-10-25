import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, writeFileSync, readFileSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { Approvals } from './approvals.ts';
import { expectBefore, planAllowRule, settingsFor, writePlan } from '../agents/settings-file.ts';

function setup() {
  const home = mkdtempSync(join(tmpdir(), 'aylul-admission-'));
  const now = () => new Date('2026-10-13T19:00:00.000Z');
  const approvals = new Approvals({ now, newId: () => 'id1', record: () => {}, requestHoldMs: 500, refusalHoldMs: 500,
    writeRule: (project, rule, seen) => ({ ruleId: 'r',
      backupPath: writePlan(expectBefore(planAllowRule(settingsFor(project), rule), seen), join(home, 'b'), now()) }) });
  return { home, approvals };
}

test('a refusal is previewed and approved into the project settings', async () => {
  const { approvals, home } = setup();
  const project = join(home, 'proj');
  void approvals.refusal({ session_id: 's1', hook_event_name: 'PermissionDenied', cwd: join(project, 'src'), project_dir: project,
    tool_name: 'Bash', tool_input: { command: 'npm run deploy' }, reason: '[X]' }, new AbortController().signal);
  const id = approvals.pending()[0].id;
  const preview = await approvals.previewRule(id, 'Bash(npm run deploy)') as { lines: string[]; before: string };
  assert.ok(preview.lines.some(l => l.includes('Bash(npm run deploy)')));
  await approvals.approvePreview(id, 'Bash(npm run deploy)', preview.before);
  assert.deepEqual(JSON.parse(readFileSync(settingsFor(project), 'utf8')).permissions.allow, ['Bash(npm run deploy)']);
});

test('an approval after the settings file changed since the preview writes nothing', async () => {
  const { approvals, home } = setup();
  const project = join(home, 'proj');
  void approvals.refusal({ session_id: 's1', hook_event_name: 'PermissionDenied', project_dir: project, tool_name: 'Bash',
    tool_input: { command: 'npm run deploy' }, reason: '[X]' }, new AbortController().signal);
  const id = approvals.pending()[0].id;
  const preview = await approvals.previewRule(id, 'Bash(npm run deploy)') as { before: string };
  mkdirSync(join(project, '.claude'), { recursive: true });
  writeFileSync(settingsFor(project), '{"model": "x"}');
  assert.throws(() => approvals.approvePreview(id, 'Bash(npm run deploy)', preview.before),
    { message: 'settings_changed' });
  assert.equal(readFileSync(settingsFor(project), 'utf8'), '{"model": "x"}');
  assert.equal(approvals.pending().length, 1);
});

test('a preview for one rule cannot approve a different rule', async () => {
  const { approvals, home } = setup();
  const project = join(home, 'proj');
  void approvals.refusal({ session_id: 's1', hook_event_name: 'PermissionDenied', project_dir: project,
    tool_name: 'Bash', tool_input: { command: 'npm run deploy' } }, new AbortController().signal);
  const id = approvals.pending()[0].id;
  const preview = approvals.previewRule(id, 'Bash(npm run deploy)') as { before: string };
  assert.throws(() => approvals.approvePreview(id, 'Bash(npm test)', preview.before),
    { message: 'preview_mismatch' });
  assert.equal(existsSync(settingsFor(project)), false);
  assert.equal(approvals.pending().length, 1);
});

test('a newer rule preview invalidates the earlier one', () => {
  const { approvals, home } = setup();
  const project = join(home, 'proj');
  void approvals.refusal({ session_id: 's1', hook_event_name: 'PermissionDenied', project_dir: project,
    tool_name: 'Bash', tool_input: { command: 'npm run deploy' } }, new AbortController().signal);
  const id = approvals.pending()[0].id;
  const first = approvals.previewRule(id, 'Bash(npm run deploy)') as { before: string };
  approvals.previewRule(id, 'Bash(npm test)');
  assert.throws(() => approvals.approvePreview(id, 'Bash(npm run deploy)', first.before),
    { message: 'preview_mismatch' });
  assert.equal(existsSync(settingsFor(project)), false);
});

test('approval admission rejects a changed fingerprint and consumes a successful preview', async () => {
  const { approvals, home } = setup();
  const project = join(home, 'proj');
  const hook = approvals.refusal({ session_id: 's1', hook_event_name: 'PermissionDenied', project_dir: project,
    tool_name: 'Bash', tool_input: { command: 'npm test' } }, new AbortController().signal);
  const id = approvals.pending()[0].id;
  const preview = approvals.previewRule(id, 'Bash(npm test)');
  assert.throws(() => approvals.approvePreview(id, 'Bash(npm test)', 'different'), { message: 'preview_mismatch' });
  assert.equal(existsSync(settingsFor(project)), false);
  approvals.approvePreview(id, 'Bash(npm test)', preview.before);
  await hook;
  assert.throws(() => approvals.approvePreview(id, 'Bash(npm test)', preview.before), { message: 'preview_mismatch' });
});

test('dismissal removes the preview and approval still requires a current refusal', async () => {
  const { approvals, home } = setup();
  const project = join(home, 'proj');
  const hook = approvals.refusal({ session_id: 's1', hook_event_name: 'PermissionDenied', project_dir: project,
    tool_name: 'Bash', tool_input: { command: 'npm test' } }, new AbortController().signal);
  const id = approvals.pending()[0].id;
  const preview = approvals.previewRule(id, 'Bash(npm test)');
  approvals.dismiss(id);
  assert.deepEqual(await hook, {});
  assert.throws(() => approvals.approvePreview(id, 'Bash(npm test)', preview.before), { message: 'preview_mismatch' });
  assert.throws(() => approvals.previewRule(id, 'Bash(npm test)'), { message: 'not_pending' });
  assert.equal(existsSync(settingsFor(project)), false);
});
