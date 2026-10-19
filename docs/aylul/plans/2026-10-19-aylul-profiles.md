# AYLUL profiles from the UI: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The sidebar title becomes a workspace switcher (Shadows / AYLUL). In
AYLUL, Mohammed creates several agents (profiles), each with its own provider
URL, key, models and environment variables. A conversation is started under its
agent, and its model menu shows exactly that agent's models, once each.

**Architecture:**
- A new `profiles` service in `shadows-core` owns the profile rows. Keys and
  variables stay in `secrets`, keyed by the profile's slug.
- A thread carries its `profile`. The AYLUL opening resolves it, writes
  `availableModels` and `model` into the profile home's `settings.json`, and
  builds the spawn environment from the profile.
- `aylul.json` keeps only `home_root`.

**Tech Stack:** Rust (SQLx 0.9, serde_json), axum, React in `web/`. No new
dependency.

**Spec:** [AYLUL brain §11](../specs/2026-10-19-aylul-claude-code-brain-design.md)
(read §7, §10 and §11).

## Global Constraints

- No provider key in any response, event, log, error, `Debug`, `settings.json`
  or test message. §10's rules on secrets hold unchanged.
- The key is the profile secret `PROVIDER_API_KEY`. `key_env` and the
  daemon-variable fallback are removed.
- Slug: `^[a-z][a-z0-9-]{0,31}$`, made from the name at creation and never
  changed. `aylul` is allowed and reuses the existing home.
- `models`: 1..=20 ids, each 1..=128 printable characters with no spaces, no
  duplicates. `default_model` is one of them. `context_window` is
  1000..=10000000.
- `settings.json`: only the keys `availableModels` and `model` are written.
  Every other key and its order are kept. If the file is not a JSON object, the
  opening fails with a named error, and the file is never overwritten.
- `default` is dropped from AYLUL model choices only. Planner choices are
  unchanged.
- Remembered model and effort use the `harness_preference` key
  `aylul:<slug>`, so profiles never share them.
- Commands follow the project pattern: public methods take `command_id` and
  return `CoreError`; `user_command`; one `write_txn`.
- Product logic is ours (CLAUDE.md): no new crate.
- File rules: 300/500 lines, contracts and code-map rows updated in the same
  commit, Rust lines under 100 columns, project `target/` only, only the
  touched tests per commit, and the full gate once at the end.

## Review Focus

1. A conversation whose profile was deleted: its history renders, and sending
   gives `AYLUL_PROFILE_NOT_FOUND`, not a panic or a Planner fallback.
2. Mohammed edited the profile's `settings.json` (permissions, persona keys).
   After a spawn his keys are still there, and only `availableModels` and
   `model` changed.
3. Editing a profile's models while a conversation is open: the open adapter
   keeps its menu, and a new conversation shows the new list.
4. The workspace choice survives a reload, and a deep link to an AYLUL
   conversation opens the AYLUL workspace even if Shadows was the last choice.
5. Two profiles with the same name (different slugs are needed): the second
   gets a `-2` slug, never a collision with the first's secrets.

---

### Task 1: `profiles` service and threads carry a profile

**Files:**
- Create: `crates/shadows-core/migrations/0029_aylul_profile.sql`, with:
  - `aylul_profile(slug TEXT PRIMARY KEY, name TEXT NOT NULL, base_url TEXT NOT NULL, models TEXT NOT NULL, default_model TEXT NOT NULL, context_window INTEGER NOT NULL, created_at TEXT, updated_at TEXT)`, where `models` is a JSON array
  - `ALTER TABLE planning_thread ADD COLUMN profile TEXT NULL`
- Create: `crates/shadows-core/src/profiles/{mod.rs,model.rs,store.rs,contract.yaml}`
- Modify: `crates/shadows-core/src/app.rs` (compose `Profiles`, accessor `profiles()`, shared with `Sessions`), `lib.rs`, `error.rs` (`AYLUL_PROFILE_NOT_FOUND`, `PROFILE_INVALID`)
- Modify: `crates/shadows-core/src/threads/{mod.rs,model.rs,store}`:
  - `create` takes `profile: Option<&str>`, which is required when the harness is `aylul` and refused otherwise
  - `PlanningThread.profile: Option<String>`; there is no way to change it after
    creation
  - `list_for_profile(slug) -> Result<Vec<PlanningThread>, CoreError>`, ordered
    by `created_at` then `id`, the newest first
- Modify: `docs/codebase/README.md`, `docs/superpowers/specs/2026-09-26-application-core-design.md` §14.4 (twelve services), and CLAUDE.md's service count
- Test: `crates/shadows-core/tests/aylul_profiles.rs`, and `threads` tests

