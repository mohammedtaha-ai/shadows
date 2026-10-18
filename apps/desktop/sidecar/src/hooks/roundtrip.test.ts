import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createEndpoint, removeEndpoint } from './endpoint.ts';
import { startHookServer } from './server.ts';

const exe = process.env.AYLUL_HOOK_EXE;
const event = JSON.stringify({ session_id: 's1', hook_event_name: 'Stop' });

function runHook(localAppData: string): Promise<{ code: number | null; stdout: string }> {
  return new Promise((resolve, reject) => {
    const child = spawn(exe!, [], { env: { ...process.env, LOCALAPPDATA: localAppData, CLAUDE_PROJECT_DIR: 'C:\\proj-x' } });
    let stdout = '';
    child.stdout.setEncoding('utf8').on('data', d => { stdout += d; });
    child.on('error', reject);
    child.on('close', code => resolve({ code, stdout }));
    child.stdin.end(event);
  });
}

test('aylul-hook.exe carries an event and prints the answer', { skip: !exe && 'set AYLUL_HOOK_EXE' }, async () => {
  const home = mkdtempSync(join(tmpdir(), 'aylul-'));
  const endpoint = createEndpoint(join(home, 'AYLUL'));
  const server = await startHookServer(endpoint.pipe, endpoint.token,
    async e => ({ output: { got: e.hook_event_name, project: e.project_dir } }));
  try {
    const result = await runHook(home);
    assert.equal(result.code, 0);
    assert.deepEqual(JSON.parse(result.stdout), { got: 'Stop', project: 'C:\\proj-x' });
  } finally {
    server.close();
    removeEndpoint(endpoint);
    rmSync(home, { recursive: true, force: true });
  }
});

test('aylul-hook.exe with no sidecar prints nothing and exits 0', { skip: !exe && 'set AYLUL_HOOK_EXE' }, async () => {
  const home = mkdtempSync(join(tmpdir(), 'aylul-'));
  try {
    const result = await runHook(home);
    assert.equal(result.code, 0);
    assert.equal(result.stdout, '');
  } finally {
    rmSync(home, { recursive: true, force: true });
  }
});
