import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, writeFileSync, readFileSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { PassThrough } from 'node:stream';
import { serveRpc } from './rpc.ts';
import { makeHandlers } from './handlers.ts';
import { Store } from './store/store.ts';
import { Approvals } from './approvals/approvals.ts';
import { expectBefore, planAllowRule, settingsFor, writePlan } from './agents/settings-file.ts';

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

async function rpc(handlers: ReturnType<typeof makeHandlers>, method: string, params: Record<string, unknown>) {
  const input = new PassThrough();
  const output = new PassThrough();
  let reply = '';
  output.setEncoding('utf8').on('data', chunk => { reply += chunk; });
  const ended = new Promise<void>(resolve => serveRpc(input, output, handlers, resolve));
  input.end(JSON.stringify({ id: 15, method, params }) + '\n');
  await ended;
  await new Promise(resolve => setImmediate(resolve));
  return JSON.parse(reply);
}

test('refusal.approve RPC returns the backup path and retries the held hook', async () => {
  const { handlers, approvals, home } = setup();
  const project = join(home, 'proj');
  const hook = approvals.refusal({ session_id: 's1', hook_event_name: 'PermissionDenied', project_dir: project,
    tool_name: 'Bash', tool_input: { command: 'npm test' } }, new AbortController().signal);
  const preview = await rpc(handlers, 'refusal.preview', { id: 'id1', rule: 'Bash(npm test)' });
  assert.deepEqual(preview, { id: 15, result: {
    path: settingsFor(project),
    lines: ['+ {', '+   "permissions": {', '+     "allow": [', '+       "Bash(npm test)"', '+     ]', '+   }', '+ }'],
    before: 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855',
  } });
  assert.deepEqual(await rpc(handlers, 'refusal.approve', { id: 'id1', rule: ' Bash(npm test) ', before: preview.result.before }),
    { id: 15, result: { backupPath: '' } });
  assert.deepEqual(await hook, { output: { hookSpecificOutput: { hookEventName: 'PermissionDenied', retry: true } } });
  assert.deepEqual(approvals.pending(), []);
  assert.deepEqual(JSON.parse(readFileSync(settingsFor(project), 'utf8')), { permissions: { allow: ['Bash(npm test)'] } });
});

test('request.answer deny RPC returns null and the exact denial message', async () => {
  const { handlers, approvals } = setup();
  const hook = approvals.request({ session_id: 's1', hook_event_name: 'PermissionRequest', tool_name: 'Bash',
    tool_input: { command: 'npm test' } }, new AbortController().signal);
  assert.deepEqual(await rpc(handlers, 'request.answer', { id: 'id1', behavior: 'deny' }), { id: 15, result: null });
  assert.deepEqual(await hook, { output: { hookSpecificOutput: { hookEventName: 'PermissionRequest',
    decision: { behavior: 'deny', message: 'The person denied this in AYLUL.' } } } });
  assert.deepEqual(approvals.pending(), []);
});

test('refusal.approve RPC rejects a superseded preview without writing', async () => {
  const { handlers, approvals, home } = setup();
  const project = join(home, 'proj');
  const closed = new AbortController();
  const hook = approvals.refusal({ session_id: 's1', hook_event_name: 'PermissionDenied', project_dir: project,
    tool_name: 'Bash', tool_input: { command: 'npm test' } }, closed.signal);
  const first = await rpc(handlers, 'refusal.preview', { id: 'id1', rule: 'Bash(npm test)' });
  await rpc(handlers, 'refusal.preview', { id: 'id1', rule: 'Bash(npm run deploy)' });
  assert.deepEqual(await rpc(handlers, 'refusal.approve', { id: 'id1', rule: 'Bash(npm test)', before: first.result.before }),
    { id: 15, error: 'preview_mismatch' });
  assert.equal(existsSync(settingsFor(project)), false);
  assert.equal(approvals.pending().length, 1);
  closed.abort();
  assert.deepEqual(await hook, {});
});

test('refusal.approve RPC preserves changed settings and permits retry after restoration', async () => {
  const { handlers, approvals, home } = setup();
  const project = join(home, 'proj');
  mkdirSync(join(project, '.claude'), { recursive: true });
  writeFileSync(settingsFor(project), '{}');
  const hook = approvals.refusal({ session_id: 's1', hook_event_name: 'PermissionDenied', project_dir: project,
    tool_name: 'Bash', tool_input: { command: 'npm test' } }, new AbortController().signal);
  const preview = await rpc(handlers, 'refusal.preview', { id: 'id1', rule: 'Bash(npm test)' });
  writeFileSync(settingsFor(project), '{"model":"changed"}');
  assert.deepEqual(await rpc(handlers, 'refusal.approve', { id: 'id1', rule: 'Bash(npm test)', before: preview.result.before }),
    { id: 15, error: 'settings_changed' });
  assert.equal(readFileSync(settingsFor(project), 'utf8'), '{"model":"changed"}');
  assert.equal(existsSync(join(home, 'b')), false);
  assert.equal(approvals.pending().length, 1);
  writeFileSync(settingsFor(project), '{}');
  const approved = await rpc(handlers, 'refusal.approve', { id: 'id1', rule: 'Bash(npm test)', before: preview.result.before });
  assert.equal(approved.id, 15);
  assert.deepEqual(Object.keys(approved.result), ['backupPath']);
  assert.equal(readFileSync(approved.result.backupPath, 'utf8'), '{}');
  assert.deepEqual(await hook, { output: { hookSpecificOutput: { hookEventName: 'PermissionDenied', retry: true } } });
});

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

test('missing or wrong parameters are bad_params', () => {
  const { handlers } = setup();
  assert.throws(() => handlers['request.answer']({ id: 'x' }), { message: 'bad_params' });
  assert.throws(() => handlers['request.answer']({ id: 'x', behavior: 'maybe' }), { message: 'bad_params' });
  assert.throws(() => handlers.steps({}), { message: 'bad_params' });
});

test('refusal.preview RPC checks the pending item and project before the rule parameter', async () => {
  const { handlers, approvals } = setup();
  assert.deepEqual(await rpc(handlers, 'refusal.preview', { id: 'missing' }), { id: 15, error: 'not_pending' });
  const closed = new AbortController();
  const hook = approvals.refusal({ session_id: 's1', hook_event_name: 'PermissionDenied' }, closed.signal);
  assert.deepEqual(await rpc(handlers, 'refusal.preview', { id: 'id1' }), { id: 15, error: 'project_unknown' });
  closed.abort();
  await hook;
});
