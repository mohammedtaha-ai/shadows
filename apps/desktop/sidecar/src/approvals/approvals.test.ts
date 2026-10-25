import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { Approvals, type Outcome, type Pending } from './approvals.ts';

function setup(hold = { request: 200, refusal: 200 }) {
  let n = 0;
  const records: Array<[string, Outcome]> = [];
  const written: Array<[string, string]> = [];
  const approvals = new Approvals({
    now: () => new Date('2026-10-13T19:00:00.000Z'),
    newId: () => `id${++n}`,
    writeRule: (project, rule, _seen) => { written.push([project, rule]); return { ruleId: 'r1', backupPath: 'b1' }; },
    record: (item: Pending, outcome) => { records.push([item.id, outcome]); },
    requestHoldMs: hold.request,
    refusalHoldMs: hold.refusal,
  });
  return { approvals, records, written };
}
const request = { session_id: 's1', hook_event_name: 'PermissionRequest', cwd: 'C:\\p\\src', project_dir: 'C:\\p', tool_name: 'Bash', tool_input: { command: 'npm test' } };
const refusal = { session_id: 's1', hook_event_name: 'PermissionDenied', cwd: 'C:\\p\\src', project_dir: 'C:\\p', tool_name: 'Bash', tool_input: { command: 'npm run deploy' }, reason: '[Prod Deploy]' };
const open = () => new AbortController();
// A preview reads the project's settings file, so an approval needs a real, empty project.
const projectRoot = () => mkdtempSync(join(tmpdir(), 'aylul-approvals-'));

test('allow from AYLUL answers the hook with allow', async () => {
  const { approvals, records } = setup();
  const reply = approvals.request(request, open().signal);
  const [item] = approvals.pending();
  assert.equal(item.target, 'npm test');
  approvals.answer(item.id, 'allow');
  assert.deepEqual(await reply, { output: { hookSpecificOutput: { hookEventName: 'PermissionRequest', decision: { behavior: 'allow' } } } });
  assert.deepEqual(records, [['id1', 'allowed']]);
  assert.deepEqual(approvals.pending(), []);
});

test('a PowerShell request shows the command before it can be answered', () => {
  const { approvals } = setup();
  void approvals.request({ ...request, tool_name: 'PowerShell', tool_input: { command: 'Remove-Item -Recurse C:/work/project' } }, open().signal);
  assert.equal(approvals.pending()[0].target, 'Remove-Item -Recurse C:/work/project');
});

test('a request keeps the command tail visible before approval', () => {
  const { approvals } = setup();
  const command = `echo ${'x'.repeat(180)} && rm -rf important`;
  void approvals.request({ ...request, tool_input: { command } }, open().signal);
  assert.equal(approvals.pending()[0].target, command);
});

test('deny from AYLUL answers with deny and a message', async () => {
  const { approvals } = setup();
  const reply = approvals.request(request, open().signal);
  approvals.answer(approvals.pending()[0].id, 'deny');
  const out = (await reply).output as { hookSpecificOutput: { decision: { behavior: string; message: string } } };
  assert.equal(out.hookSpecificOutput.decision.behavior, 'deny');
  assert.ok(out.hookSpecificOutput.decision.message.length > 0);
});

test('an answer in the terminal ends the request without a decision', async () => {
  const { approvals, records } = setup();
  const hook = open();
  const reply = approvals.request(request, hook.signal);
  hook.abort();
  assert.deepEqual(await reply, {});
  assert.deepEqual(records, [['id1', 'answered_in_terminal']]);
  assert.deepEqual(approvals.pending(), []);
});

test('an unanswered request times out with no decision', async () => {
  const { approvals, records } = setup({ request: 30, refusal: 200 });
  assert.deepEqual(await approvals.request(request, open().signal), {});
  assert.deepEqual(records, [['id1', 'timed_out']]);
  assert.deepEqual(approvals.pending(), []);
});

test('two requests are answered independently', async () => {
  const { approvals } = setup();
  const first = approvals.request(request, open().signal);
  const second = approvals.request({ ...request, session_id: 's2' }, open().signal);
  const [a, b] = approvals.pending();
  approvals.answer(b.id, 'deny');
  approvals.answer(a.id, 'allow');
  assert.equal(((await first).output as any).hookSpecificOutput.decision.behavior, 'allow');
  assert.equal(((await second).output as any).hookSpecificOutput.decision.behavior, 'deny');
});

