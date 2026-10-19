import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { planHookInstall, hooksInstalled, HOOK_EVENTS } from './install.ts';
import { writePlan } from './settings-file.ts';

const exe = 'C:\\Program Files\\AYLUL\\aylul-hook.exe';
const fresh = () => join(mkdtempSync(join(tmpdir(), 'aylul-claude-')), 'settings.json');
const now = new Date('2026-10-13T19:00:00.000Z');

test('every event gets one exec-form entry with its options', () => {
  const path = fresh();
  writePlan(planHookInstall(path, exe), join(path, '..', 'backups'), now);
  const hooks = JSON.parse(readFileSync(path, 'utf8')).hooks;
  for (const [event, extra] of HOOK_EVENTS) {
    assert.equal(hooks[event].length, 1, event);
    assert.deepEqual(hooks[event][0], { hooks: [{ type: 'command', command: exe, args: [], ...extra }] });
  }
  assert.equal(hooks.PostToolUse[0].hooks[0].async, true);
  assert.equal(hooks.PermissionRequest[0].hooks[0].timeout, 600);
  assert.equal(hooks.PermissionDenied[0].hooks[0].timeout, 130);
  assert.equal(hooksInstalled(path, exe), true);
});

test("the person's own hooks and settings are kept; AYLUL's are replaced, not repeated", () => {
  const path = fresh();
  const old = 'C:\\old\\aylul-hook.exe';
  writeFileSync(path, JSON.stringify({ model: 'opus', hooks: {
    Stop: [{ hooks: [{ type: 'command', command: 'mine.cmd' }, { type: 'command', command: old, args: [] }] }],
    PostToolUse: [{ matcher: 'Bash', hooks: [{ type: 'command', command: 'lint.cmd' }] }],
  } }));
  writePlan(planHookInstall(path, exe), join(path, '..', 'backups'), now);
  writePlan(planHookInstall(path, exe), join(path, '..', 'backups'), now);
  const settings = JSON.parse(readFileSync(path, 'utf8'));
  assert.equal(settings.model, 'opus');
  assert.deepEqual(settings.hooks.Stop, [
    { hooks: [{ type: 'command', command: 'mine.cmd' }] },
    { hooks: [{ type: 'command', command: exe, args: [] }] },
  ]);
  assert.equal(settings.hooks.PostToolUse.length, 2);
  assert.equal(settings.hooks.PostToolUse[0].matcher, 'Bash');
});

test('hooksInstalled is false for a missing, partial or invalid file', () => {
  const path = fresh();
  assert.equal(hooksInstalled(path, exe), false);
  writeFileSync(path, JSON.stringify({ hooks: { Stop: [{ hooks: [{ type: 'command', command: exe }] }] } }));
  assert.equal(hooksInstalled(path, exe), false);
  writeFileSync(path, '{');
  assert.equal(hooksInstalled(path, exe), false);
});

test('a hooks value of the wrong shape is refused', () => {
  const path = fresh();
  writeFileSync(path, JSON.stringify({ hooks: [] }));
  assert.throws(() => planHookInstall(path, exe), { message: 'settings_invalid' });
  writeFileSync(path, JSON.stringify({ hooks: { Stop: {} } }));
  assert.throws(() => planHookInstall(path, exe), { message: 'settings_invalid' });
});
