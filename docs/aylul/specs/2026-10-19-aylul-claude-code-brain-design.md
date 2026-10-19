# AYLUL brain: Claude Code in its own home, run by Shadows core

Date: 2026-10-19.
Status: **Design for review. Nothing implemented.** Decided in conversation
with Mohammed on 2026-10-19; probe facts are in
[the isolated-home evidence](../../evidence/aylul/2026-10-19-claude-code-isolated-home.md).

This file owns AYLUL's brain and where it runs. It replaces Hermes as AYLUL's
local brain. It does not change the [vision](../vision.md), the hook/sidecar
owner, or the character owner. Later projects (work monitoring, personal
profile, voice, remote alerts) build on this one and get their own designs.

## 1. Intent and why

AYLUL is Mohammed's assistant that stands between him and his agents (Claude,
Codex, Gemini) and later handles personal tasks. Today its brain is Hermes,
reached through `services/assistant` and the product daemon. That path shows
the same problems the first Shadows UI had:

- The screen looks stuck after a message: the reply arrives only when the
  Hermes run completes (`getRun` polling, `POLL_MS = 1500`), with no streamed
  text or tool progress.
- No model or effort choice: the local protocol has seven actions and no
  settings (`services/assistant/src/protocol.ts`).

Shadows core already streams a turn's events to its web client and offers the
harness's models, efforts and modes (`shadows-agent` `SessionChoices`). So
AYLUL reuses Shadows core, its web client and its settings, with Claude Code as
the brain. The only new presentation work left is the character.

The brain must stay independent: if Mohammed's own Claude subscription hits
its limit, AYLUL keeps working on its own provider and key.

## 2. Goals

1. AYLUL conversations run Claude Code through the existing Shadows harness
   (ACP adapter), streamed to the Shadows web client like Planner turns.
2. AYLUL has its own home folder, like `%LOCALAPPDATA%\hermes`, holding its
   settings, sessions, memory and profiles. Proposed root:
   `%LOCALAPPDATA%\aylul\` (builder may adjust the name, not the idea).
3. Each profile is one Claude Code home (`CLAUDE_CONFIG_DIR`) with its own
   `CLAUDE.md` (persona), `settings.json` (provider URL, models, permissions)
   and its own provider key.
4. Model and effort are chosen in the UI from what the harness offers.
5. AYLUL works from its home, never from a project folder.

## 3. Non-goals for this project

- Watching Claude/Codex/Gemini sessions, explaining them, alerts (project 2).
- Personal profile: email, YouTube, routines (project 3).
- Voice (project 4). Remote/phone alerts (later).
- A provider proxy. First version passes a provider URL and key directly.
- Server deployment of AYLUL's brain.

## 4. Anti-goals (a failure even if it works)

- AYLUL touches Mohammed's `C:\Users\Mohammed\.claude` or `.claude.json`: his
  sign-in, settings, hooks, skills, agents, memory or session history.
- AYLUL loads a project's `CLAUDE.md`, agents, hooks or `.mcp.json`.
- AYLUL uses Mohammed's Claude subscription as its brain.
- A provider key appears in a file the renderer can read, in logs, or in Git.
- The screen waits for a whole reply with nothing streamed.

## 5. Constraints (measured, see evidence)

- `CLAUDE_CONFIG_DIR=<profile home>` isolates sign-in, `.claude.json`,
  sessions, memory, skills, agents, plugins and hooks.
- The provider key must reach Claude Code as a **process environment**
  variable (`ANTHROPIC_AUTH_TOKEN`). In `settings.json` `env` it is ignored for
  sign-in; `apiKeyHelper` is rejected by MiniMax.
- Base URL and model names may live in the profile's `settings.json` `env`.
- A work folder under `C:\Users\Mohammed` loads
  `C:\Users\Mohammed\.claude\CLAUDE.md` as a parent project file. Loading only
  user settings (`settingSources: ["user"]`, CLI `--setting-sources user`)
  removes the leak and keeps the profile's own `CLAUDE.md`. AYLUL sessions use
  exactly that.
- `shadows-process` already starts children with a cleared environment plus an
  explicit list (`env_clear`, `shadows-process/src/lib.rs`), and
  `shadows-agent/src/claude.rs` says an isolation profile would replace its
  inherited `HOME`-based values.
- Planner sessions keep their current behaviour (`settingSources: []`,
  Mohammed's sign-in); this design does not change them.
- Claude Code does not recognize `MiniMax-M3`; the context window must be set
  for custom model ids or it assumes 200k.

## 6. Decided by Mohammed

- Shadows becomes part of the product as its own section; Mohammed and Codex
  have begun that.
- AYLUL uses the same Shadows core, the same web client and its settings.
- AYLUL lives inside the same product daemon and the same Shadows database, as
  its own section. Only its brain homes (profiles) live under the AYLUL root.
- AYLUL runs alone with its own settings; Shadows keeps running agents inside
  project folders as it does today.
- AYLUL does not enter projects. It learns about work through hooks and, when
  asked, reads what it needs (project 2).
- Order of projects: brain (this), work profile, personal profile, voice.
  Remote alerts later.

## 7. My calls (Claude), for review

- **The profile home is the session's working folder.** That gives AYLUL "its
  own place" without a project, and keeps project files out by construction.
- **Profiles stored under the AYLUL root**, one folder each, e.g.
  `%LOCALAPPDATA%\aylul\profiles\<name>\` as the `CLAUDE_CONFIG_DIR`.
- **Keys kept by the product daemon** (the Rust daemon in `crates/shadows`,
  not `services/assistant`), never in the profile's `settings.json`, and
  injected into the adapter's environment at spawn. Storage mechanism is the
  builder's choice within the planned `secrets` owner's rules (config holds
  references only).
- **First profile:** "AYLUL" on MiniMax-M3 through
  `https://api.minimax.io/anthropic`, the provider Mohammed already uses.
