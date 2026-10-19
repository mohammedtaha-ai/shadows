import { test } from 'node:test';
import assert from 'node:assert/strict';
import { proposeRule, isNarrowRule } from './rules.ts';

const project = 'C:\\work\\proj';

test('Bash and PowerShell propose the exact command', () => {
  assert.equal(proposeRule('Bash', { command: 'npm run deploy:staging' }, project), 'Bash(npm run deploy:staging)');
  assert.equal(proposeRule('PowerShell', { command: 'Get-ChildItem src' }, project), 'PowerShell(Get-ChildItem src)');
});

test('a command with a newline, a closing parenthesis or a * proposes nothing', () => {
  assert.equal(proposeRule('Bash', { command: 'a\nb' }, project), null);
  assert.equal(proposeRule('Bash', { command: 'echo $(date)' }, project), null);
  assert.equal(proposeRule('Bash', { command: '  ' }, project), null);
  // Claude Code reads * in a Bash rule as "any text", so the rule would cover more than this command.
  assert.equal(proposeRule('Bash', { command: 'rm -rf build/*' }, project), null);
});

test('file tools propose the path anchored at the project root', () => {
  assert.equal(proposeRule('Write', { file_path: 'C:\\work\\proj\\src\\app.ts' }, project), 'Edit(/src/app.ts)');
  // Without the leading /, Edit(a.txt) would match an a.txt in every folder.
  assert.equal(proposeRule('Edit', { file_path: 'c:\\WORK\\proj\\a.txt' }, project), 'Edit(/a.txt)');
  assert.equal(proposeRule('Read', { file_path: 'C:\\work\\proj\\x.md' }, project), 'Read(/x.md)');
});

test('a file outside the project, on another drive, or with a pattern character proposes nothing', () => {
  assert.equal(proposeRule('Edit', { file_path: 'C:\\work\\other\\a.txt' }, project), null);
  assert.equal(proposeRule('Edit', { file_path: 'D:\\proj\\a.txt' }, project), null);
  assert.equal(proposeRule('Edit', { file_path: 'C:\\work\\proj' }, project), null);
  assert.equal(proposeRule('Edit', { file_path: 'C:\\work\\proj\\[draft].md' }, project), null);
  assert.equal(proposeRule('Edit', { file_path: 'C:\\work\\proj\\a.txt' }, ''), null);
});

test('WebFetch proposes the host and MCP the single tool', () => {
  assert.equal(proposeRule('WebFetch', { url: 'https://example.com/a?b=1' }, project), 'WebFetch(domain:example.com)');
  assert.equal(proposeRule('WebFetch', { url: 'not a url' }, project), null);
  assert.equal(proposeRule('mcp__github__create_issue', {}, project), 'mcp__github__create_issue');
  assert.equal(proposeRule('Task', {}, project), null);
});

test('only exact rules are accepted', () => {
  for (const broad of [
    'Bash', 'Bash(*)', 'Bash(:*)', 'Bash( )', 'Bash(npm test:*)', 'Bash(git *)', 'Bash(rm -rf build/*)', 'PowerShell(*)',
    'Edit', 'Edit(**)', 'Edit(/**)', 'Edit(**/*)', 'Edit(src/**)', 'Edit(/src/**)', 'Edit(a.txt)', 'Edit(/)', 'Edit(/src/)',
    'Edit(//c/x.txt)', 'Edit(~/x.txt)', 'Edit(/../x.txt)', 'Read(/a?.md)', 'Write(/a.txt)',
    'WebFetch(domain:*)', 'WebFetch(domain:*.example.com)', 'WebFetch(example.com)',
    '*', '', 'mcp__github', 'mcp__github__*', 'Task(x)',
  ])
    assert.equal(isNarrowRule(broad), false, broad);
  for (const narrow of ['Bash(npm test)', 'PowerShell(npm test)', 'Edit(/src/app.ts)', 'Read(/docs/x.md)',
    'WebFetch(domain:example.com)', 'mcp__github__create_issue'])
    assert.equal(isNarrowRule(narrow), true, narrow);
});

test('every proposed rule is accepted as narrow', () => {
  const proposed = [
    proposeRule('Bash', { command: 'npm run deploy:staging' }, project),
    proposeRule('PowerShell', { command: 'Get-ChildItem src' }, project),
    proposeRule('Write', { file_path: 'C:\\work\\proj\\src\\app.ts' }, project),
    proposeRule('Read', { file_path: 'C:\\work\\proj\\x.md' }, project),
    proposeRule('WebFetch', { url: 'https://example.com/a' }, project),
    proposeRule('mcp__github__create_issue', {}, project),
  ];
  for (const rule of proposed) assert.equal(rule !== null && isNarrowRule(rule), true, String(rule));
});
