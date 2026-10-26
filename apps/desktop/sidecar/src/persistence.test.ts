import { test } from 'node:test';
import assert from 'node:assert/strict';
import { DatabaseSync } from 'node:sqlite';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createActivityApp } from './app.ts';
import { makeHandlers } from './handlers.ts';
import { makeRoute } from './hooks/route.ts';
import { settingsFor } from './agents/settings-file.ts';

const at = '2026-10-13T19:00:00.000Z';
test('RPC projections and persisted rows survive reopening the unchanged aylul.db schema', async () => {
  const home = mkdtempSync(join(tmpdir(), 'aylul-pin-'));
  const file = join(home, 'aylul.db');
  const backupDir = join(home, 'backups');
  const project = join(home, 'project');
  const settings = settingsFor(project);
  mkdirSync(join(project, '.claude'), { recursive: true });
  writeFileSync(settings, '{}');
  const exe = join(home, 'aylul-hook.exe');
  writeFileSync(exe, '');
  const now = () => new Date(at);
  const app = createActivityApp(file, { now, newId: () => 'i1', backupDir, hookExe: exe, claudeSettings: join(home, 'settings.json') });
  const handlers = makeHandlers(app);
  const route = makeRoute(app);
  const event = { session_id: 's1', hook_event_name: 'PostToolUse', project_dir: project,
    tool_name: 'Bash', tool_input: { command: 'npm test' }, tool_use_id: 't1' };
  assert.deepEqual(await route(event, new AbortController().signal), {});
  const step = { at, tool: 'Bash', target: 'npm test', summary: 'npm test' };
  const session = { id: 's1', agent: 'claude_code', project, state: 'running', startedAt: at, updatedAt: at,
    lastStep: step, lastMessage: null };
  assert.deepEqual(await handlers.state({}), { sessions: [session], pending: [], hooksInstalled: false });
  assert.deepEqual(await handlers.steps({ sessionId: 's1' }), [step]);
  const waiting = route({ ...event, hook_event_name: 'PermissionDenied' }, new AbortController().signal);
  const preview = await handlers['refusal.preview']({ id: 'i1', rule: 'Bash(npm test)' }) as { before: string };
  const approved = await handlers['refusal.approve']({ id: 'i1', rule: 'Bash(npm test)', before: preview.before }) as { backupPath: string };
  assert.deepEqual(Object.keys(approved), ['backupPath']);
  assert.equal(readFileSync(approved.backupPath, 'utf8'), '{}');
  assert.deepEqual(await waiting, { output: { hookSpecificOutput: { hookEventName: 'PermissionDenied', retry: true } } });
  await route(event, new AbortController().signal);
  const closed = new AbortController();
  const request = route({ ...event, hook_event_name: 'PermissionRequest' }, closed.signal);
  closed.abort();
  assert.deepEqual(await request, {});
  await route(event, new AbortController().signal);
  const refusal = route({ ...event, hook_event_name: 'PermissionDenied' }, new AbortController().signal);
  assert.equal(await handlers['refusal.dismiss']({ id: 'i1' }), null);
  assert.deepEqual(await refusal, {});
  app.close();
  const db = new DatabaseSync(file);
  const rows = (table: string) => db.prepare(`SELECT * FROM ${table} ORDER BY 1`).all().map(r => ({ ...r }));
  assert.deepEqual(rows('sessions'), [{ id: 's1', body: JSON.stringify({ ...session, updatedAt: at }), updated_at: at }]);
  assert.deepEqual(rows('steps'), [1, 2, 3].map(seq => ({ seq, session_id: 's1', body: JSON.stringify(step) })));
  assert.deepEqual(rows('decisions'), ['rule_written', 'answered_in_terminal_allowed', 'dismissed'].map((outcome, i) => ({
    seq: i + 1, item_id: 'i1', session_id: 's1', kind: i === 1 ? 'request' : 'refusal', tool: 'Bash', target: 'npm test', outcome, at,
  })));
  const rules = rows('rules');
  assert.equal(rules.length, 1);
  assert.deepEqual(rules, [{ id: rules[0].id, project, rule: 'Bash(npm test)', backup_path: approved.backupPath, approved_by: 'person', at }]);
  assert.match(String(rules[0].id), /^[0-9a-f-]{36}$/);
  assert.deepEqual(rows('grant_uses'), [1, 2].map(seq => ({ seq, rule_id: rules[0].id, session_id: 's1', tool_use_id: 't1', at })));
  assert.deepEqual(db.prepare("SELECT name, sql FROM sqlite_master WHERE type = 'table' ORDER BY name").all().map(r => ({ ...r })), SCHEMA);
  db.close();
  const reopened = createActivityApp(file, { now, backupDir, hookExe: exe, claudeSettings: join(home, 'settings.json') });
  assert.deepEqual(reopened.sessions.list(), [{ ...session }]);
  assert.deepEqual(reopened.sessions.steps('s1'), [step, step, step]);
  reopened.close();
});

const SCHEMA = [
  { name: 'decisions', sql: 'CREATE TABLE decisions (seq INTEGER PRIMARY KEY AUTOINCREMENT, item_id TEXT NOT NULL, session_id TEXT NOT NULL,\n  kind TEXT NOT NULL, tool TEXT NOT NULL, target TEXT NOT NULL, outcome TEXT NOT NULL, at TEXT NOT NULL)' },
  { name: 'grant_uses', sql: 'CREATE TABLE grant_uses (seq INTEGER PRIMARY KEY AUTOINCREMENT, rule_id TEXT NOT NULL, session_id TEXT NOT NULL,\n  tool_use_id TEXT NOT NULL, at TEXT NOT NULL)' },
  { name: 'rules', sql: 'CREATE TABLE rules (id TEXT PRIMARY KEY, project TEXT NOT NULL, rule TEXT NOT NULL, backup_path TEXT NOT NULL,\n  approved_by TEXT NOT NULL, at TEXT NOT NULL)' },
  { name: 'sessions', sql: 'CREATE TABLE sessions (id TEXT PRIMARY KEY, body TEXT NOT NULL, updated_at TEXT NOT NULL)' },
  { name: 'sqlite_sequence', sql: 'CREATE TABLE sqlite_sequence(name,seq)' },
  { name: 'steps', sql: 'CREATE TABLE steps (seq INTEGER PRIMARY KEY AUTOINCREMENT, session_id TEXT NOT NULL, body TEXT NOT NULL)' },
];
