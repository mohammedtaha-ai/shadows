# AYLUL brain: Claude Code in its own home, run by Shadows core

Date: 2026-10-19.
Status: **Design for review. Nothing implemented.** Decided in conversation
with Mohammed on 2026-10-19; probe facts are in
the isolated-home evidence (`docs/evidence/aylul/2026-10-19-claude-code-isolated-home.md` at
`0187365`).

This file owns AYLUL's brain and where it runs. It replaces Hermes as AYLUL's
local brain. It does not change the [vision](../vision.md), the hook/sidecar
owner, or the character owner. Later projects (work monitoring, personal
profile, voice, remote alerts) build on this one and get their own designs.

## 1. Intent and why

AYLUL is Mohammed's assistant that stands between him and his agents (Claude,
Codex, Gemini) and later handles personal tasks. Its brain was Hermes, reached
through `services/assistant` and the product daemon; both were removed on
2026-10-29. That path showed the same problems the first Shadows UI had:

- The screen looks stuck after a message: the reply arrives only when the
  Hermes run completes (`getRun` polling, `POLL_MS = 1500`), with no streamed
  text or tool progress.
- No model or effort choice: the local protocol had seven actions and no
  settings (`services/assistant/src/protocol.ts`).

Shadows core already streams a turn's events to its web client and offers the
harness's models, efforts and modes (`shadows-agent` `SessionChoices`). So
AYLUL reuses Shadows core, its web client and its settings, with Claude Code as
the brain. The remaining desktop packaging and character-to-agent connection
are owned by [the desktop and companion design](2026-10-20-aylul-desktop-web-companion-design.md).

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
- **Keys kept by Core** (the `secrets` service `shadows serve` runs),
  never in the profile's `settings.json`, and
  injected into the adapter's environment at spawn. Storage mechanism is the
  builder's choice within the planned `secrets` owner's rules (config holds
  references only).
- **First profile:** "AYLUL" on MiniMax-M3 through
  `https://api.minimax.io/anthropic`, the provider Mohammed already uses.
- **The Hermes path was left in place, unused by default**, until this path was
  accepted; Mohammed decided its removal, done on 2026-10-29.
- **Profiles in v1 are seeded, not edited in the UI.** On first run the
  product creates the "AYLUL" profile from a template kept in the repository
  (`CLAUDE.md`, `settings.json`). The key comes from a named environment
  variable of the daemon (for MiniMax, `MINIMAX_API_KEY`); configuration holds
  only that name. A UI to create profiles and enter keys is a later slice.
- **Both commands that build the core run AYLUL** (amended 2026-10-19).
  `shadows serve`, which Mohammed opens daily with the web client, takes the
  section as `--aylul-config <file>`; `shadows daemon` takes it as the `aylul`
  key of its planner configuration. Both read the same section shape and run
  the same seeding and project setup. Without it, neither shows AYLUL.
- **The profile's environment comes from an env file today, from the UI later**
  (Mohammed, 2026-10-19). The section names an `env_file` (dotenv; today
  `E:\Globalprojects\shadows\.env`, git-ignored) that the product reads at each
  AYLUL spawn: every pair goes into the session's environment, and `key_env`
  names the pair that becomes `ANTHROPIC_AUTH_TOKEN`. No Windows variable is
  needed. **Next slice, decided:** these variables are added and edited from the
  Shadows UI, like Claude Code desktop's environment settings, and stored
  **encrypted in the Shadows database** under the planned `secrets` owner; the
  env file then becomes an import path, not the store.
- **AYLUL's remembered model and effort are kept apart from the Planner's.**
  Choosing a model in AYLUL never changes the Planner's remembered defaults.
- **The picker shows what the adapter offers.** The profile names the real
  provider models it wants offered; effort is shown when the adapter offers it.
  Acceptance needs the chosen value to reach the session, not a visible
  quality difference.
- **Permissions:** the profile's `settings.json` denies Read, Edit and Write on
  `C:\Users\Mohammed\.claude` and `.claude.json`. Other permission requests
  are put to Mohammed
  ([AYLUL agents and panel §7.8](2026-11-01-aylul-agents-and-panel-design.md));
  Planner turns refuse them at once (harness controls §12.2).
  A full sandbox around Bash is out of scope; deny rules do not fence shell
  commands.
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

## 10. Environment variables from the UI (slice 2, decided 2026-10-19)

Mohammed adds, replaces and removes AYLUL's environment variables from the
Shadows web client, like Claude Code desktop's environment settings. The
values are stored encrypted in the Shadows database. The env file of §7 stops
being read at spawn and becomes a one-time import. Amended 2026-10-19 after a
Codex review of the plan.

**Decided by Mohammed:** variables come from the UI; values are stored
encrypted in the database; no Windows environment variable.

**My calls (Claude):**

