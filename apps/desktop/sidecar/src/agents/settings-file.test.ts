import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, writeFileSync, existsSync, readdirSync, mkdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { planAllowRule, writePlan, diffLines, fingerprint, expectBefore } from './settings-file.ts';

const now = new Date('2026-10-13T19:00:00.000Z');
function project() {
  const dir = mkdtempSync(join(tmpdir(), 'aylul-proj-'));
  return { dir, settings: join(dir, '.claude', 'settings.local.json'), backups: join(dir, 'backups') };
}

test('a rule is added to a missing file, with no backup', () => {
  const p = project();
  const backup = writePlan(planAllowRule(p.settings, 'Bash(npm test)'), p.backups, now);
  assert.equal(backup, '');
  assert.deepEqual(JSON.parse(readFileSync(p.settings, 'utf8')), { permissions: { allow: ['Bash(npm test)'] } });
});

test('a broad rule is refused before any settings file is written', () => {
  const p = project();
  assert.throws(() => planAllowRule(p.settings, 'Bash(git *)'), { message: 'rule_not_narrow' });
  assert.equal(existsSync(p.settings), false);
});

test('existing settings are kept, a backup is written, and the diff shows the rule', () => {
  const p = project();
  mkdirSync(join(p.dir, '.claude'));
  writeFileSync(p.settings, JSON.stringify({ hooks: { Stop: [] }, permissions: { allow: ['Read(a.md)'], deny: ['Bash(rm:*)'] } }));
  const plan = planAllowRule(p.settings, 'Bash(npm test)');
  assert.ok(diffLines(plan).some(l => l.startsWith('+ ') && l.includes('Bash(npm test)')));
  const backup = writePlan(plan, p.backups, now);
  assert.ok(existsSync(backup));
  const after = JSON.parse(readFileSync(p.settings, 'utf8'));
  assert.deepEqual(after.permissions, { allow: ['Read(a.md)', 'Bash(npm test)'], deny: ['Bash(rm:*)'] });
  assert.deepEqual(after.hooks, { Stop: [] });
});

test('adding a rule that is already there changes nothing in the diff', () => {
  const p = project();
  mkdirSync(join(p.dir, '.claude'));
  writeFileSync(p.settings, JSON.stringify({ permissions: { allow: ['Bash(npm test)'] } }, null, 2) + '\n');
  assert.deepEqual(diffLines(planAllowRule(p.settings, 'Bash(npm test)')), []);
});

test('invalid JSON or a wrong shape is refused and the file is untouched', () => {
  const p = project();
  mkdirSync(join(p.dir, '.claude'));
  for (const text of ['{ "permissions": ', '[]', '{"permissions": []}', '{"permissions": {"allow": "x"}}']) {
    writeFileSync(p.settings, text);
    assert.throws(() => planAllowRule(p.settings, 'Bash(npm test)'), { message: 'settings_invalid' });
    assert.equal(readFileSync(p.settings, 'utf8'), text);
  }
});

test('a file changed after the preview is not overwritten', () => {
  const p = project();
  mkdirSync(join(p.dir, '.claude'));
  writeFileSync(p.settings, '{}');
  const plan = planAllowRule(p.settings, 'Bash(npm test)');
  writeFileSync(p.settings, '{"model": "x"}');
  assert.throws(() => writePlan(plan, p.backups, now), { message: 'settings_changed' });
  assert.equal(readFileSync(p.settings, 'utf8'), '{"model": "x"}');
});

test('no temporary file is left after a write', () => {
  const p = project();
  writePlan(planAllowRule(p.settings, 'Bash(npm test)'), p.backups, now);
  assert.deepEqual(readdirSync(join(p.dir, '.claude')), ['settings.local.json']);
});

test('a file holding only whitespace is treated as empty, in the plan and the diff', () => {
  const p = project();
  mkdirSync(join(p.dir, '.claude'));
  writeFileSync(p.settings, '  \n');
  const plan = planAllowRule(p.settings, 'Bash(npm test)');
  assert.ok(diffLines(plan).every(l => l.startsWith('+ ')));
});

test('approving against a preview of a file that changed since is refused', () => {
  const p = project();
  mkdirSync(join(p.dir, '.claude'));
  writeFileSync(p.settings, '{}');
  const seen = fingerprint(planAllowRule(p.settings, 'Bash(npm test)').before);
  assert.doesNotThrow(() => expectBefore(planAllowRule(p.settings, 'Bash(npm test)'), seen));
  writeFileSync(p.settings, '{"model": "x"}');
  assert.throws(() => expectBefore(planAllowRule(p.settings, 'Bash(npm test)'), seen), { message: 'settings_changed' });
});
