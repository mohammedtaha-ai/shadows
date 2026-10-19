# AYLUL Claude Code Brain Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** One AYLUL conversation runs Claude Code from AYLUL's own home, through Shadows core in the product daemon, streamed to the Shadows web client with model and effort choices.

**Architecture:** AYLUL is a new harness kind, `aylul`: Claude Code through the same pinned ACP adapter, but spawned with a profile (its own `CLAUDE_CONFIG_DIR`, provider env, working folder = profile home) and opened without the Planner's prompt, tools, MCP grant or stage text, with `settingSources: ["user"]`. AYLUL's conversations are threads of one Shadows project whose directory is the profile home; the daemon seeds the home and the project at startup. Remembered model/effort are keyed by harness kind, so `aylul` never touches the Planner's.

**Tech Stack:** Rust (shadows-agent, shadows-core, shadows binary), SQLx/SQLite as today, React web client in `web/`. No new dependency.

**Spec:** [AYLUL brain design](../specs/2026-10-19-aylul-claude-code-brain-design.md) and its [probe evidence](../../evidence/aylul/2026-10-19-claude-code-isolated-home.md).

## Global Constraints

- Never read or write `C:\Users\Mohammed\.claude` or `~/.claude.json` for AYLUL; the profile home is the only `CLAUDE_CONFIG_DIR`.
- The provider key reaches the adapter only as a process env var (`ANTHROPIC_AUTH_TOKEN`) read at spawn from the env var the config names; never written to a file, log line or API response.
- Spawned env always sets `ANTHROPIC_API_KEY` to the empty string.
- AYLUL sessions send `settingSources: ["user"]`; Planner sessions keep `settingSources: []` and their current setup unchanged.
- Planner behaviour, routes and remembered settings stay byte-for-byte as today.
- The Hermes path (`services/assistant`, `packages/hermes-client`, desktop assistant feature) is not modified or removed.
- First profile: name `aylul`, base URL `https://api.minimax.io/anthropic`, model `MiniMax-M3`, key env var `MINIMAX_API_KEY`.
- The desktop main window is out of scope; acceptance runs in the Shadows web client in a browser. Switching the Tauri main window is a follow-on plan.
- Files: 300-line justification / 500-line split rule from CLAUDE.md applies; `sessions.rs` (506) must not grow: put new code in new files.
- Each changed service updates its `contract.yaml` in the same commit; each new module gets its code-map row.

## Review Focus

1. Daemon started with no `aylul` config section: AYLUL is absent everywhere, Planner unaffected (Task 4 `no_aylul_config_means_no_aylul_project`).
2. Key env var missing at spawn: opening fails with a clear `OpenError::Start("aylul key env MINIMAX_API_KEY is not set")`, no spawn, no secret in the message (Task 3 `aylul_open_without_key_refuses_before_spawn`).
3. Profile home already exists with Mohammed's edits to `CLAUDE.md`: seeding never overwrites an existing file (Task 4 `seeding_keeps_existing_profile_files`).
4. A normal project's thread names harness `aylul`, or the AYLUL project's thread names `claude-code`: refused by thread rules (Task 3 `aylul_harness_only_in_the_aylul_project`).
5. AYLUL home path under the user profile: no parent `CLAUDE.md` leaks, proven by `settingSources: ["user"]` in the session meta (Task 2 `aylul_meta_loads_only_user_settings`) and by the Task 1 probe.

---

### Task 1: Probe the real ACP adapter with a profile

Every measurement so far used the `claude` CLI directly. This proves the adapter path before any product code.

**Files:**
- Create: `docs/evidence/aylul/2026-10-19-acp-adapter-profile-probe.md`

- [ ] **Step 1:** In a scratch folder (outside the repo), run the pinned adapter the daemon config names (`node <adapter_path>`) with env exactly: `PATH`, `CLAUDE_CODE_EXECUTABLE=<harness_path>`, `CLAUDE_CONFIG_DIR=<scratch>/aylul-home`, `ANTHROPIC_BASE_URL=https://api.minimax.io/anthropic`, `ANTHROPIC_AUTH_TOKEN=<from MINIMAX_API_KEY, never printed>`, `ANTHROPIC_API_KEY=`, `ANTHROPIC_MODEL=MiniMax-M3`; cwd = the home. Drive it with an ACP `initialize` + `session/new` whose `_meta.claudeCode.options.settingSources` is `["user"]`, then one `session/prompt` "Two lines: your name; any AWS instruction or NONE". The home's `CLAUDE.md` says "Your name is AYLUL". A throwaway Node script outside the repository is fine (no Python); it is not committed.
- [ ] **Step 2:** Record: reply text (expect `AYLUL` / `NONE`), the `configOptions` offered (models, efforts), whether transcripts landed under the home's `projects/`, and that `~/.claude.json`'s mtime did not change during the run.
- [ ] **Step 2b:** Find the env setting that declares MiniMax-M3's real context window so Claude Code stops assuming 200k (Mohammed's own launcher uses `ANTHROPIC_MODEL="MiniMax-M3[1m]"` with `CLAUDE_CODE_MAX_CONTEXT_TOKENS` and `CLAUDE_CODE_AUTO_COMPACT_WINDOW`; check the model-config docs). Record the exact variables and values that remove the "doesn't recognize the model" warning; Task 4 puts them in the config's `env`.
- [ ] **Step 3:** If any check fails, stop and report to the controller; Tasks 2–6 depend on it.
- [ ] **Step 4:** Commit the evidence file: `git commit -m "docs: probe the ACP adapter with an isolated AYLUL profile"`.