- **Owner: a `secrets` service in `shadows-core`,** the eleventh, one folder
  like the others (amends the service list of
  [application core §14.4](../../superpowers/specs/2026-09-26-application-core-design.md)).
  It is the planned `secrets` owner of
  [architecture §1.3](../../superpowers/specs/2026-09-21-architecture-design.md),
  built as a core service because its store is SQLite and its first caller is
  the harness. Its public methods take a `command_id` and return `CoreError`,
  like every service. This closes the OPEN "credential backend" of
  [agent profiles §19.5](../../superpowers/specs/2026-10-03-agent-profiles-and-extensions-design.md)
  for AYLUL profiles.
- **Master key:** 32 random bytes in the Windows Credential Manager,
  `keyring` 3.6.3 with the `windows-native` feature (without a platform
  feature, v3 silently uses a mock store), service `shadows`, user
  `secrets-master-key`, stored as a binary secret. Only the binary touches it.
  It is created when absent, then read back, so two daemons starting together
  end on the one stored key. An entry that exists but cannot be read or has the
  wrong length is an error and is never replaced. The binary hands the key to
  `AppCore::start`; the core never calls the credential store. There is no
  plaintext fallback. This is how principle 15 holds: the database keeps only
  ciphertext, and the key that opens it lives outside the database.
- **Two keys come from the master key,** by HKDF-SHA256 with the infos
  `shadows/secrets/aead/v1` and `shadows/secrets/fingerprint/v1`. Neither is
  stored.
- **Encryption:** AES-256-GCM, a fresh 12-byte nonce from the OS RNG on every
  write, and a `format` column (1) on each row. The associated data is
  `profile`, a NUL byte, then `name`, so a row copied under another name fails
  to decrypt. A malformed row (bad nonce length, unknown format) is a decrypt
  error, never a panic.
- **Idempotency:** a replayed `set` with a different value must conflict. The
  fingerprint covers `profile`, `name` and HMAC-SHA256(fingerprint key,
  value). That is never a plain hash, so `command_record` gives no offline
  guessing oracle.
- **A value goes in and never comes out to a reader.** Listing returns names
  and `updated_at` only. No HTTP response, SSE event, durable event, command
  record, log line, error text or `Debug` output carries a value. The routes'
  body-parse errors are replaced with a fixed message, because serde's text can
  quote the input. The only reader is the AYLUL opening.
- **When an edit applies:** at the next AYLUL adapter spawn. An adapter that
  is already running keeps the environment it started with, because Shadows
  reuses live adapters. The UI says "applies to new AYLUL conversations".
- **No MCP tool.** No agent can read or set these through Shadows. This is
  not a sandbox: an AYLUL session can read its own environment with its shell,
  as any process can.
- **Names** are upper case only, `^[A-Z_][A-Z0-9_]{0,127}$`. That avoids two
  rows Windows would treat as one variable. `CLAUDE_CONFIG_DIR`,
  `ANTHROPIC_API_KEY` and `ANTHROPIC_AUTH_TOKEN` are refused, because the
  product sets them. `key_env` must match the same rule. **Values** are
  1..=32768 bytes with no NUL. The UI takes one line (a password input); the
  API accepts newlines.
- **Spawn order is unchanged** (profile `env`, then stored pairs, then the
  home, a blank API key and the token). The stored pairs take the env file's
  old slot. The token is the stored pair named by `key_env`; if there is none,
  it is the daemon's variable of that name.
- **Errors keep their names to the client.** A missing key
  (`AYLUL_KEY_MISSING`), an undecryptable row (`SECRET_UNREADABLE`, which
  names the variable and says to replace it in Settings) and no master key
  (`SECRETS_UNAVAILABLE`) reach the web client as their own codes, not as
  "the harness could not start".
- **Import:** at start, the binary asks whether the profile was ever imported
  (a marker row) before it opens the env file. If not, it parses the file.
  One write transaction then re-checks the marker, stores every valid pair
  (last duplicate wins; invalid names are skipped and counted by name) and
  writes the marker, even for an empty file. A failure stores nothing and no
  marker, is logged without values, and never stops the daemon; the next start
  retries it.
- **Lost master key:** the names still list. Each value must be replaced in
  Settings, because a replace overwrites the old row.
- **Web:** the Settings page shows an "AYLUL environment" section, hidden only
  on `AYLUL_NOT_CONFIGURED`. Other errors are shown. It lists names, lets
  Mohammed replace or remove each, and has an add form. A saved value is never
  shown again.

**Acceptance:**

1. Mohammed adds `API_KEY` from Settings, moves `.env` away, restarts
   `shadows serve`, and AYLUL answers.
2. A byte search of `shadows.db`, `-wal` and `-shm` for the key value finds
   nothing.
