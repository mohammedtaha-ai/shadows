import { test } from 'node:test';
import assert from 'node:assert/strict';
import { trimStep, oneLine } from './steps.ts';
import { applyEvent, MAX_MESSAGE } from './session.ts';

const at = '2026-10-13T19:00:00.000Z';
const write = {
  session_id: 's1', hook_event_name: 'PostToolUse', cwd: 'C:\\p', tool_name: 'Write',
  tool_input: { file_path: 'C:\\p\\a.txt', content: 'SECRET-CONTENT' },
  tool_response: { content: 'SECRET-CONTENT', originalFile: 'OLD-SECRET', structuredPatch: [{ lines: ['+SECRET'] }] },
};

test('a step keeps tool and path and drops every content field', () => {
  const step = trimStep(write, at);
  assert.deepEqual(step, { at, tool: 'Write', target: 'C:\\p\\a.txt', summary: 'Write C:\\p\\a.txt' });
  assert.equal(JSON.stringify(step).includes('SECRET'), false);
});

test('a Bash step is the command on one line with its description', () => {
  const step = trimStep({ session_id: 's1', hook_event_name: 'PostToolUse', tool_name: 'Bash',
    tool_input: { command: 'npm\n  test', description: 'Run tests' }, tool_response: { stdout: 'SECRET' } }, at);
  assert.deepEqual(step, { at, tool: 'Bash', target: 'npm test', summary: 'Run tests' });
});

test('a URL step keeps scheme, host and path, never the query or fragment', () => {
  const step = trimStep({ session_id: 's1', hook_event_name: 'PostToolUse', tool_name: 'WebFetch',
    tool_input: { url: 'https://example.com/a?token=SECRET#x' } }, at);
  assert.deepEqual(step, { at, tool: 'WebFetch', target: 'https://example.com/a', summary: 'WebFetch https://example.com/a' });
  assert.equal(JSON.stringify(step).includes('SECRET'), false);
});

test('oneLine collapses whitespace and cuts long text', () => {
  assert.equal(oneLine(' a\n\tb '), 'a b');
  assert.equal(oneLine('x'.repeat(200), 10), 'x'.repeat(9) + '…');
});

test('a session moves through its states and keeps the last step and message', () => {
  let { session } = applyEvent(undefined, { session_id: 's1', hook_event_name: 'SessionStart', cwd: 'C:\\p' }, at);
  assert.equal(session.state, 'running');
  assert.equal(session.project, 'C:\\p');
  const afterStep = applyEvent(session, write, at);
  assert.equal(afterStep.step?.tool, 'Write');
  assert.equal(afterStep.session.lastStep?.target, 'C:\\p\\a.txt');
  session = applyEvent(afterStep.session, { session_id: 's1', hook_event_name: 'Notification', notification_type: 'permission_prompt' }, at).session;
  assert.equal(session.state, 'waiting');
  session = applyEvent(session, { session_id: 's1', hook_event_name: 'Stop', last_assistant_message: 'Done.' }, at).session;
  assert.equal(session.lastMessage, 'Done.');
  assert.equal(session.state, 'waiting');
  session = applyEvent(session, { session_id: 's1', hook_event_name: 'SessionEnd' }, at).session;
  assert.equal(session.state, 'ended');
});

test('an auth_success notice does not mark the session waiting', () => {
  const { session } = applyEvent(undefined, { session_id: 's1', hook_event_name: 'Notification', notification_type: 'auth_success' }, at);
  assert.equal(session.state, 'running');
});

test('the last message is kept whole up to its limit', () => {
  const exact = 'a'.repeat(MAX_MESSAGE);
  const kept = applyEvent(undefined, { session_id: 's1', hook_event_name: 'Stop', last_assistant_message: exact }, at);
  assert.equal(kept.session.lastMessage, exact);
  const long = 'a'.repeat(MAX_MESSAGE + 50);
  const { session } = applyEvent(undefined, { session_id: 's1', hook_event_name: 'Stop', last_assistant_message: long }, at);
  assert.equal(session.lastMessage?.length, MAX_MESSAGE);
});

test('the project is the root from aylul-hook, not the folder Claude moved into', () => {
  const first = applyEvent(undefined, { session_id: 's1', hook_event_name: 'SessionStart', cwd: 'C:\\p\\src', project_dir: 'C:\\p' }, at).session;
  assert.equal(first.project, 'C:\\p');
  const later = applyEvent(first, { session_id: 's1', hook_event_name: 'Stop', cwd: 'C:\\p\\src\\deep' }, at).session;
  assert.equal(later.project, 'C:\\p');
});

test('applyEvent does not change the session it was given', () => {
  const first = applyEvent(undefined, { session_id: 's1', hook_event_name: 'SessionStart', cwd: 'C:\\p' }, at).session;
  applyEvent(first, { session_id: 's1', hook_event_name: 'SessionEnd' }, at);
  assert.equal(first.state, 'running');
});
