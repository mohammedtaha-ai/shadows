import { test } from 'node:test';
import assert from 'node:assert/strict';
import { RulePreviewGuard } from '../src/features/approvals/rule-preview.ts';

test('an edit invalidates a preview response still in flight', async () => {
  const guard = new RulePreviewGuard();
  let resolve!: (plan: { path: string; lines: string[]; before: string }) => void;
  const loading = guard.load('Bash(npm test)', () => new Promise(done => { resolve = done; }));
  guard.invalidate();
  resolve({ path: 'settings.local.json', lines: ['+ Bash(npm test)'], before: 'old' });
  assert.equal(await loading, null);
});

test('approval uses the rule captured for the displayed preview', async () => {
  const guard = new RulePreviewGuard();
  const plan = await guard.load('Bash(npm test)', async () => ({
    path: 'settings.local.json', lines: ['+ Bash(npm test)'], before: 'old',
  }));
  assert.ok(plan);
  assert.deepEqual(guard.approval(plan), { rule: 'Bash(npm test)', before: 'old' });
});

test('settings_changed invalidates the stale preview so it can be requested again', async () => {
  const guard = new RulePreviewGuard();
  assert.equal(guard.shouldClear('settings_changed'), true);
  const fresh = await guard.load('Bash(npm test)', async () => ({
    path: 'settings.local.json', lines: ['+ Bash(npm test)'], before: 'new',
  }));
  assert.equal(fresh?.before, 'new');
});

test('preview_mismatch also clears the stale preview', () => {
  const guard = new RulePreviewGuard();
  assert.equal(guard.shouldClear('preview_mismatch'), true);
});
