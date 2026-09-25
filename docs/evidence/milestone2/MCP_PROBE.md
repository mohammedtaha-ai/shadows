# MCP probe — Milestone 2 Task 0

- **Date:** 2026-09-25
- **Adapter:** `@agentclientprotocol/claude-agent-acp` 0.81.1 (`agentInfo.version`)
- **Claude Code:** 2.1.281, passed as `CLAUDE_CODE_EXECUTABLE=%USERPROFILE%\.local\bin\claude.exe` (its MCP client identifies as `claude-code/2.1.281 (claude-desktop, agent-sdk/0.3.280)`)
- **`rmcp`:** 3.4.1, read from its published source (`src/model.rs`, `src/transport/streamable_http_server/tower.rs`)
- **Node:** 24.15.0 · **OS:** Windows 11
- **Probe:** throwaway and not committed, run from a scratch directory outside the repository, like `harness/ACP_PROBE.md`. It had two parts:
  - a raw JSON-RPC ACP client (newline-delimited JSON over the adapter's stdio, no SDK);
  - a minimal MCP server in Node with one tool, `echo_marker`, over Streamable HTTP. It required `Authorization: Bearer probe-token` and answered with plain JSON.

  Every line in both directions was logged; the lines below are quoted from those logs, trimmed.
- **Cost:** 11 prompts on `sonnet`, in three runs. The first run was cut off after its Step 3. The second repeated Step 3 without the flaw described in §3 and ran Steps 2b and 5. The third tested a cold resume.

Answers spec §13.13's Task 0 items 1–4, plus the 401 check of §13.7.

## 1. `append` reaches Claude, and Claude Code's tools remain — pass

`session/new` with `_meta.systemPrompt = { "append": "When asked for the marker, answer exactly: ORCHID-7." }`:

- The prompt "what is the marker?" was answered `ORCHID-7`.
- "List every tool you can call" was answered with Claude Code's own tools (`Agent, Bash, Edit, Glob, Grep, PowerShell, Read, Write, WebFetch, …`) and `mcp__shadows__echo_marker`.
- It also listed the person's own claude.ai connectors (`mcp__claude_ai_Google_Drive__*`). **The Planner inherits the person's Claude configuration**, including MCP servers Shadows did not pass.

## 2. `allowedTools` pre-approves Shadows' tools — pass

The session was set to mode `acceptEdits` with `mcpServers` = one http server named `shadows`, and the prompt was "Call the echo_marker tool with text hi…".

| `_meta.claudeCode.options.allowedTools` | `session/request_permission` | Reply |
|---|---|---|
| `["mcp__shadows__*"]` | none | `ECHO-MARKER:hi` |
| absent | one, for `mcp__shadows__echo_marker` | `ECHO-MARKER:hi` (the probe answered `allow_once`) |

```text
<< {"method":"session/request_permission","params":{"toolCall":{"name":"mcp__shadows__echo_marker","rawInput":{"text":"hi"},
    "_meta":{"claudeCode":{"toolName":"mcp__shadows__echo_marker","mcpServer":{"name":"shadows","source":"dynamic"}}}},
    "options":[{"optionId":"allow-once","kind":"allow_once"},{"optionId":"allow-with-updates","kind":"allow_always"},{"optionId":"reject",…}]}}
```

With `allowedTools` set, the adapter warns on stderr:

```text
[CLAUDE_SDK_CAN_USE_TOOL_SHADOWED] Warning: canUseTool will not be invoked for: mcp__shadows__*. Bare allowedTools entries auto-approve the whole tool before the callback is consulted.
```

This is the intended effect. Milestone 1's refusal of every permission request never sees Shadows' tools, and the grant stays the authority (§13.7).

Claude called `ToolSearch` before the MCP tool: MCP tools are deferred in Claude Code 2.1.281, so the Planner's first use of a Shadows tool in a session costs one extra step.

## 3. A resume keeps the conversation and the original `append` — **fail**

**Run 1 was flawed.**

- It followed `ACP_PROBE.md` §4: the prompt "remember the word amber" was answered by Claude writing the word to its auto-memory, at `~\.claude\projects\<cwd>\memory\remember_word_amber.md`.
- Recalling "amber" afterwards therefore proved nothing about the conversation.
- The marker had been asked before the resume, so an old answer could also come from history.

**Run 2** (warm resume: the same adapter process):

1. `session/new` with `append` "When asked for the marker, answer exactly: ORCHID-7." The marker was never asked.
2. The prompt "Pick a random four-digit number…" was answered `4729`.
3. `session/resume` on the same session with `append` changed to "…answer exactly: LILAC-3.", the same `mcpServers` and the same `allowedTools`.
4. The prompt "What is the marker, and what number did you pick earlier?" was answered **`ORCHID-7, 4729`**.

The adapter did rebuild its session: its fingerprint covers `systemPrompt` (`computeSessionFingerprint`, `acp-agent.js:392`). stderr shows the second query started with `resume=<same id>`, and the model reset to the settings default (`set-model … model=opus`). A second resume with the same settings did not rebuild.

**Run 3** (cold resume: a new adapter process, as after the idle close):

1. `session/new` with an `append` naming ORCHID-7 and `Bearer probe-token`. Claude called the tool and picked `4827`.
2. The adapter was killed and a new one started.
3. `session/resume` with the LILAC-3 `append` and `Bearer probe-token-2`.
4. The reply was **`ORCHID-7, ECHO-MARKER:two, 4827`**. The probe server's log:

```text
00:44:48 POST Bearer probe-token   server/discover … tools/list
00:45:05 POST Bearer probe-token   tools/call
00:45:13 POST Bearer probe-token-2 server/discover … tools/list
00:45:25 POST Bearer probe-token-2 tools/call
```

**Why.** The session transcript (`~\.claude\projects\<cwd>\<session>.jsonl`) holds the appended text as it was when the session was created, and it never contains the changed one. Claude Code restores the system prompt from the transcript on resume.

**Findings:**

- A resume, warm or cold, keeps the conversation.
- A resume ignores a changed `append`. The instructions a Claude session was created with are its instructions for good.
- A resume reads `mcpServers` again, so **a new adapter's new token is used**. §13.7's grant-per-adapter design holds.
- The same applies to conversations started before Milestone 2: they have no `append` at all, and a resume cannot add one.

**Changes:** §13.8 ("What a Claude session keeps", "When instructions change"). Changed project instructions, and Shadows' own instructions for a conversation that never had them, now reach Claude as a context block after the person's text in the next turn. Shadows never resumes a session to change instructions. §13.7's line on a change of instructions is amended to match.

## 4. MCP protocol version — Claude Code tries `2026-07-28`, then falls back

Claude Code's first request to the server:

```text
POST /mcp  MCP-Protocol-Version: 2026-07-28  Mcp-Method: server/discover
{"jsonrpc":"2.0","id":"server-discover-probe-1","method":"server/discover","params":{"_meta":{
  "io.modelcontextprotocol/protocolVersion":"2026-07-28",
  "io.modelcontextprotocol/clientInfo":{"name":"claude-code","version":"2.1.281",…},
  "io.modelcontextprotocol/clientCapabilities":{"roots":{"listChanged":true},"elicitation":{"form":{},"url":{}}}}}}
```

The probe server answered `-32601` (method not found). Claude Code then fell back to the older lifecycle:

```text
POST /mcp  {"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{"roots":{"listChanged":true},"elicitation":{}},…},"id":0}
POST /mcp  MCP-Protocol-Version: 2025-11-25  notifications/initialized
GET  /mcp  MCP-Protocol-Version: 2025-11-25  Accept: text/event-stream   → probe answered 405 Allow: POST; the session carried on
POST /mcp  MCP-Protocol-Version: 2025-11-25  tools/list, tools/call
```

No request carried `Mcp-Session-Id`.

`rmcp` 3.4.1:

- `ProtocolVersion::KNOWN_VERSIONS` is `2024-11-05`, `2025-03-26`, `2025-06-18`, `2025-11-25`, `2026-07-28`, and `LATEST` is `2025-11-25`.
- `ServerHandler::discover` answers `server/discover`. `StreamableHttpServerConfig::legacy_session_mode` reads: "sessions are removed from the `2026-07-28` version, so requests negotiating that version are always served statelessly"; with `false`, the legacy fallback is stateless too.
- Session support is a separate feature, `transport-streamable-http-server-session`. **Corrected 2026-09-25 (B6):** `transport-streamable-http-server` enables it itself (rmcp 3.4.1 `Cargo.toml`), so it cannot be left out; the server is stateless by its config (`legacy_session_mode: false`, `NeverSessionManager`).
- The `server` feature pulls in `schemars` 1.0, from which `rmcp`'s `#[tool]` macros derive tool input schemas.
- `allowed_hosts` defaults to loopback only; `allowed_origins` is empty (no Origin check) by default.

**Changes:** §13.6 names both versions. Plan B6 configures `legacy_session_mode: false` and `json_response: true`, derives tool schemas with `schemars`, and leaves Origin to the daemon's existing guard.

## 5. A 401 is a failed connection, not OAuth

A second probe server answered every request with `401`, an empty body and no `WWW-Authenticate` header, which is what §13.6 has Shadows send. Claude Code sent two `POST /mcp` requests, both with the configured bearer. It made no other request: no `/.well-known/oauth-protected-resource`, no `/.well-known/oauth-authorization-server`.

Asked "Do you have any tool whose name starts with mcp__shadows?", Claude answered:

> No. The `shadows` MCP server failed to connect (401 auth error), so no `mcp__shadows` tools are available in this session.

The session opened normally, with the server's tools missing. The probe advertised no ACP `elicitation.url` capability, so the adapter's MCP OAuth bridge (`startMcpAuthentication`) stayed off; Shadows advertises none either.

This was observed when a session opens. What Claude Code shows when a token is revoked during a session is still acceptance step 6.