test('answering twice or an unknown id is refused', async () => {
  const { approvals } = setup();
  void approvals.request(request, open().signal);
  const id = approvals.pending()[0].id;
  approvals.answer(id, 'allow');
  assert.throws(() => approvals.answer(id, 'allow'), { message: 'not_pending' });
  assert.throws(() => approvals.answer('nope', 'deny'), { message: 'not_pending' });
});

test('a refusal carries its reason and a proposed rule', async () => {
  const { approvals } = setup();
  void approvals.refusal(refusal, open().signal);
  const [item] = approvals.pending();
  assert.equal(item.kind, 'refusal');
  assert.equal(item.reason, '[Prod Deploy]');
  assert.equal(item.proposedRule, 'Bash(npm run deploy)');
});

test('approving the rule within the hold writes it, then asks Claude to retry', async () => {
  const { approvals, written, records } = setup();
  const root = projectRoot();
  const reply = approvals.refusal({ ...refusal, project_dir: root }, open().signal);
  const id = approvals.pending()[0].id;
  const result = approvals.approvePreview(id, 'Bash(npm run deploy)', approvals.previewRule(id, 'Bash(npm run deploy)').before);
  assert.deepEqual(result, { backupPath: 'b1' });
  assert.deepEqual(written, [[root, 'Bash(npm run deploy)']]);
  assert.deepEqual(await reply, { output: { hookSpecificOutput: { hookEventName: 'PermissionDenied', retry: true } } });
  assert.deepEqual(records, [['id1', 'rule_written']]);
});

test('a late approval still writes the rule', async () => {
  const { approvals, written, records } = setup({ request: 200, refusal: 30 });
  const root = projectRoot();
  assert.deepEqual(await approvals.refusal({ ...refusal, project_dir: root }, open().signal), {});
  const [item] = approvals.pending();
  assert.equal(item.held, false);
  approvals.approvePreview(item.id, 'Bash(npm run deploy)', approvals.previewRule(item.id, 'Bash(npm run deploy)').before);
  assert.deepEqual(written, [[root, 'Bash(npm run deploy)']]);
  assert.deepEqual(records, [['id1', 'listed'], ['id1', 'rule_written']]);
  assert.deepEqual(approvals.pending(), []);
});

test('a broad rule is refused and nothing is written', () => {
  const { approvals, written } = setup();
  void approvals.refusal({ ...refusal, project_dir: projectRoot() }, open().signal);
  const id = approvals.pending()[0].id;
  assert.throws(() => approvals.previewRule(id, 'Bash(*)'), { message: 'rule_too_broad' });
  assert.throws(() => approvals.approvePreview(id, 'Bash(*)', 'seen'), { message: 'preview_mismatch' });
  assert.deepEqual(written, []);
  assert.equal(approvals.pending().length, 1);
});

test('without a preview no rule is written', () => {
  const { approvals, written } = setup();
  void approvals.refusal({ ...refusal, project_dir: projectRoot() }, open().signal);
  const id = approvals.pending()[0].id;
  assert.throws(() => approvals.approvePreview(id, 'Bash(npm run deploy)', 'seen'), { message: 'preview_mismatch' });
  assert.deepEqual(written, []);
  assert.equal(approvals.pending().length, 1);
});

test('a refusal with no project root proposes nothing and cannot be approved', () => {
  const { approvals, written } = setup();
  const { project_dir: _root, ...withoutRoot } = refusal;
  void approvals.refusal(withoutRoot, open().signal);
  const [item] = approvals.pending();
  assert.equal(item.project, '');
  assert.equal(item.proposedRule, null);
  assert.throws(() => approvals.previewRule(item.id, 'Bash(npm run deploy)'), { message: 'project_unknown' });
  assert.throws(() => approvals.approvePreview(item.id, 'Bash(npm run deploy)', 'seen'), { message: 'preview_mismatch' });
  assert.deepEqual(written, []);
});

test('a dismissed refusal leaves the list and answers a held hook with no decision', async () => {
  const { approvals, records } = setup();
  const reply = approvals.refusal(refusal, open().signal);
  approvals.dismiss(approvals.pending()[0].id);
  assert.deepEqual(await reply, {});
  assert.deepEqual(records, [['id1', 'dismissed']]);
  assert.deepEqual(approvals.pending(), []);
});
