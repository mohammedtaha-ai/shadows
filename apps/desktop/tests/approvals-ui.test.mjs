import { test, after } from 'node:test';
import assert from 'node:assert/strict';
import React from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { createServer } from 'vite';

const server = await createServer({ configFile: false, server: { middlewareMode: true }, appType: 'custom' });
after(() => server.close());
const { ApprovalsPanel } = await server.ssrLoadModule('/src/features/approvals/ApprovalsPanel.tsx');

const base = {
  kind: 'request', tool: 'Bash', target: 'npm run deploy', reason: null,
  proposedRule: null, at: '2026-10-13T19:00:00Z', held: true,
};

test('identical requests expose their own project and session before approval', () => {
  const pending = [
    { ...base, id: 'one', sessionId: 'session-one', project: 'C:/work/first' },
    { ...base, id: 'two', sessionId: 'session-two', project: 'C:/work/second' },
  ];
  const html = renderToStaticMarkup(React.createElement(ApprovalsPanel, {
    state: { sessions: [], pending, hooksInstalled: true }, refresh: async () => {},
  }));
  assert.match(html, /C:\/work\/first/);
  assert.match(html, /session-one/);
  assert.match(html, /C:\/work\/second/);
  assert.match(html, /session-two/);
});

test('a refusal exposes its project and session before rule approval', () => {
  const pending = [{ ...base, id: 'refusal-one', kind: 'refusal', sessionId: 'session-refused', project: 'C:/work/refused', proposedRule: 'Bash(npm run deploy)' }];
  const html = renderToStaticMarkup(React.createElement(ApprovalsPanel, {
    state: { sessions: [], pending, hooksInstalled: true }, refresh: async () => {},
  }));
  assert.match(html, /C:\/work\/refused/);
  assert.match(html, /session-refused/);
});
