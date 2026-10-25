import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { openDatabase } from '../store/store.ts';
import { SessionsStore, KEPT_STEPS } from './store.ts';
import type { Session } from '../sessions/session.ts';

const session = (id: string, updatedAt: string): Session => ({ id, agent: 'claude_code', project: 'C:\\p', state: 'running',
  startedAt: updatedAt, updatedAt, lastStep: null, lastMessage: null });
test('sessions are saved, replaced and listed newest first', () => {
  const store = new SessionsStore(openDatabase(':memory:'));
  store.saveSession(session('a', '2026-10-13T19:00:00.000Z'));
  store.saveSession(session('b', '2026-10-13T19:01:00.000Z'));
  store.saveSession({ ...session('a', '2026-10-13T19:02:00.000Z'), state: 'ended' });
  assert.deepEqual(store.listSessions().map(s => [s.id, s.state]), [['a', 'ended'], ['b', 'running']]);
  assert.equal(store.getSession('missing'), undefined);
});

test('only the latest steps of a session are kept, in order', () => {
  const store = new SessionsStore(openDatabase(':memory:'));
  for (let i = 0; i < KEPT_STEPS + 5; i++) store.addStep('s1', { at: String(i), tool: 'Bash', target: `c${i}`, summary: `c${i}` });
  store.addStep('s2', { at: 'x', tool: 'Read', target: 'r', summary: 'r' });
  const steps = store.steps('s1');
  assert.equal(steps.length, KEPT_STEPS);
  assert.equal(steps[0].target, 'c5');
  assert.equal(steps.at(-1)?.target, `c${KEPT_STEPS + 4}`);
  assert.equal(store.steps('s2').length, 1);
});

test('the database survives reopening', () => {
  const file = join(mkdtempSync(join(tmpdir(), 'aylul-db-')), 'aylul.db');
  const firstDb = openDatabase(file);
  const first = new SessionsStore(firstDb);
  first.saveSession(session('a', '2026-10-13T19:00:00.000Z'));
  firstDb.close();
  const secondDb = openDatabase(file);
  const second = new SessionsStore(secondDb);
  assert.equal(second.getSession('a')?.id, 'a');
  secondDb.close();
});
