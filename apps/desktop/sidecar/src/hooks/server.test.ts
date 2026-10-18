import { test } from 'node:test';
import assert from 'node:assert/strict';
import { connect } from 'node:net';
import { randomBytes } from 'node:crypto';
import { startHookServer, type Route } from './server.ts';

const pipeName = () => String.raw`\\.\pipe\aylul-test-` + randomBytes(8).toString('hex');
const token = 'a'.repeat(64);
const event = { session_id: 's1', hook_event_name: 'Stop' };
const line = (t: string) => JSON.stringify({ v: 1, token: t, event }) + '\n';

function call(pipe: string, text: string): Promise<string> {
  return new Promise((resolve, reject) => {
    const socket = connect(pipe, () => socket.write(text));
    let out = '';
    socket.setEncoding('utf8');
    socket.on('data', d => { out += d; });
    socket.on('close', () => resolve(out));
    socket.on('error', reject);
  });
}

test('a request with the token gets the route reply as one line', async () => {
  const pipe = pipeName();
  const server = await startHookServer(pipe, token, async () => ({ output: { ok: true } }));
  try {
    assert.deepEqual(JSON.parse(await call(pipe, line(token))), { output: { ok: true } });
  } finally { server.close(); }
});

test('a request without the right token is dropped unanswered', async () => {
  const pipe = pipeName();
  let called = false;
  const server = await startHookServer(pipe, token, async () => { called = true; return {}; });
  try {
    assert.equal(await call(pipe, line('b'.repeat(64))), '');
    assert.equal(called, false);
  } finally { server.close(); }
});

test('a malformed line is dropped unanswered', async () => {
  const pipe = pipeName();
  const server = await startHookServer(pipe, token, async () => ({}));
  try {
    assert.equal(await call(pipe, '{not json\n'), '');
  } finally { server.close(); }
});

test('the project root from aylul-hook reaches the route; one sent inside the event does not', async () => {
  const pipe = pipeName();
  const seen: Array<string | undefined> = [];
  const server = await startHookServer(pipe, token, async e => { seen.push(e.project_dir); return {}; });
  try {
    await call(pipe, JSON.stringify({ v: 1, token, project_dir: 'C:\\proj', event }) + '\n');
    await call(pipe, JSON.stringify({ v: 1, token, event: { ...event, project_dir: 'C:\\other' } }) + '\n');
    assert.deepEqual(seen, ['C:\\proj', undefined]);
  } finally { server.close(); }
});

test('a failing route answers with no decision', async () => {
  const pipe = pipeName();
  const server = await startHookServer(pipe, token, async () => { throw new Error('boom'); });
  try {
    assert.deepEqual(JSON.parse(await call(pipe, line(token))), {});
  } finally { server.close(); }
});

test('an answered request does not abort the route signal', async () => {
  const pipe = pipeName();
  let signal: AbortSignal | undefined;
  const route: Route = async (_event, closed) => { signal = closed; return { output: { ok: true } }; };
  const server = await startHookServer(pipe, token, route);
  try {
    assert.deepEqual(JSON.parse(await call(pipe, line(token))), { output: { ok: true } });
    await new Promise(resolve => setTimeout(resolve, 50));
    assert.ok(signal);
    assert.equal(signal.aborted, false);
  } finally { server.close(); }
});

test('the route learns when the hook hangs up before the answer', async () => {
  const pipe = pipeName();
  let aborted: Promise<void> | undefined;
  const route: Route = (_event, closed) => {
    aborted = new Promise(resolve => closed.addEventListener('abort', () => resolve()));
    return new Promise(() => {});
  };
  const server = await startHookServer(pipe, token, route);
  try {
    await new Promise<void>(resolve => {
      const socket = connect(pipe, () => {
        socket.write(line(token));
        setTimeout(() => { socket.destroy(); resolve(); }, 100);
      });
    });
    assert.ok(aborted);
    await aborted;
  } finally { server.close(); }
});