**Interfaces (produced):**
- `pub struct Profile { slug, name, base_url, models: Vec<String>, default_model, context_window: u32, created_at, updated_at }`
- `pub struct ProfileInput { name, base_url, models: Vec<String>, default_model, context_window: Option<u32> }`
- `impl Profiles`:
  - `list() -> Result<Vec<Profile>, CoreError>` (ordered by `name`, then `slug`)
  - `get(slug) -> Result<Profile, CoreError>`
  - `create(command_id, ProfileInput) -> Result<Profile, CoreError>`
  - `update(command_id, slug, ProfileInput) -> Result<Profile, CoreError>`
  - `delete(command_id, slug) -> Result<(), CoreError>`, which also removes the profile's `secret_env` rows in the same transaction

- [ ] **Step 1: Write the failing tests.**
  - `create_makes_slug_from_name`
  - `same_name_gets_dash_two`
  - `invalid_models_and_default_are_refused` (empty, 21 ids, a duplicate, a space, `default_model` not in the list)
  - `update_keeps_slug`
  - `delete_removes_its_secrets`
  - `list_is_ordered_by_name`
  - `aylul_thread_requires_a_profile`
  - `planner_thread_refuses_a_profile`
  - `threads_list_by_profile_only`
  - `replay_with_other_input_conflicts`
- [ ] **Step 2:** Run `cargo test -p shadows-core --test aylul_profiles`. Expected: FAIL.
- [ ] **Step 3:** Implement. The contract lists the methods and their rules.
- [ ] **Step 4:** Run the same tests, plus `cargo test -p shadows-core --test storage_contract`, `cargo test -p shadows --test codemap` and `cargo test -p shadows-core --test contracts`. Expected: PASS.
- [ ] **Step 5:** Commit `feat(core): AYLUL profiles service; threads carry their profile`.

### Task 2: Opening by profile, model allowlist, `aylul.json` shrinks

**Files:**
- Modify: `crates/shadows-core/src/harness/assistant.rs`:
  - `opening` reads the thread's profile with `profiles.get`, then resolves the secrets
  - it builds the `ClaudeProfile` at spawn: home `<home_root>/profiles/<slug>`; env `ANTHROPIC_BASE_URL`, `ANTHROPIC_MODEL` and the three `ANTHROPIC_DEFAULT_*_MODEL` set to `default_model`, and the context-window pair set to `context_window`
  - the token comes from `PROVIDER_API_KEY`
- Create: `crates/shadows-core/src/harness/profile_home.rs`, with:
  - `seed(home)`: the persona and settings templates move here from the binary, and only absent files are written
  - `write_models(home, models, default)`: reads `settings.json` as a `serde_json::Map`, sets the two keys and writes it back atomically (temp file + rename)
- Modify: `crates/shadows-agent/src/profile.rs`: remove `key_env` and the daemon fallback; the token comes from the stored pair `PROVIDER_API_KEY` only.
- Modify: `crates/shadows-core/src/harness/*`, where AYLUL's `SessionChoices` are built: drop the model `default`. Remembered settings use the key `aylul:<slug>`.
- Modify: `crates/shadows/src/aylul.rs`:
  - `AylulSection` becomes `{ home_root }` only, with `deny_unknown_fields`
  - remove `prepare`'s provider env, `env_file`, `import_env` and `parse_dotenv`
  - `AylulConfig` holds `home_root`
  - availability means "a section exists"; a turn with no profile fails with a named error
- Modify: `crates/shadows/src/cli/mod.rs` and `daemon/mod.rs` (no import call)
- Modify: `crates/shadows-core/src/secrets` (drop `import_env_once` and `import_needed`; keep the marker table in place, unused, because migrations never drop)
- Modify: contracts and code-map rows
- Test:
  - `aylul_session.rs`:
    - `opening_uses_the_threads_profile`
    - `deleted_profile_is_profile_not_found`
    - `missing_provider_key_is_named`
    - `settings_json_keeps_other_keys`
    - `non_object_settings_json_fails_named`
    - `aylul_choices_have_no_default_model`
    - `remembered_model_is_per_profile`
  - `profile.rs`: unit tests for the token rule
  - `aylul.rs`: `old_section_keys_are_refused`

