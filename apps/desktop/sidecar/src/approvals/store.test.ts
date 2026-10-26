import { test } from 'node:test';
import assert from 'node:assert/strict';
import { openDatabase } from '../store/store.ts';
import { ApprovalsStore } from './store.ts';
import type { Pending } from './approvals.ts';

const item: Pending = { id: 'i1', kind: 'request', sessionId: 's1', project: 'C:\\p', tool: 'Bash', target: 'npm test',
  reason: null, proposedRule: null, at: '2026-10-13T19:00:00.000Z', held: false };

test('a terminal answer is marked allowed by the matching step', () => {
  const store = new ApprovalsStore(openDatabase(':memory:'));
  store.recordDecision(item, 'answered_in_terminal', item.at);
  assert.equal(store.markTerminalAllowed('s1', 'Bash', 'other'), false);
  assert.equal(store.markTerminalAllowed('s1', 'Bash', 'npm test'), true);
  assert.equal(store.decisions()[0].outcome, 'answered_in_terminal_allowed');
  assert.equal(store.markTerminalAllowed('s1', 'Bash', 'npm test'), false);
});