3. A replaced value is used by a new AYLUL conversation without a restart.
4. With the key removed and no daemon variable, a new AYLUL conversation
   shows `AYLUL_KEY_MISSING` naming `API_KEY`.

## 11. Agents and providers from the UI (slice 3, decided 2026-10-19)

Mohammed adds **providers** (a URL, a key and their models) and **agents** (an
AYLUL profile on one provider) in the AYLUL workspace of the sidebar. A
conversation is started under its agent, and the model menu shows exactly that
agent's provider models. This replaces §7's single seeded profile and its
config-file provider.

**Decided by Mohammed:**

- Providers, agents and models are entered in the UI and stored in the
  database. Nothing is taken from the env file or `aylul.json` any more.
- Several agents now, not only AYLUL.
- **A provider is separate from the agents.** A provider (URL, key, models) is
  added once, and each agent picks one from a list, so a key is entered once
  even when several agents share it.
- **Workspace switcher.** The sidebar's "Shadows" title becomes a dropdown
  (title trigger with a chevron, items with a label, a description and a check
  on the current one): **Shadows** ("Plan and build projects") and **AYLUL**
  ("Your agents and their conversations"). The title shows the current
  workspace, and the choice is remembered in the browser.
  - Shadows shows today's sidebar: projects, their conversations and
    workflows. The separate "AYLUL" entry above the projects is removed.
  - AYLUL shows the agents, each with its conversations, laid out like
    projects, with "New agent" in place of "New project", and a "Providers"
    link.
- A conversation is started under its agent in the sidebar, so it belongs to
  that agent from creation and never changes.

**My calls (Claude):**

- **Owner: a `profiles` service in `shadows-core`** (the twelfth), whose one
  job is AYLUL's agent configuration: providers in `providers.rs`, agents in
  `agents.rs`. Keys and variables stay with `secrets`. A provider's key is
  stored under the scope `provider/<slug>` as `PROVIDER_API_KEY`, and an
  agent's variables are stored under its slug. `/` cannot appear in an agent
  slug, so the two never collide.
- **A provider is:**
  - `slug`: made from the name, never changed
  - `name`
  - `base_url`: an Anthropic-compatible URL
  - `models`: an ordered list of model ids, 1 to 20, no duplicates
  - `context_window`: tokens, default 200000
  - its key, entered in a password field and never shown again
  - `created_at` and `updated_at`
  A provider used by an agent cannot be deleted (`PROVIDER_IN_USE`, naming the
  agents). Removing a model that an agent has as its default is refused the
  same way.
- **An agent is:**
  - `slug`: made from the name, never changed; it names the home folder and
    the agent's secrets
  - `name`
  - `provider`: a provider slug
  - `default_model`: one of the provider's models
  - its environment variables (§10, now per agent)
  - `created_at` and `updated_at`
- **At spawn:**
  - The provider key becomes `ANTHROPIC_AUTH_TOKEN`, and the provider URL
    becomes `ANTHROPIC_BASE_URL`.
  - `key_env` and the daemon-variable fallback are removed.
  - A provider with no key fails the turn with `AYLUL_KEY_MISSING`, naming the
    provider.
