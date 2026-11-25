# §24.3 probe: delivering the project switches without the settings they ride on — 2026-11-25

Answers the §24.3 OPEN "how each switch is delivered without the settings it rides on"
(`docs/core/specs/2026-10-08-approval-authority-design.md`). A measurement, not design
authority.

Setup: a Linux cloud container, Claude Code 2.1.332 through the pinned `claude-agent-acp`
0.101.0 (`harness/claude/package.json`; 0.112.0 is the latest published). No Shadows code
ran: a throwaway ACP client (Node, newline-delimited JSON-RPC: `initialize`, `session/new`,
one `session/prompt`, every permission request cancelled) opened a session with the
Planner's options as `crates/core-harness/src/setup.rs` sends them at `e7b9332`:
`tools` = `PLANNER_TOOLS` (Read, Grep, Glob, WebSearch, WebFetch, ToolSearch, Agent,
TodoWrite, Bash), `settingSources: []`, `strictMcpConfig: true`, plus the option under
test. The client and its fixtures were not committed.

Planted in the session's project folder: `CLAUDE.md` with a code word;
`.claude/settings.json` with a `SessionStart` and a `UserPromptSubmit` hook, each touching a
file; `.claude/skills/marker-skill/SKILL.md` with a second word. Outside it: a local plugin
with one skill (`skills/plugin-skill`) and a `SessionStart` hook (`hooks/hooks.json`); a copy
of that plugin holding `skills/` only; a stdio MCP server with one tool, `kestrel_word`.
Each session was asked for the code word, its skill list, its MCP tools (ToolSearch
"kestrel") and its built-in tools.

## Runs

| Run | Option under test | Code word | Skills seen | MCP tool | Hooks run |
|---|---|---|---|---|---|
| R0, control | plain `claude -p` in the folder | read (via the skill) | marker-skill and the account's own | — | project SessionStart, UserPromptSubmit |
| R1 | none (Shadows today) | none | none | none | none |
| R2 | `skills: ["marker-skill"]` | none | none | none | none |
| R3 | `plugins: [plugin with hook]` | none | none | none | **plugin SessionStart** |
| R4 | `mcpServers: [probe]` on `session/new` | none | none | `mcp__probe__kestrel_word` | none |
| R5 | R2 with `Skill` added to `tools` | none | none | none | none |
| R6 | `Skill` in `tools`, `plugins: [skills-only copy]`, `skills: "all"` | none | `shadows-skills:plugin-skill` and Claude Code's bundled skills | none | none |
| R7 | as R6, with the plugin that has a hook | none | `probe-plugin:plugin-skill` and the bundled skills | none | **plugin SessionStart** |
| R8 | as R6, `skills: ["shadows-skills:plugin-skill"]` | none | `shadows-skills:plugin-skill` only | none | none |

R1 to R8 kept the built-in tools to the list sent (no Write, Edit or PowerShell).

## Answers

- **`CLAUDE.md`:** with `settingSources: []` the project's `CLAUDE.md` never reaches the
  session (R1 to R8), and loading it through `settingSources: ["project"]` brings the
  project's hooks with it (R0). Shadows reads the file itself and appends it to the
  Planner's instructions.
- **MCP connectors:** a server passed in `session/new`'s `mcpServers` reaches the session
  under `strictMcpConfig: true`, beside Shadows' own, and nothing else does (R4).
- **Skills:** with `settingSources: []` no skill folder is discovered, so `skills` alone
  shows nothing (R2, R5). A local plugin delivers its skills once `Skill` is in `tools`
  (R6). A plugin runs its own hooks (R3, R7), so Shadows passes a plugin folder it builds
  holding only the chosen skills' folders. `skills: "all"` also lists Claude Code's bundled
  skills; naming the chosen ones (`<plugin>:<skill>`) shows only them (R8).

## Not measured

- Codex: not installed in the container.
- The Planner's subagents, Windows, and a skill's own scripts run through `Bash`.
- The pinned adapter 0.101.0 only; the latest is 0.112.0.
