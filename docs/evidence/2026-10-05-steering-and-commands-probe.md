# Queue, steering, slash commands and spinner words over ACP

- **Date:** 2026-10-05
- **Adapter:** `@agentclientprotocol/claude-agent-acp` 0.81.1 (the pinned one), ACP SDK 1.5.0
- **Claude Code:** 2.1.289, on Windows 11, Claude Pro
- **Session:** `model=sonnet` (`claude-sonnet-5-5`), `effort=medium`, `mode=auto`, an empty project folder
- **Asked by:** Mohammed, before designing "write while it works" (a queue and Send now),
  the `/` menu, and the CLI's spinner word and tip in the web client.

The probe was a throwaway Node script speaking ACP to the adapter. It was never
committed; its source is at the end of this file and the script is deleted.

## Findings

1. **A second `session/prompt` while a turn runs is queued by the adapter.**
   A1 (count to 40) was sent at 2.7 s, A2 ("reply BANANA") at 6.7 s. A1 ran to
   its end untouched and resolved `end_turn` at 13.2 s; A2's reply streamed
   after it and resolved `end_turn` at 17.4 s. Each prompt resolves on its own.
   The queued message is not echoed: no `user_message_chunk` was sent.
2. **`_session/steering` during a turn injects into it.** The adapter
   advertises it at `initialize` (`_meta.steering.supported: true`). B1 (count
   to 40) had streamed "1. One is" when the steer was sent at 21.4 s; the
   request answered `{"outcome":"injected"}` within 4 ms. The counting stopped,
   the reply "STEERED" streamed, and **B1's own `session/prompt` resolved
   `end_turn`** at 29.3 s with the steered reply's usage. The steer gets no
   turn of its own, and its text is not echoed either.
3. **`_session/steering` with no running turn**, sent with
   `_meta.steering.idleBehavior: "promptRequired"`, answers
   `{"outcome":"promptRequired","reason":"noRunningTurn"}` and starts nothing.
   Without that `_meta` the adapter starts a detached turn itself (read in
   `dist/acp-agent.js`, not exercised).
4. **`available_commands_update` carries the `/` menu.** It arrived right after
   `session/new` (144 entries) and again at the first prompt (140). Each entry is
   `{name, description, input?}`; `input` is `{hint}` (e.g.
   `"[app directory path or GitHub URL]"`). It holds skills
   (`superpowers:brainstorming`), plugin commands (`codex:review`) and built-in
   commands (`compact`, `context`, `model`, `effort`, `init`, `review` …)
   together, with no field saying which is which; a description sometimes ends
   with its source in parentheses, e.g. `(user)`.
5. **No spinner word and no tip are sent.** Over the whole run the only
   `session/update` kinds were `agent_message_chunk` (148),
   `usage_update` (10), `available_commands_update` (2) and
   `session_info_update` (1, an auto title). No field anywhere held a spinner
   word or a tip. The words exist only inside `claude.exe`, as a fixed array
   ("Simmering", "Tempering", "Thinking", …) that the terminal UI draws from.
6. Also seen: `_auth/status_update` notifications (plan and account),
   `usage_update` carrying `cost` and `_claude/rateLimit` (`five_hour`,
   `resetsAt`), and `session_info_update` with a generated `title`.

## Source of the probe

```js
// Throwaway spike: queue, steering, slash commands, spinner words over ACP.
import { spawn } from 'node:child_process'
import { Readable, Writable } from 'node:stream'
import { appendFileSync, mkdirSync, writeFileSync } from 'node:fs'
import { pathToFileURL } from 'node:url'

const HARNESS = 'E:/Globalprojects/shadows/harness/claude/node_modules/@agentclientprotocol'
const acp = await import(pathToFileURL(`${HARNESS}/sdk/dist/acp.js`).href)
const HERE = new URL('.', import.meta.url).pathname.replace(/^\/(\w:)/, '$1')
const PROJECT = `${HERE}project`
const LOG = `${HERE}log.jsonl`
mkdirSync(PROJECT, { recursive: true })
writeFileSync(LOG, '')
const t0 = Date.now()
const log = (kind, data) => appendFileSync(LOG, JSON.stringify({ t: Date.now() - t0, kind, data }) + '\n')
const sleep = (ms) => new Promise((r) => setTimeout(r, ms))

const child = spawn(process.execPath, [`${HARNESS}/claude-agent-acp/dist/index.js`], {
  cwd: PROJECT,
  env: { ...process.env, CLAUDE_CODE_EXECUTABLE: `${process.env.USERPROFILE}\\.local\\bin\\claude.exe` },
  stdio: ['pipe', 'pipe', 'pipe'],
})
child.stderr.on('data', (d) => log('stderr', d.toString()))

const client = {
  async sessionUpdate(n) { log('update', n) },
  async requestPermission(p) {
    log('permission', p)
    const o = p.options.find((x) => x.kind === 'allow_once') ?? p.options[0]
    return { outcome: { outcome: 'selected', optionId: o.optionId } }
  },
  async extNotification(m, p) { log('extNotification', { m, p }) },
  async extMethod(m, p) { log('extMethod', { m, p }); return {} },
}
const stream = acp.ndJsonStream(Writable.toWeb(child.stdin), Readable.toWeb(child.stdout))
const conn = new acp.ClientSideConnection(() => client, stream)

const init = await conn.initialize({ protocolVersion: 1, clientCapabilities: {} })
log('initialize', init)
const session = await conn.newSession({ cwd: PROJECT, mcpServers: [] })
log('newSession', session)
const sessionId = session.sessionId

for (const opt of session.configOptions ?? []) {
  const flat = (opt.options ?? []).flatMap((o) => o.options ?? [o])
  const want = opt.category === 'model' ? flat.find((o) => /sonnet/i.test(o.value))
    : /effort/i.test(opt.id) ? flat.find((o) => o.value === 'medium') : undefined
  if (want) {
    log('setConfig', await conn.setSessionConfigOption({ sessionId, configId: opt.id, value: want.value }))
  }
}

const long = 'Count from 1 to 40, one number per line, with one short sentence about each number. Use no tools.'
const prompt = (text, tag) => {
  log('send', { tag, text })
  return conn.prompt({ sessionId, prompt: [{ type: 'text', text }] })
    .then((r) => log('resolved', { tag, r }), (e) => log('rejected', { tag, e: String(e?.message ?? e) }))
}

// A: a second session/prompt while the first runs.
const a1 = prompt(long, 'A1-long')
await sleep(4000)
const a2 = prompt('Reply with exactly the word BANANA.', 'A2-queued')
await Promise.all([a1, a2])

// B: _session/steering while a turn runs.
const b1 = prompt(long, 'B1-long')
await sleep(4000)
log('send', { tag: 'B-steer' })
try {
  log('steerResult', await conn.extMethod('_session/steering', {
    sessionId, prompt: [{ type: 'text', text: 'Stop counting now. Reply with exactly the word STEERED.' }],
  }))
} catch (e) { log('steerError', String(e?.message ?? e)) }
await b1

// C: steering when idle, with the host-owned fallback.
try {
  log('steerIdle', await conn.extMethod('_session/steering', {
    sessionId, prompt: [{ type: 'text', text: 'hi' }], _meta: { steering: { idleBehavior: 'promptRequired' } },
  }))
} catch (e) { log('steerIdleError', String(e?.message ?? e)) }

log('done', {})
child.kill()
process.exit(0)
```
