import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { Store, KEPT_STEPS } from './store.ts';
import type { Session } from '../sessions/session.ts';
import type { Pending } from '../approvals/approvals.ts';

const session = (id: string, updatedAt: string): Session => ({ id, agent: 'claude_code', project: 'C:\\p', state: 'running',
  startedAt: updatedAt, updatedAt, lastStep: null, lastMessage: null });
const item: Pending = { id: 'i1', kind: 'request', sessionId: 's1', project: 'C:\\p', tool: 'Bash', target: 'npm test',
  reason: null, proposedRule: null, at: '2026-10-13T19:00:00.000Z', held: false };

test('sessions are saved, replaced and listed newest first', () => {
  const store = new Store(':memory:');
  store.saveSession(session('a', '2026-10-13T19:00:00.000Z'));
  store.saveSession(session('b', '2026-10-13T19:01:00.000Z'));
  store.saveSession({ ...session('a', '2026-10-13T19:02:00.000Z'), state: 'ended' });
  assert.deepEqual(store.listSessions().map(s => [s.id, s.state]), [['a', 'ended'], ['b', 'running']]);
  assert.equal(store.getSession('missing'), undefined);
});

test('only the latest steps of a session are kept, in order', () => {
  const store = new Store(':memory:');
  for (let i = 0; i < KEPT_STEPS + 5; i++) store.addStep('s1', { at: String(i), tool: 'Bash', target: `c${i}`, summary: `c${i}` });
  store.addStep('s2', { at: 'x', tool: 'Read', target: 'r', summary: 'r' });
  const steps = store.steps('s1');
  assert.equal(steps.length, KEPT_STEPS);
  assert.equal(steps[0].target, 'c5');
  assert.equal(steps.at(-1)?.target, `c${KEPT_STEPS + 4}`);
  assert.equal(store.steps('s2').length, 1);
});

test('a terminal answer is marked allowed by the matching step', () => {
  const store = new Store(':memory:');
  store.recordDecision(item, 'answered_in_terminal', item.at);
  assert.equal(store.markTerminalAllowed('s1', 'Bash', 'other'), false);
  assert.equal(store.markTerminalAllowed('s1', 'Bash', 'npm test'), true);
  assert.equal(store.decisions()[0].outcome, 'answered_in_terminal_allowed');
  assert.equal(store.markTerminalAllowed('s1', 'Bash', 'npm test'), false);
});

test('rules and their uses are recorded per project', () => {
  const store = new Store(':memory:');
  const id = store.addRule('C:\\p', 'Bash(npm test)', 'C:\\b\\x', '2026-10-13T19:00:00.000Z');
  store.addRule('C:\\q', 'Bash(ls)', '', '2026-10-13T19:00:00.000Z');
  assert.deepEqual(store.rulesFor('C:\\p').map(r => r.rule), ['Bash(npm test)']);
  store.addGrantUse(id, 's1', 'toolu_1', '2026-10-13T19:01:00.000Z');
  assert.deepEqual(store.grantUses(), [{ ruleId: id, sessionId: 's1', toolUseId: 'toolu_1' }]);
});

test('the database survives reopening', () => {
  const file = join(mkdtempSync(join(tmpdir(), 'aylul-db-')), 'aylul.db');
  const first = new Store(file);
  first.saveSession(session('a', '2026-10-13T19:00:00.000Z'));
  first.close();
  const second = new Store(file);
  assert.equal(second.getSession('a')?.id, 'a');
  second.close();
});
