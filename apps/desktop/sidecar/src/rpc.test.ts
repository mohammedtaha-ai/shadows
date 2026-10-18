import { test } from 'node:test';
import assert from 'node:assert/strict';
import { PassThrough } from 'node:stream';
import { serveRpc } from './rpc.ts';

async function exchange(lines: string[], handlers: Parameters<typeof serveRpc>[2]) {
  const input = new PassThrough();
  const output = new PassThrough();
  let out = '';
  output.setEncoding('utf8').on('data', d => { out += d; });
  const ended = new Promise<void>(resolve => serveRpc(input, output, handlers, resolve));
  for (const line of lines) input.write(line + '\n');
  input.end();
  await ended;
  await new Promise(r => setImmediate(r));
  return out.trim().split('\n').map(l => JSON.parse(l));
}

test('a known method returns its result under the same id', async () => {
  assert.deepEqual(await exchange([JSON.stringify({ id: 1, method: 'state', params: {} })], { state: () => ({ ok: 1 }) }),
    [{ id: 1, result: { ok: 1 } }]);
});

test('unknown methods, stable errors and unexpected errors come back as codes', async () => {
  const replies = await exchange([
    JSON.stringify({ id: 1, method: 'shell', params: {} }),
    JSON.stringify({ id: 2, method: 'bad', params: {} }),
    JSON.stringify({ id: 3, method: 'oops', params: {} }),
    JSON.stringify({ id: 4, method: 'toString', params: {} }),
    'not json',
  ], { bad: () => { throw new Error('rule_too_broad'); }, oops: () => { throw new Error('C:\\secret path'); } });
  // Synchronous refusals are written before handler results, so compare by id.
  replies.sort((a, b) => String(a.id).localeCompare(String(b.id)));
  assert.deepEqual(replies, [
    { id: 1, error: 'unknown_method' },
    { id: 2, error: 'rule_too_broad' },
    { id: 3, error: 'action_failed' },
    { id: 4, error: 'unknown_method' },
    { id: null, error: 'bad_request' },
  ]);
});

test('unexpected error details are not written to stderr', async () => {
  const written: string[] = [];
  const original = process.stderr.write;
  process.stderr.write = ((chunk: string | Uint8Array) => { written.push(String(chunk)); return true; }) as typeof process.stderr.write;
  try {
    assert.deepEqual(await exchange([JSON.stringify({ id: 1, method: 'fail', params: {} })], {
      fail: () => { throw new Error('token=SECRET-CONTENT'); },
    }), [{ id: 1, error: 'action_failed' }]);
  } finally {
    process.stderr.write = original;
  }
  assert.equal(written.join('').includes('SECRET-CONTENT'), false);
});
