# A Claude subagent over ACP

- **Date:** 2026-10-05
- **Adapter:** `@agentclientprotocol/claude-agent-acp` 0.81.1 (the pinned one), ACP SDK 1.5.0
- **Claude Code:** 2.1.289, on Windows 11, Claude Pro
- **Session:** `model=sonnet`, `effort=medium`, `mode=auto`; a folder holding `alpha.txt` and `beta.md`
- **Asked by:** Mohammed, before designing how a subagent the Planner launches is
  shown (a card in the conversation, its own transcript, later a per-project list).

The probe was a throwaway Node script, never committed; its source is at the end
of this file and the script is deleted. It ran one prompt twice: asking the
model to launch one subagent on Sonnet to list and read the folder's files.
Run `plain` sent `clientCapabilities: {}` (what Shadows sends today); run
`optin` sent `clientCapabilities: { subagents: {} }`.

## Findings

1. **The opt-in changed nothing here.** Both runs sent every update on the one
   root session; no child session was announced. The adapter's
   `NativeSubagentRuntime` (`dist/native-subagents.js`) routes to child sessions
   only when the client advertises the AIR capability `nativeSubagentSessions`;
   that path was not exercised. Everything below needs no opt-in.
2. **The subagent is one `tool_call` on the parent**, `kind: "think"`, with
   `_meta.claudeCode = {toolName: "Agent", subagent: true}`. Its title starts as
   `"Task"`, then becomes the description ("List and read folder files").
   `rawInput` fills in over several `tool_call_update`s:
   `description`, `subagent_type` (`general-purpose`), `model` (`sonnet`), `prompt`.
3. **The subagent's own tool calls stream live on the same session**, each
   carrying `_meta.claudeCode.parentToolUseId` = the Agent call's `toolCallId`:
   a Glob ("Find …"), "Read alpha.txt", "Read beta.md", and finally a
   `SubagentHandback` call whose `rawInput.message` is the subagent's report.
   Each has its own `pending` → `completed` updates and a `toolResponse`
   (e.g. `filenames`, `durationMs`).
4. **The subagent's text is not streamed.** All 26 `agent_message_chunk`s were
   the parent's; none carried `parentToolUseId`. The subagent's words reach the
   client only as the `SubagentHandback` input and the final `toolResponse`.
5. **The Agent call's last `tool_call_update` carries the card's numbers** in
   `_meta.claudeCode.toolResponse`: `status: "completed"`, `agentId`,
   `agentType`, `resolvedModel: "claude-sonnet-5-5"`, `totalDurationMs: 13665`,
   `totalTokens: 43119`, `totalToolUseCount: 4`, `usage`, `toolStats`
   (`readCount`, `searchCount` …), `prompt`, and `handbackReport.text`. Its
   `content` says only that the report was delivered by `SubagentHandback`.
6. The parent then answered in one line using the report; the turn resolved
   `end_turn` (plain: 38.6 s, ~100k tokens; optin: 18.7 s).

## Source of the probe

```js
// Throwaway spike: how a Claude subagent reaches an ACP client, with and without opt-in.
import { spawn } from 'node:child_process'
import { Readable, Writable } from 'node:stream'
import { appendFileSync, mkdirSync, writeFileSync } from 'node:fs'
import { pathToFileURL } from 'node:url'

const HARNESS = 'E:/Globalprojects/shadows/harness/claude/node_modules/@agentclientprotocol'
const acp = await import(pathToFileURL(`${HARNESS}/sdk/dist/acp.js`).href)
const HERE = new URL('.', import.meta.url).pathname.replace(/^\/(\w:)/, '$1')
const PROJECT = `${HERE}project`
mkdirSync(PROJECT, { recursive: true })
writeFileSync(`${PROJECT}/alpha.txt`, 'the first file\n')
writeFileSync(`${PROJECT}/beta.md`, '# Beta\nThe second file.\n')

async function run(label, clientCapabilities) {
  const LOG = `${HERE}log-${label}.jsonl`
  writeFileSync(LOG, '')
  const t0 = Date.now()
  const log = (kind, data) => appendFileSync(LOG, JSON.stringify({ t: Date.now() - t0, kind, data }) + '\n')
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
  const conn = new acp.ClientSideConnection(() => client,
    acp.ndJsonStream(Writable.toWeb(child.stdin), Readable.toWeb(child.stdout)))
  log('initialize', await conn.initialize({ protocolVersion: 1, clientCapabilities }))
  const session = await conn.newSession({ cwd: PROJECT, mcpServers: [] })
  const sessionId = session.sessionId
  log('newSession', { sessionId })
  for (const opt of session.configOptions ?? []) {
    const flat = (opt.options ?? []).flatMap((o) => o.options ?? [o])
    const want = opt.category === 'model' ? flat.find((o) => /sonnet/i.test(o.value))
      : /effort/i.test(opt.id) ? flat.find((o) => o.value === 'medium') : undefined
    if (want) await conn.setSessionConfigOption({ sessionId, configId: opt.id, value: want.value })
  }
  const text = 'Launch exactly one subagent (the Agent tool) with model sonnet. Its job: list the files ' +
    'in the current folder and read each one, then report their contents. Do not read the files ' +
    'yourself. When it returns, reply with one line summarizing what it found.'
  log('send', { text })
  try { log('resolved', await conn.prompt({ sessionId, prompt: [{ type: 'text', text }] })) }
  catch (e) { log('rejected', String(e?.message ?? e)) }
  await new Promise((r) => setTimeout(r, 1500))
  child.kill()
}

await run('plain', {})
await run('optin', { subagents: {} })
process.exit(0)
```