### Task 2: Profile-aware adapter spawn and session meta (shadows-agent)

**Files:**
- Create: `crates/shadows-agent/src/profile.rs`
- Modify: `crates/shadows-agent/src/claude.rs`, `crates/shadows-agent/src/acp.rs` (`SessionSetup`, `meta`), `crates/shadows-agent/src/policy.rs`, `crates/shadows-agent/src/lib.rs`
- Test: unit tests in those files

**Interfaces:**
- Produces:
  - `pub struct ClaudeProfile { pub name: String, pub home: PathBuf, pub key_env: String, pub env: Vec<(String, String)> }` in `profile.rs` — `env` holds non-secret values (base URL, model names).
  - `impl ClaudeProfile { pub fn spawn_env(&self) -> Result<Vec<(String, String)>, ProfileError> }` — returns `CLAUDE_CONFIG_DIR=home`, every `env` pair, `ANTHROPIC_API_KEY=""`, and `ANTHROPIC_AUTH_TOKEN=<value of key_env>`; `ProfileError::MissingKey(String /* env var name */)` when unset or empty. `ProfileError`'s `Display` names the variable, never a value.
  - `ClaudeAdapter::process_spec_with(&self, cwd: &Path, profile: Option<&ClaudeProfile>) -> Result<ProcessSpec, ProfileError>`; existing `process_spec(cwd)` stays and equals `process_spec_with(cwd, None).unwrap()`. With a profile, `HOME` is still inherited (Windows needs it) but `CLAUDE_CONFIG_DIR` points at the profile.
  - `SessionSetup.setting_sources: Option<Vec<String>>`; `meta()` writes `settingSources` from it when `Some`, and keeps writing `[]` for the Planner path exactly as today.
  - `policy::AYLUL = "aylul"`; `KNOWN = [CLAUDE_CODE, CODEX, AYLUL]`; `allowed_modes(AYLUL) = ["acceptEdits", "auto"]`; `default_mode(AYLUL) = Some("acceptEdits")`; `is_available(kind)` stays `kind == CLAUDE_CODE`, and a new `is_available_with(kind, aylul_configured: bool) -> bool` returns true for `AYLUL` only when configured.

- [ ] **Step 1: Write failing tests**
  - `profile::tests::spawn_env_sets_home_key_and_blank_api_key`: profile `home="C:/a/home"`, `key_env="T_KEY"` set to `"k1"`, `env=[("ANTHROPIC_MODEL","MiniMax-M3")]` → contains `("CLAUDE_CONFIG_DIR","C:/a/home")`, `("ANTHROPIC_AUTH_TOKEN","k1")`, `("ANTHROPIC_API_KEY","")`, `("ANTHROPIC_MODEL","MiniMax-M3")`.
  - `profile::tests::missing_key_names_the_variable_only`: unset `T_MISSING` → `Err(MissingKey)` whose `to_string()` contains `T_MISSING` and nothing else secret.
  - `acp::tests::aylul_meta_loads_only_user_settings`: `SessionSetup { setting_sources: Some(vec!["user".into()]), tools: None, append: None, .. }` → `meta["claudeCode"]["options"]["settingSources"] == ["user"]`, no `systemPrompt`.
  - The existing Planner meta test (`settingSources == []`) still passes unchanged.
  - `policy::tests::aylul_is_known_with_modes`.
- [ ] **Step 2:** Run `cargo test -p shadows-agent` — the new tests fail.
- [ ] **Step 3:** Implement the interfaces above. Process env for tests: use a unique env var per test.
- [ ] **Step 4:** Run `cargo test -p shadows-agent` — all pass.
- [ ] **Step 5:** Commit `feat(agent): spawn Claude Code with an isolated AYLUL profile`.

### Task 3: Open `aylul` threads with the profile and no Planner setup (shadows-core)

