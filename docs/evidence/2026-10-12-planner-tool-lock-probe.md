# §24.3 probe: what the adapter honours — 2026-10-12

Answers the §24.3 OPEN "what the adapter actually honours". A measurement,
not design authority. The probe's code is commit `d8d3465` (reverted in the
next commit); it changed only `SessionSetup::meta` in
`crates/shadows-agent/src/acp.rs`.

Setup: the person's own daemon (port 4318, `--debug`), project "Book
Catalogue", Claude Code 2.1.292 through `claude-agent-acp` 0.81.1,
Opus 5.5 Medium. Shadows' policy offers two modes, `acceptEdits` and `auto`
(`crates/shadows-agent/src/policy.rs`); bypass is never offered.

## Adapter reading

`claude-agent-acp` 0.81.1 `dist/acp-agent.js`, `newSession` (around
line 6230): `_meta.claudeCode.options` is spread over the adapter's own
options, so a host sets any SDK option. Three matter here:

- `tools`: the built-in tool set itself. The default is the `claude_code`
  preset. A tool left out does not exist in the session, so no mode,
  bypass included, can run it.
- `settingSources`: default `["user", "project", "local"]`; the host's value
  wins.
- `strictMcpConfig`: only the MCP servers the host passes.

`allowDangerouslySkipPermissions: false` would also remove bypass from the
mode list (line 6241). Not needed while Shadows' policy omits it.

## Before: main at `05b07a5`

Mode `acceptEdits`. The Planner listed, among others: Bash, PowerShell,
Write, Edit, NotebookEdit, Workflow, CronCreate, RemoteTrigger; and MCP
servers from the person's own account: `claude_ai_Claude_Docs`,
`claude_ai_Google_Drive` (including `share_file`, `trash_file`), plus a
failed `aws`. It declined to run `curl` against the daemon by its own
choice, citing Shadows' instructions. Nothing stopped it.

## After: probe at `d8d3465`

Options sent: `tools: [Read, Grep, Glob, WebSearch, WebFetch, ToolSearch,
Agent, TodoWrite]`, `strictMcpConfig: true`, `settingSources: []`, with the
existing `allowedTools: [mcp__shadows__*]`.

| Turn | Recorded mode | Result |
|---|---|---|
| 1 | `auto` | Tools: Agent, Glob, Grep, Read, ToolSearch; deferred WebFetch, WebSearch and the 14 `shadows` MCP tools. ToolSearch found no Bash, PowerShell, Write or Edit. No Google Drive, Docs or aws. |
| 1, subagent | (inherits) | The subagent (`general-purpose`) listed Agent, Glob, Grep, Read, ToolSearch and its hand-back tool; no Bash, PowerShell, Edit, Write or NotebookEdit. |
| 2 | `acceptEdits` | ToolSearch found no Bash, PowerShell, Write, Edit or NotebookEdit. |

No `probe.txt` was created in the project folder. Mode is read from the
operations route (`invocation.requested_mode`).

## Answers

- (a) The fixed tool list holds in both modes Shadows offers, and in the
  Planner's subagents: the tools are absent, not refused.
- (b) Yes: without these options a session loads the person's own claude.ai
  MCP connectors and settings. `strictMcpConfig` and `settingSources: []`
  keep them out, hooks, skills and the project's `CLAUDE.md` included
  (next section).

## Settings: hooks, skills, `CLAUDE.md`

Run after Codex's review of the spec, on the same probe build. Planted in the
project folder: `CLAUDE.md` with a marker word; `.claude/settings.json` with
a `SessionStart` and a `UserPromptSubmit` hook, each writing a file;
`.claude/skills/marker-skill/SKILL.md` with a second marker.

| Run | Marker word | Marker skill | Hook files |
|---|---|---|---|
| Plain `claude -p` in that folder (control) | read | listed | both written |
| A new Shadows Planner conversation | absent | absent | none written |

The Planner named no skill or plugin of the person's (their `superpowers`
plugin included); it saw only Claude Code's built-in commands and agent
types. The planted files were removed afterwards.

## Not measured

- A `bypassPermissions` turn: Shadows never offers it, so it was not run.
- The adapter logs `CLAUDE_SDK_CAN_USE_TOOL_SHADOWED` for
  `mcp__shadows__*`: the bare `allowedTools` entry approves Shadows' tools
  before the permission callback. Expected; Shadows' tools are the ones the
  Planner may use.