- [ ] **Step 1: Probe first.** The plan depends on this. Run the real adapter once on a temp profile home whose `settings.json` has `"availableModels": ["MiniMax-M3", "MiniMax-M2.7"]`, with a fake key and `settingSources ["user"]`. Record its `configOptions` model values in `docs/evidence/aylul/2026-10-19-available-models-probe.md`. If the adapter does not list exactly `default` plus those ids, stop and report; do not build on a guess.
- [ ] **Step 2:** Write the failing tests listed above.
- [ ] **Step 3:** Run `cargo test -p shadows-core -p fake-acp --features fake-acp/test-support --test aylul_session`, `cargo test -p shadows-agent profile` and `cargo test -p shadows --lib aylul`. Expected: FAIL.
- [ ] **Step 4:** Implement.
- [ ] **Step 5:** Run the same tests. Expected: PASS.
- [ ] **Step 6:** Commit `feat(aylul): sessions open by profile with the profile's models`.

### Task 3: HTTP routes and the web

**Files:**
- Create: `crates/shadows-http/src/aylul_profiles.rs`, with routes that each call one method and hold no rule:
  - `GET /api/aylul/profiles`
  - `POST /api/aylul/profiles` with `{command_id, name, base_url, models, default_model, context_window?}`
  - `PUT /api/aylul/profiles/{slug}`
  - `DELETE /api/aylul/profiles/{slug}?command_id=`
- Modify: `crates/shadows-http/src/aylul_env.rs`: the routes become `/api/aylul/profiles/{slug}/env[/{name}]`, and the old `/api/aylul/env` routes are removed.
- Modify: the thread create route (`profile` field), add `GET /api/aylul/profiles/{slug}/threads`, update `openapi.rs`, and regenerate `api/openapi.json` (`UPDATE_OPENAPI=1 cargo test -p shadows --test openapi`)
- Create:
  - `web/src/api/aylul-profiles.ts` (the env calls move here too; `aylul-env.ts` is removed)
  - `web/src/app/sidebar/workspace-switcher.tsx`: a title dropdown with label, description and check. "Shadows: Plan and build projects" and "AYLUL: Your agents and their conversations". The choice is kept in `localStorage` (inside try/catch); a route under an AYLUL thread or agent wins over the stored choice.
  - `web/src/app/sidebar/agent-list.tsx`: the agents, each expandable to its conversations like a project, with a "New conversation" entry and "Agent settings". With no agents it shows "Add an agent to start".
  - `web/src/app/agents/agent-settings.tsx`: one form for "New agent" and "Agent settings" (name, provider URL, an "API key" password field, a models editor, default model, context window, delete with confirmation), plus the env section
  - `web/src/app/agents/agent-env.tsx`: the §10 env section, scoped to one agent
- Modify:
  - `web/src/app/sidebar/sidebar.tsx` (switcher in the header; projects or agents below; "New agent" replaces "New project" in AYLUL)
  - remove `web/src/app/sidebar/aylul-entry.tsx` and its test
  - `web/src/app/settings/settings-page.tsx` (the §10 section leaves the global Settings page)
  - `web/src/app/conversation/draft.tsx` (a new conversation under an agent creates its thread with that `profile`; the model menu before the session exists shows the agent's models)
  - `web/src/app/conversation/conversation.tsx` (the header names the agent)
  - the routes for `/agents/new`, `/agents/{slug}/settings` and `/agents/{slug}/new`
  - contracts, `docs/frontend/codebase/README.md`, and `schema.d.ts` (`gen:api`)
- Test:
  - Rust route tests: `create_list_update_delete_profile`, `profile_responses_have_no_key`, `env_routes_are_per_profile`, `thread_create_with_profile`, `threads_listed_per_profile`
  - Web: `switcher_shows_both_with_check_on_current`, `switcher_choice_survives_reload`, `aylul_thread_route_opens_aylul_workspace`, `shadows_workspace_has_no_aylul_entry`, `agent_list_shows_conversations_per_agent`, `agent_form_saves_and_clears_key`, `models_editor_requires_default_in_list`, `new_conversation_under_agent_sets_profile`, `no_agents_shows_add_prompt`

- [ ] **Step 1:** Write the failing tests.
- [ ] **Step 2:** Run `cargo test -p shadows-http aylul` and `npm --prefix web test -- aylul profile`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run the same tests, plus `cargo test -p shadows --test openapi` and `npm --prefix web run typecheck`. Expected: PASS.
- [ ] **Step 5:** Commit `feat(web): workspace switcher; AYLUL agents with their conversations and settings`.

### Task 4: Gate (once)

- [ ] Run CI steps 1–6 from `CLAUDE.md`, the 100-column check and `web` gen:api/typecheck/lint/test/build. All must be green.
- [ ] Report the test counts and each file that passed 300 lines, with its one job. Do not push.

The controller runs the browser journey (spec §11 acceptance 1–5) on a copy of
the database and records it in `docs/evidence/aylul/`.