- **The model menu comes from the adapter, restricted by the provider.**
  `claude-agent-acp` 0.81.1 honours `availableModels` in the user
  `settings.json` (`session-model.js`, `applyAvailableModelsAllowlist`). It
  shows those exact ids and passes them unchanged to `setModel`, unless two
  things intervene:
  - a `modelOverrides` entry rewrites an id
  - `ANTHROPIC_CUSTOM_MODEL_OPTION` adds an entry outside the list

  So the opening refuses, with a named error, a `settings.json` that holds
  `modelOverrides`, or an `env` key in the reserved list below. The adapter may
  show or hide its own "Default" entry (it hides it when the client supports
  `recommendedValue`).
  - At each spawn, Shadows writes two keys in the agent home's
    `settings.json`: `availableModels` (the provider's `models`) and `model`
    (the agent's `default_model`). Every other key stays as Mohammed left it.
  - The adapter always adds a "Default" entry. Shadows drops `default` from an
    AYLUL session's model choices, so the menu lists each provider model once.
  - The env still sets `ANTHROPIC_MODEL` and the `ANTHROPIC_DEFAULT_*_MODEL`
    names to `default_model`, so Claude Code's internal small-model calls reach
    a model the provider has.
  - The context-window pair takes the provider's `context_window`.
- **Home:** `<home_root>/profiles/<agent slug>/`, seeded from the repository's
  persona and settings templates when the agent is created. Deleting an agent
  keeps its home folder on disk. The "aylul" home that exists today is reused
  by an agent whose slug is `aylul`.
- **The conversation holds its agent:** a `profile` column on the thread, set
  when the thread is created under an agent and never changed. Opening
  resolves the agent, then its provider. A conversation whose agent was deleted
  fails with `AYLUL_PROFILE_NOT_FOUND`, and its history stays readable.
- **Remembered model and effort are per agent.**
- **AYLUL is available** when at least one agent exists. With no provider, the
  AYLUL workspace shows "Add a provider first". With providers but no agent, it
  shows "Add an agent to start".
- **`aylul.json` shrinks to `home_root`.** The other keys are refused, and
  `env_file` with them, so a stale file fails loudly at start. The one-time env
  import of §10 is removed, because the variables already imported stay in the
  database.
- **Amended after a Codex review (2026-10-19):**
  - **The provider decides where its key goes.** These names are reserved:
    - for an agent's variables (refused at set; an existing row fails the
      opening with `SECRET_NAME_RESERVED` naming it)
    - inside the agent home's `settings.json` `env`

    They are `ANTHROPIC_BASE_URL`, `ANTHROPIC_MODEL`,
    `ANTHROPIC_DEFAULT_OPUS_MODEL`, `ANTHROPIC_DEFAULT_SONNET_MODEL`,
    `ANTHROPIC_DEFAULT_HAIKU_MODEL`, `ANTHROPIC_SMALL_FAST_MODEL`,
    `ANTHROPIC_CUSTOM_MODEL_OPTION`, `CLAUDE_CODE_MAX_CONTEXT_TOKENS` and
    `CLAUDE_CODE_AUTO_COMPACT_WINDOW`, plus §10's three.
  - **A provider URL is `https://`.** `http://` is accepted only for
    `localhost`, `127.0.0.1` and `[::1]`.
  - **The model id `default` is refused.** Each provider model is listed once,
    and the session's current model is always one of them.
  - **Agent slugs are never reused.** Deleting an agent marks it deleted and
    keeps its row. Its slug stays taken, its conversations stay readable by
    their URL, and sending in one gives `AYLUL_PROFILE_NOT_FOUND`. Deleting an
    agent also closes its live adapters. Every send checks the thread's agent
    before it reuses a live adapter.
  - Deleted agent rows keep their slug permanently. Deleting an unused provider
    sets only deleted agents provider references to null; live agents require a provider.
  - **Edits apply at the next spawn,** including a resumed conversation. The
    agent and provider forms say so.
  - **Conversations live in one hidden assistant project,** slug `aylul`, whose
    directory is `home_root`. The agents list returns its id, so a draft under
    an agent can create its thread.
  - **The conversations that exist today belong to the agent `aylul`.** The
    migration sets `profile = 'aylul'` on every AYLUL thread. When Mohammed
    creates an agent named "AYLUL", it takes that slug and those
    conversations. Its earlier variables (`API_KEY`) are already stored under
    `aylul` and stay.
  - **`aylul.json`'s cutover is manual.** The file becomes
    `{"home_root": "..."}`. The provider URL and key are re-entered under
    Providers, and the old keys are refused at start, naming each one.
  - **Writes to `settings.json`** are serialized per home and skipped when the
    two keys are already right. They use a unique temp file and a rename. Key
    order may change. Every other key and value is kept.
  - **The workspace follows the page:** an AYLUL conversation or agent page
    shows the AYLUL workspace whatever was stored. Switching workspaces
    navigates to that workspace's last page or its home.
- **Web:**
  - The workspace switcher above.
  - "Providers" opens a list and one form: name, URL, API key, models, context
    window. Deleting asks for confirmation.
  - "New agent" and each agent's "Agent settings" (like "Project settings")
    open one form: name, a provider dropdown, a default-model dropdown with that
    provider's models, and the agent's environment variables. Deleting asks for
    confirmation.
  - The §10 section leaves the global Settings page.
  - The conversation header names its agent. The model menu shows the
    provider's models.

**Acceptance:**

1. Mohammed switches the sidebar to AYLUL and adds provider "MiniMax" (URL,
   key, two models). He creates agent "AYLUL" on it, starts a conversation
   under it, and it answers. Its model menu shows exactly those two models,
   once each.
2. He creates a second agent on the same provider without entering the key
   again, and it answers too. Switching back to Shadows shows the projects as
   before, with no AYLUL entry.
3. Switching between the two models in one conversation works, and the next
   reply says which model answered.
4. Deleting the provider while agents use it is refused, naming them.
   Deleting the second agent leaves its conversation readable, and sending in
   it shows `AYLUL_PROFILE_NOT_FOUND`.
5. No provider key appears in any response, log, `settings.json` or plaintext
   database byte.
6. The conversations from before this slice appear under the agent "AYLUL"
   once it is created.
7. An agent variable named `ANTHROPIC_BASE_URL` is refused. A
   `settings.json` holding `modelOverrides` fails the opening with a named
   error, and the key is never sent.