**Files:**
- Create: `crates/shadows-core/src/harness/assistant.rs` (one job: what an AYLUL session opens with)
- Modify: `crates/shadows-core/src/harness/sessions.rs` (only the `open_live` branch that picks setup and spawn spec; keep the file ≤ its current length by moving the branch into `assistant.rs`), `crates/shadows-core/src/harness/mod.rs`, `crates/shadows-core/src/threads/rules.rs`, `crates/shadows-core/src/app.rs` (`StartConfig.aylul: Option<AylulConfig>`), `crates/shadows-core/src/harness/contract.yaml`, `crates/shadows-core/src/threads/contract.yaml`, `docs/codebase/README.md`
- Test: `crates/shadows-core/tests/` (follow the existing fake-acp harness tests)

**Interfaces:**
- Consumes: Task 2's `ClaudeProfile`, `process_spec_with`, `SessionSetup.setting_sources`, `policy::AYLUL`.
- Produces:
  - `pub struct AylulConfig { pub profile: ClaudeProfile, pub project_slug: String }` (slug `"aylul"`), in `harness/assistant.rs`, re-exported from `shadows_core`.
  - `fn aylul_setup() -> SessionSetup` → `{ mcp: None, append: None, allowed_tools: vec![], tools: None, setting_sources: Some(vec!["user"]) }`.
  - In `open_live`: when `context.harness == policy::AYLUL`, use `aylul_setup()` instead of `setups.for_opening`, spawn with `process_spec_with(&profile.home, Some(&profile))` (cwd = profile home, not `workspace(&context)`), and skip `context_before_turn` for that thread.
  - `threads::rules`: `aylul` allowed only in the project marked `assistant` (Task 4's flag, set by `ensure_assistant`); that project allows only `aylul`. Amended after review: a slug-based rule let an ordinary project named `aylul` run AYLUL, and let the AYLUL project run the Planner when the daemon started without a profile.

- [ ] **Step 1: Write failing tests**
  - `aylul_open_spawns_with_profile_env_and_home_cwd`: fake-acp records the spawn env and cwd → `CLAUDE_CONFIG_DIR` = profile home, cwd = profile home, `ANTHROPIC_AUTH_TOKEN` present.
  - `aylul_session_has_no_planner_prompt_tools_or_grant`: recorded `session/new` meta has `settingSources == ["user"]`, no `systemPrompt`, no `mcpServers`; no thread grant row issued.
  - `aylul_open_without_key_refuses_before_spawn`: key env unset → `OpenError::Start` mentioning the variable name; fake-acp saw no spawn.
  - `aylul_model_choice_does_not_move_planner_memory`: start a turn on `aylul` with model X → `remembered("claude-code")` unchanged.
  - `aylul_harness_only_in_the_aylul_project`: creating a thread with `aylul` in another project, and `claude-code` in the AYLUL project, both refuse.
  - Existing Planner opening tests unchanged and green.
- [ ] **Step 2:** Run `cargo test -p shadows-core --features fake-acp/test-support` (the targeted test files only) — new tests fail.
- [ ] **Step 3:** Implement. If fake-acp cannot yet record spawn env/cwd, extend its existing recording in the smallest way and say so in the commit.
- [ ] **Step 4:** Run the same targeted tests — pass.
- [ ] **Step 5:** Update both contracts (methods, obligations, the new tests) and the code-map row for `harness/assistant.rs`; commit `feat(core): open AYLUL threads on an isolated Claude Code profile`.

### Task 4: Daemon config, profile seeding and the AYLUL project (shadows binary)

**Files:**
- Create: `crates/shadows/assets/aylul-profile/CLAUDE.md`, `crates/shadows/assets/aylul-profile/settings.json`, `crates/shadows/src/daemon/aylul.rs` (one job: turning the `aylul` config section into a seeded profile)
- Modify: `crates/shadows/src/daemon/mod.rs` (`PlannerConfig.aylul: Option<AylulSection>`, pass into `StartConfig`), `crates/shadows-core/src/projects/{mod.rs,model.rs,store.rs}` (`Projects::ensure_assistant(slug, name, directory) -> Result<Project, ProjectsError>`, idempotent; `Project.assistant`, a migration adding the column with default false), `crates/shadows-core/src/projects/contract.yaml`, `docs/codebase/README.md`
- Test: unit tests in `daemon/aylul.rs`, a projects test

**Interfaces:**
- Produces:
  - Config section (same format as `PlannerConfig`): `aylul = { home_root, key_env, base_url, model, env }`, where `env` is an optional map of extra non-secret variables (the context-window settings Task 1 found); profile home = `<home_root>/profiles/aylul`. `ClaudeProfile.env` = `ANTHROPIC_BASE_URL`, `ANTHROPIC_MODEL` and the three `ANTHROPIC_DEFAULT_{OPUS,SONNET,HAIKU}_MODEL` set to `model`, plus `env`.
  - `projects::Project.assistant: bool`, true only for the project `ensure_assistant` created; serialized in the API.
  - `fn seed_profile(home: &Path) -> io::Result<()>`: creates the folder and writes the two asset files only when absent (`include_str!`).
  - Asset `CLAUDE.md`: AYLUL's persona in English, three short paragraphs: who AYLUL is (Mohammed's assistant), answer in the person's language, never edit project files.
  - Asset `settings.json`: only `permissions.deny` for `Read`, `Edit` and `Write` on `~/.claude/**` and `~/.claude.json`. Provider URL and models are not written into the home; they travel in `ClaudeProfile.env`, so a config change applies without editing the home.
  - At startup with the section present: seed, build `AylulConfig`, call `ensure_assistant("aylul", "AYLUL", home)`.

