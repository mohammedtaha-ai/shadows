# ACP adapter probe — Milestone 1 Task 0

- **Date:** 2026-09-24
- **Adapter:** `@agentclientprotocol/claude-agent-acp` 0.81.1 (`agentInfo.version`), installed with `--omit=optional`
- **Claude Code:** 2.1.281, passed as `CLAUDE_CODE_EXECUTABLE=%USERPROFILE%\.local\bin\claude.exe`
- **Node:** 24.15.0 · **OS:** Windows 11
- **Probe:** a throwaway raw JSON-RPC client (newline-delimited JSON over the adapter's stdio, no SDK), run from a scratch directory outside the repository and not committed. Every line in both directions was logged; the lines below are quoted from those logs, trimmed.
- **Cost:** 9 prompts. The weekly limit read 0.98 during the run, so checks 3–8 ran on `sonnet`.

Answers spec §12.3's box, items 1–8.

## 1. Choices come back from `session/new`

`initialize` advertises `sessionCapabilities: { additionalDirectories, close, delete, fork, list, resume, subagents }` and `loadSession: true`.

`session/new` returns four config options:

| id | category | current | values |
|---|---|---|---|
| `mode` | `mode` | `auto` | `default` (named "Manual"), `acceptEdits`, `plan`, `auto`, `bypassPermissions` |
| `model` | `model` | `opus` | `default` ("Default (recommended)", description "Opus"), `opus` ("Opus 5.5"), `claude-fable-5-1[1m]` ("Fable 5.1"), `sonnet` ("Sonnet 5"), `haiku` ("Haiku 4.5") |
| `effort` | `thought_level` | `high` | `default`, `low`, `medium`, `high`, `xhigh`, `max` |
| `fast` | `model_config` | `off` | `on`, `off` |

Switching the model with `session/set_config_option` answers the complete set again:

| model set | effort option | effort current | mode current after the switch | `fast` option |
|---|---|---|---|---|
| `default` | default…max | high | auto | present |
| `opus` | default…max | high | auto | present |
| `claude-fable-5-1[1m]` | — | — | — | refused: `{"code":-32603,"data":{"details":"API error: 429 Usage credits are required for this model · model not changed"}}` |
| `sonnet` | default…max | **medium** | auto | absent |
| `haiku` | **absent** | — | **acceptEdits** | absent |

Findings:

- The model list is live: it names Opus 5.5 and Fable 5.1, neither of which Shadows has ever listed.
- **A listed model can be unusable by the account.** Fable is listed; selecting it is refused by the harness with a message, and the session keeps its model.
- **A model can offer no effort at all** (`haiku`): the `thought_level` option disappears.
- **`auto` is not withheld from the mode list.** It stays listed for `haiku`, but the adapter moves the current mode from `auto` to `acceptEdits` when a model without auto mode is chosen.
- A new session starts in the mode of the person's Claude settings (`auto` here), not in a mode Shadows chose.

## 2. A turn's updates

Prompt `say ok` on Opus, 8.8 s. Updates: `available_commands_update` ×2, `usage_update` ×3, `agent_message_chunk` ×1, `session_info_update` ×1.

```text
<< {"sessionUpdate":"usage_update","used":45196,"size":200000,"_meta":{"_claude/model":"claude-opus-5-5"}}
<< {"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"ok"},"messageId":"msg_011CfMUQpGbR6aCpM3kyQLkU"}
<< {"sessionUpdate":"usage_update","used":45196,"size":200000,"_meta":{"_claude/rateLimit":{…},"_claude/model":"claude-opus-5-5"}}
<< {"sessionUpdate":"usage_update","used":45196,"size":1000000,"cost":{"amount":0.361608,"currency":"USD"},"_meta":{"_claude/origin":{"kind":"human"},"_claude/model":"claude-opus-5-5"}}
<< {"id":5,"result":{"stopReason":"end_turn","usage":{"inputTokens":2,"outputTokens":4,"cachedReadTokens":0,"cachedWriteTokens":45190,"totalTokens":45196},"_meta":{"quota":{"model_usage":[{"model":"claude-opus-5-5",…}]}}}}
```

- Agent message chunks carry `messageId`.
- The prompt answers `stopReason: "end_turn"`, with end-turn `usage` and the per-model usage under `_meta.quota`.
- A new session's first turn wrote 45k tokens of cache ($0.36 on Opus) for a two-letter answer.

## 3. Cancel

Prompt `count slowly from 1 to 200…` on Sonnet; `session/cancel` sent after the first chunk. The prompt answered `stopReason: "cancelled"` **59 ms** after the cancel.

## 4. Resume after the adapter was killed

Session A: `remember the word amber. reply only: ok` → `end_turn`. The adapter's process tree was killed (`taskkill /T /F`), a new adapter started, `initialize`, `session/resume { sessionId: A, cwd, mcpServers: [] }` answered in **2.8 s**, then `what word did I ask you to remember?` → **`amber`**.

The resumed session answered `model.currentValue = "opus"` and `mode.currentValue = "auto"`, although A had been set to `sonnet`: **a resumed session starts from the person's defaults, not from the settings it last ran with.**

## 5. The context breakdown

`available_commands_update` lists every Claude Code command, including `context`. Sending the prompt `/context` answers one agent message in markdown — model, `43.1k / 1m (4%)`, and a table "Estimated usage by category": System prompt, System tools, MCP tools (deferred), System tools (deferred), Custom agents, Memory files, Skills, Messages, Free space, Autocompact buffer — followed by per-tool, per-agent and per-skill tables.

| call | time | `usage_update`s |
|---|---|---|
| first `/context` in a fresh session | 16.8 s (22.9 s in another run) | 0 |
| second `/context` in the same session | **0.6 s** | 0 |

- It costs no model tokens (no usage report).
- The slow first call matches the adapter's own note that the SDK's context request stalls before a session's first turn.
- It is a prompt on the session, so it cannot run while a turn runs.
- Not established: whether a `/context` exchange is written into the session transcript that `session/resume` replays to the model.

## 6. Fork

`session/fork { sessionId: A }` answered a new id in 20 ms. **Prompting that id at once failed**: `{"code":-32603,"data":{"details":"Session not found"}}`.

`session/fork`, then `session/resume` on the returned id, then a prompt: the fork answered **`amber`**. `session/resume` on A then `say ok` → `end_turn`, `ok`. The source is unaffected.

## 7. Usage, the answering model, rate limits

- The answering model is in `_meta["_claude/model"]` on every `usage_update` (`claude-opus-5-5`, `claude-sonnet-5`).
- `size` is first reported as `200000` and corrected to `1000000` after the turn's result. Only the last report of a turn is right.
- The rate-limit report is in `_meta["_claude/rateLimit"]`:

```json
{"status":"allowed_warning","resetsAt":1790542800,"rateLimitType":"seven_day","utilization":0.98,
 "unifiedWindows":{"five_hour":{"utilization":0.35,"resetsAt":1790230200},
                   "seven_day":{"utilization":0.98,"resetsAt":1790542800}}}
```

- `usage_update` also carries `cost` (USD) after a turn.

## 8. Permission requests

Mode set to `acceptEdits` (`current_mode_update` confirmed it).

- `run the shell command: echo probe` → **no request**; the command ran (`Output: probe`). Claude Code treats it as safe without asking.
- `node -e "console.log(41+1)"` → `session/request_permission`:

```json
{"toolCall":{"toolCallId":"toolu_01Dc…","name":"Bash","title":"node -e \"console.log(41+1)\"","kind":"execute",
             "rawInput":{"command":"node -e \"console.log(41+1)\"","description":"Run node command to print 41+1"}},
 "_meta":{"permission":{"title":"node -e \"console.log(41+1)\"","description":"Reason: This command requires approval"}},
 "options":[{"optionId":"allow-once","kind":"allow_once"},
            {"optionId":"allow-with-updates","kind":"allow_always"},
            {"optionId":"reject","kind":"reject_once"}]}
```

Answered with `reject` (`reject_once`). The tool call ended `status: "failed"`; Claude replied "Command blocked (permission denied). Can't show output." and the turn ended `end_turn`.

- A tool call's first `tool_call` update is titled generically (`"Terminal"`); the real title (the command) arrives in later `tool_call_update`s, and the final one carries `status` `completed` or `failed`.

## Also seen

- The adapter writes timing diagnostics to stderr (`[session/create] … phase=sdk-initialize durationMs=2669`). A session is ready about 2.8 s after `session/new`.
- `session_info_update` carries a title Claude gave the session.
