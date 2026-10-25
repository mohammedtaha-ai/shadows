import { test } from 'node:test';
import assert from 'node:assert/strict';
import { openDatabase } from '../store/store.ts';
import { SettingsStore } from './store.ts';

test('rules and their uses are recorded per project', () => {
  const store = new SettingsStore(openDatabase(':memory:'));
  const id = store.addRule('C:\\p', 'Bash(npm test)', 'C:\\b\\x', '2026-10-13T19:00:00.000Z');
  store.addRule('C:\\q', 'Bash(ls)', '', '2026-10-13T19:00:00.000Z');
  assert.deepEqual(store.rulesFor('C:\\p').map(r => r.rule), ['Bash(npm test)']);
  store.addGrantUse(id, 's1', 'toolu_1', '2026-10-13T19:01:00.000Z');
  assert.deepEqual(store.grantUses(), [{ ruleId: id, sessionId: 's1', toolUseId: 'toolu_1' }]);
});