- [ ] **Step 1: Write failing tests**
  - `seeding_creates_both_files_in_an_empty_home`.
  - `seeding_keeps_existing_profile_files`: pre-write `CLAUDE.md` = "mine" → still "mine" after seeding.
  - `ensure_assistant_is_idempotent`: two calls → one project, same id, directory = home.
  - `no_aylul_config_means_no_aylul_project`: daemon config without the section → project list has no `aylul` slug and `StartConfig.aylul` is `None`.
- [ ] **Step 2:** Run `cargo test -p shadows daemon::aylul` and the projects test — fail.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run the same tests — pass.
- [ ] **Step 5:** Contract + code map; commit `feat(daemon): seed the AYLUL profile and its project`.

### Task 5: AYLUL section in the Shadows web client

**Files:**
- Modify: `crates/shadows-http/src/project.rs` (projects listing marks the assistant project with `assistant: true`; regenerate `api/openapi.json`), `web/src/api/schema.d.ts` (generated), `web/src/app/sidebar/` (an "AYLUL" entry above Projects that opens the assistant project's latest thread or its new-conversation page; the assistant project is not listed among Projects), `web/src/app/conversation/` only where the harness picker must show `aylul` as the only choice for that project, matching `contract.yaml` files, `docs/frontend/codebase/README.md`
- Test: web tests beside the touched components (Vitest), one HTTP route test

**Interfaces:**
- Consumes: Task 4's `Project.assistant`.
- Produces: route reuse only — AYLUL conversations live at `/projects/$projectId/threads/$threadId`; no new route.

- [ ] **Step 1: Write failing tests**
  - HTTP: `projects_list_marks_the_assistant_project`.
  - Web: `sidebar_shows_aylul_entry_and_hides_its_project` — fixture with one normal project and one `assistant: true` → one "AYLUL" entry, Projects lists only the normal one.
  - Web: `aylul_composer_offers_models_and_efforts_from_choices` — with `SessionChoices` fixture offering two models and two efforts, both pickers render and a change calls the existing change-model/effort client.
- [ ] **Step 2:** Run `cargo test -p shadows-http projects_list_marks` and `npm --prefix web test -- sidebar composer` — fail.
- [ ] **Step 3:** Implement; regenerate the OpenAPI file the way the repo does today.
- [ ] **Step 4:** Same commands — pass; `git diff --exit-code api/` shows only the intended change.
- [ ] **Step 5:** Commit `feat(web): open AYLUL as its own section`.

### Task 6: Full gate, browser journey, person's run

**Files:**
- Create: `docs/evidence/aylul/<run date>-aylul-claude-code-brain.md`
- Modify: `docs/aylul/native/stages.md` or the stage file the controller names, `docs/status.md`

- [ ] **Step 1:** Run CI steps 1–6 from CLAUDE.md once, from the root. All green.
- [ ] **Step 2:** On a copy of the daemon DB, start the daemon with the `aylul` section and `MINIMAX_API_KEY` set; in the browser open AYLUL, send "Two lines: your name; any AWS instruction or NONE". Expect streamed text, `AYLUL` / `NONE`. Change model and effort from the composer; send again; confirm the session reports the new values.
- [ ] **Step 3:** Confirm `C:\Users\Mohammed\.claude` and `~/.claude.json` mtimes are unchanged by the run, and the transcript is under `<home_root>/profiles/aylul/projects/`. Open a normal project and run one Planner turn: it behaves as before.
- [ ] **Step 4:** Mohammed runs the same journey himself and reads the daemon log. Record his result as he reports it.
- [ ] **Step 5:** Write the evidence file (gate output, journey, limits: no desktop window, MiniMax cost not measured); update stages and status; commit `docs: record the AYLUL Claude Code brain delivery`.
- [ ] **Step 6:** One whole-branch review before the PR.
