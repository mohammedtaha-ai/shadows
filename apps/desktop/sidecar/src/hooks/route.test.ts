import { test } from 'node:test';
import assert from 'node:assert/strict';
import { makeRoute } from './route.ts';
import { Store } from '../store/store.ts';
import { Approvals } from '../approvals/approvals.ts';

function setup() {
  const store = new Store(':memory:');
  const approvals = new Approvals({ now: () => new Date('2026-10-13T19:00:00.000Z'), newId: () => 'id1',
    writeRule: () => ({ ruleId: 'r', backupPath: '' }), record: (i, o) => store.recordDecision(i, o, i.at),
    requestHoldMs: 200, refusalHoldMs: 200 });
  const route = makeRoute({ store, approvals, now: () => new Date('2026-10-13T19:00:00.000Z') });
  return { store, approvals, route };
}
const never = new AbortController().signal;

test('a PostToolUse event saves the session and a trimmed step, and answers at once', async () => {
  const { store, route } = setup();
  const reply = await route({ session_id: 's1', hook_event_name: 'PostToolUse', cwd: 'C:\\p', tool_name: 'Write',
    tool_input: { file_path: 'C:\\p\\a.txt', content: 'SECRET' }, tool_use_id: 't1' }, never);
  assert.deepEqual(reply, {});
  assert.equal(store.getSession('s1')?.lastStep?.target, 'C:\\p\\a.txt');
  assert.equal(JSON.stringify(store.steps('s1')).includes('SECRET'), false);
});

test('a PermissionRequest waits in approvals', async () => {
  const { approvals, route } = setup();
  const reply = route({ session_id: 's1', hook_event_name: 'PermissionRequest', cwd: 'C:\\p', tool_name: 'Bash',
    tool_input: { command: 'npm test' } }, never);
  await new Promise(r => setImmediate(r));
  approvals.answer(approvals.pending()[0].id, 'allow');
  assert.equal(((await reply).output as any).hookSpecificOutput.decision.behavior, 'allow');
});

test('a call covered by a recorded rule is noted as a grant use', async () => {
  const { store, route } = setup();
  const ruleId = store.addRule('C:\\p', 'Bash(npm test)', '', '2026-10-13T18:00:00.000Z');
  await route({ session_id: 's1', hook_event_name: 'PostToolUse', cwd: 'C:\\p\\src', project_dir: 'C:\\p', tool_name: 'Bash',
    tool_input: { command: 'npm test' }, tool_use_id: 't9' }, never);
  await route({ session_id: 's1', hook_event_name: 'PostToolUse', cwd: 'C:\\p', project_dir: 'C:\\p', tool_name: 'Bash',
    tool_input: { command: 'npm run build' }, tool_use_id: 't10' }, never);
  assert.deepEqual(store.grantUses(), [{ ruleId, sessionId: 's1', toolUseId: 't9' }]);
});

test('a step after a terminal answer marks it allowed', async () => {
  const { store, route } = setup();
  const hook = new AbortController();
  const waiting = route({ session_id: 's1', hook_event_name: 'PermissionRequest', cwd: 'C:\\p', tool_name: 'Bash',
    tool_input: { command: 'npm test' } }, hook.signal);
  await new Promise(r => setImmediate(r));
  hook.abort();
  await waiting;
  await route({ session_id: 's1', hook_event_name: 'PostToolUse', cwd: 'C:\\p', tool_name: 'Bash',
    tool_input: { command: 'npm test' }, tool_use_id: 't1' }, never);
  assert.equal(store.decisions()[0].outcome, 'answered_in_terminal_allowed');
});