- **The Hermes path is left in place, unused by default**, until this path is
  accepted; removing it is a separate decision.
- **Profiles in v1 are seeded, not edited in the UI.** On first run the
  product creates the "AYLUL" profile from a template kept in the repository
  (`CLAUDE.md`, `settings.json`). The key comes from a named environment
  variable of the daemon (for MiniMax, `MINIMAX_API_KEY`); configuration holds
  only that name. A UI to create profiles and enter keys is a later slice.
- **AYLUL's remembered model and effort are kept apart from the Planner's.**
  Choosing a model in AYLUL never changes the Planner's remembered defaults.
- **The picker shows what the adapter offers.** The profile names the real
  provider models it wants offered; effort is shown when the adapter offers it.
  Acceptance needs the chosen value to reach the session, not a visible
  quality difference.
- **Permissions:** the profile's `settings.json` denies Read, Edit and Write on
  `C:\Users\Mohammed\.claude` and `.claude.json`; other permission requests go
  through the web client's existing prompt, as Planner turns do. A full
  sandbox around Bash is out of scope; deny rules do not fence shell commands.
- **First task is a probe through the real adapter.** Every measurement so far
  used the `claude` CLI directly. The ACP adapter (SDK, Node) must be shown to
  honour `CLAUDE_CONFIG_DIR`, `settingSources: ["user"]` and the process-env
  key before anything else is built.

## 8. Left to the builder

- How AppCore represents an AYLUL conversation: a thread kind, a separate
  service, or a project rooted at the profile home. It must not receive the
  Planner's appended prompt, Planner tools, stage text or MCP grant.
- Where the profile/env choice enters `ClaudeAdapter::process_spec`.
- How the desktop main window shows the Shadows web client and where the
  character window keeps working unchanged.
- How the custom model's context window is declared.

## 9. Acceptance

- One AYLUL conversation in the Shadows web client, MiniMax-M3, text streams
  while the reply is written; model and effort changed from the UI.
- After it: `C:\Users\Mohammed\.claude` and `.claude.json` unchanged by AYLUL;
  the conversation's transcript is inside the profile home.
- Asked about AWS guidance or project slugs, AYLUL has none (no leak), and
  answers with its own persona.
- Mohammed's own Claude sessions and the Shadows Planner behave as before.
- Run by a person in the real app, logs read, before it is called done.
