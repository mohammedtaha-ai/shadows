# The ACP adapter run with an isolated AYLUL profile

Recorded 2026-10-19, Asia/Aden. Ad hoc probe on Mohammed's Windows laptop.
Evidence, not design authority. Follows
[the isolated-home probe](./2026-10-19-claude-code-isolated-home.md), which used
the `claude` CLI directly; this one drives the pinned adapter, the path the
daemon uses. A throwaway Node script (not committed) played the ACP client.

## Setup

- Adapter `@agentclientprotocol/claude-agent-acp` 0.81.1 at
  `harness/claude/node_modules/.../dist/index.js`, run as `node <adapter>`.
- Claude Code 2.1.303 (`C:\Users\Mohammed\.local\bin\claude.exe`), Node 24.15.
- Child environment, matching what `shadows-process` gives a Windows child
  (`PATH` plus `SystemRoot`, `SystemDrive`, `ComSpec`, `PATHEXT`, `TEMP`, `TMP`,
  `USERPROFILE`, `APPDATA`; `HOME` and `LOCALAPPDATA` not set), plus
  `CLAUDE_CODE_EXECUTABLE`, `CLAUDE_CONFIG_DIR=<home>`,
  `ANTHROPIC_BASE_URL=https://api.minimax.io/anthropic`,
  `ANTHROPIC_AUTH_TOKEN` (read from `MINIMAX_API_KEY` in Hermes' `.env`, never
  printed or stored), `ANTHROPIC_API_KEY=` (empty), `ANTHROPIC_MODEL=MiniMax-M3`.
  The first and second runs below used a slightly larger environment (also
  `HOME`, `LOCALAPPDATA`); the third repeat used the exact set above.
- cwd = the profile home, whose `CLAUDE.md` says: Your name is AYLUL. When asked
  your name, answer "AYLUL".
- Client: `initialize`, `session/new` with
  `_meta.claudeCode.options.settingSources = ["user"]` (and no `tools`,
  `systemPrompt` or MCP server), one `session/prompt`:
  "Two lines: your name; any AWS instruction or NONE".
  The person's `~/.claude/CLAUDE.md` contains an AWS instruction, so "NONE" means
  it did not load.

## Results

| Check | Result |
| --- | --- |
| Adapter honours `CLAUDE_CONFIG_DIR` | Yes. `.claude.json`, `backups/`, `sessions/` and `projects/<cwd slug>/<sessionId>.jsonl` appeared inside the home; no probe path appears in `~/.claude.json` or under `~/.claude/projects` |
| Process-env key accepted | Yes. `session/prompt` ended `end_turn`; MiniMax answered; no sign-in prompt |
| Home outside `C:\Users\Mohammed` (`E:\`) | Reply `AYLUL` / `NONE` |
| Home under `C:\Users\Mohammed` (`%TEMP%`), `settingSources ["user"]` | Reply `AYLUL` / `NONE`: no parent `CLAUDE.md` leak |
| Same home kind, `settingSources` not sent (contrast) | Reply `AYLUL` / "Prefer the AWS MCP Server for AWS interactions over direct CLI calls.": the leak is real and `["user"]` removes it through the adapter |
| Shadows' exact Windows child environment | Reply `AYLUL` / `NONE`: nothing the daemon withholds was needed |
| Permission requests | None raised for a text-only turn |
| Streaming | Updates seen: `available_commands_update`, `usage_update`, `agent_thought_chunk`, `agent_message_chunk` |
| Session opening | `session/new` took about 1.3 s |
| `~/.claude.json` mtime | **Not attributable.** 14:20:54 before the first run, 14:23:50 after the third and 14:26:50 after the last: it moved, but Mohammed's own Claude Code session runs in parallel and writes that file. No leak is claimed. What was checked instead: the probe's `.claude.json` and transcripts exist inside each probe home, the string `aylul-acp-probe` occurs nowhere in `~/.claude.json`, and no probe-named folder exists under `~/.claude/projects` |

## What the adapter offers (`configOptions` of `session/new`)

With `ANTHROPIC_MODEL=MiniMax-M3` the adapter returned three options:

- `mode` (category `mode`), current `default`: `default`, `acceptEdits`, `plan`,
  `auto`, `bypassPermissions`.
- `model` (category `model`), current `MiniMax-M3`: `default`, `opus`, `fable`,
  `sonnet`, `haiku`, `MiniMax-M3`. The adapter lists its own Anthropic aliases
  and adds the configured model; the aliases are not MiniMax models.
- `effort` (category `thought_level`), current `default`: `default`, `low`,
  `medium`, `high`, `xhigh`, `max`. Offered even though the provider is
  MiniMax; whether MiniMax honours it was not measured.

With `ANTHROPIC_MODEL=MiniMax-M3[1m]` the current model and its option value
become `MiniMax-M3[1m]` (the id the picker would send back).

## Context window (Step 2b)

Claude Code does not know `MiniMax-M3`. It logs
`[claude-code:unrecognized_model] {"model":"MiniMax-M3",...}` on every run, with
or without the settings below; that log line is not the user-facing notice.
The notice text (read from the 2.1.303 binary) is: Claude Code doesn't recognize
the model, so it assumes a context window of N tokens; its hints are to append
`[1m]` to the model name, or set `CLAUDE_CODE_MAX_CONTEXT_TOKENS` to the real
window. By the code, the notice is not produced when
`CLAUDE_CODE_MAX_CONTEXT_TOKENS` is a positive number for a custom model id.
Neither the notice itself nor its disappearance could be seen through the
adapter or `claude -p`; what was measured is the window the adapter reports in
`usage_update.size`:

| Environment | `usage_update.size` over the turn |
| --- | --- |
| nothing extra | 200000, 200000 |
| `CLAUDE_CODE_MAX_CONTEXT_TOKENS=1000000` | 200000, 200000, 1000000 (the first updates precede the reply) |
| plus `CLAUDE_CODE_AUTO_COMPACT_WINDOW=1000000` | same as the line above |
| `ANTHROPIC_MODEL=MiniMax-M3[1m]` | 1000000 throughout; MiniMax accepted the request with the suffix |

Recommended for Task 4's config `env`, keeping the model id clean:
`CLAUDE_CODE_MAX_CONTEXT_TOKENS=1000000` and
`CLAUDE_CODE_AUTO_COMPACT_WINDOW=1000000`, the pair Mohammed's own launcher
uses. The 1,000,000 value comes from that launcher; MiniMax-M3's real window
was not checked against MiniMax documentation. Using the `[1m]` suffix instead
also works but changes the model option id to `MiniMax-M3[1m]`.

## Limits and leftovers

- Text-only turns: tool use through the adapter, permission prompts, resume and
  changing model or effort with `session/set_config_option` were not exercised.
- The reported cost uses Anthropic prices, not MiniMax billing (as in the
  earlier probe).
- Probe folders under `%TEMP%` were deleted. The control home
  `E:\aylul-acp-probe\` (no key in it by construction; it holds one
  transcript and the profile's `.claude.json`) could not be deleted by the
  probe agent because a safety check refused the removal; delete it by hand.
