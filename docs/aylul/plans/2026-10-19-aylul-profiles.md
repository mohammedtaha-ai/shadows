# AYLUL agents and providers from the UI: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The sidebar title becomes a workspace switcher (Shadows / AYLUL). In
AYLUL, Mohammed adds providers (URL, key, models) once and creates agents that
each pick a provider. A conversation starts under its agent, and its model menu
shows exactly that provider's models, once each.

**Architecture:**
- A new `profiles` service in `shadows-core` owns provider rows
  (`providers.rs`) and agent rows (`agents.rs`). Keys and variables stay in
  `secrets`: a provider's key is under the scope `provider/<slug>`, and an
  agent's variables are under its slug.
- AYLUL conversations live in one hidden assistant project (slug `aylul`,
  directory `home_root`). A thread carries its agent in `profile`, fixed at
  creation.
- The AYLUL opening checks the agent, then its provider. It writes
  `availableModels` and `model` into the agent home's `settings.json`, and
  builds the spawn environment.
- `aylul.json` keeps only `home_root`.

**Tech Stack:** Rust (SQLx 0.9, serde_json), axum, React in `web/`. No new
dependency.

**Spec:** [AYLUL brain §11](../specs/2026-10-19-aylul-claude-code-brain-design.md)
(read §7, §10 and §11, including §11's "Amended after a Codex review").
Amended after a Codex review of this plan on 2026-10-19.

## Global Constraints

- No provider key in any response, event, log, error, `Debug`, `settings.json`
  or test message. §10's secret rules hold unchanged.
  - Every route with a JSON body uses the fixed-text body extractor that
    `aylul_env.rs` already uses (`EnvJson`).
- The provider key is the secret `PROVIDER_API_KEY` under the scope
  `provider/<slug>`. `key_env` and the daemon-variable fallback are removed.
- Reserved names:
  - The list is in spec §11, "Amended after a Codex review", and it extends
    §10's three.
  - An agent variable with a reserved name is refused at set.
  - At opening, an existing variable row with a reserved name fails the
    opening with `SECRET_NAME_RESERVED`, naming it.
  - A reserved name in the agent home's `settings.json` `env` also fails the
    opening, and so does any `modelOverrides` key there.
- Slugs (provider and agent are separate namespaces): `^[a-z][a-z0-9-]{0,31}$`.
  - The slug is made from the name; a name with no Latin letters or digits
    falls back to `provider` or `agent`.
  - The base is truncated to 29 characters, and a collision adds `-2`, `-3`,
    and so on.
  - The slug never changes.
  - Agent slugs are never reused: a deleted agent's row stays with
    `deleted_at`.
- Provider fields:
  - `base_url` is `https://`; `http://` is allowed only for `localhost`,
    `127.0.0.1` and `[::1]`.
  - `models` holds 1..=20 ids, each 1..=128 printable characters with no
    spaces, no duplicates, and never `default`.
  - `context_window` is 1000..=10000000 (default 200000).
- An agent's `default_model` is one of its provider's models.
  - Deleting a provider that a live agent uses is refused with
    `PROVIDER_IN_USE`, naming the agents.
  - Removing a model that is some agent's default is refused the same way.
  - These checks run inside the same write transaction as the change, after
    replay classification.
- `settings.json`: only `availableModels` and `model` are written. Every other
  key and value is kept; key order may change. Writes are serialized per home
  (one async mutex per path), use a unique temp file and a rename, and are
  skipped when both keys already match. A file that is not a JSON object fails
  the opening with a named error and is never overwritten.
- Model choices: `default` is dropped from AYLUL choices whether or not the
  adapter sends it. Planner choices are unchanged.
- One rule names the remembered-settings key: `preference_key(harness,
  profile)`, which is `aylul:<slug>` for AYLUL and the harness kind otherwise.
  It is used by:
  - the turn's model/effort write (`turns/store/turn.rs`)
  - the opening's read
  - `Harness::change_model`
- Commands follow the project pattern: public methods take `command_id` and
  return `CoreError`; `user_command`; one `write_txn`; `profile` is part of
  the fingerprint wherever it is a parameter.
- Product logic is ours (CLAUDE.md): no new crate.
- File rules:
  - `sessions.rs` (494 lines) is split by responsibility before it gains
    anything. Move the live-adapter cache (`open_live` and its map) into
    `harness/live.rs` and name each file's one job.
  - Contracts and code-map rows are updated in the same commit.
  - Rust lines stay under 100 columns.
  - Use the project `target/` only, run only the touched tests per commit, and
    run the full gate once at the end.

## Review Focus

1. Delete an agent while its conversation has a live adapter: the next send
   gives `AYLUL_PROFILE_NOT_FOUND`, also when it was the last agent. The
   history still renders by URL.
2. Delete agent "Work", then create a new "Work": it gets `work-2`, and the old
   conversations stay with the deleted agent.
3. An agent variable or a `settings.json` `env` entry named
   `ANTHROPIC_BASE_URL` never redirects the key; the opening fails, naming it.
4. Two conversations of one agent open at once: `settings.json` ends valid,
   with Mohammed's keys intact.
5. A deep link to an AYLUL conversation shows the AYLUL workspace even when
   Shadows was stored. Switching back navigates to a Shadows page.

---

### Task 1: `profiles` service; threads carry an agent; migration

**Files:**
- Create: `crates/shadows-core/migrations/0029_aylul_profiles.sql`, with:
  - `aylul_provider(slug TEXT PRIMARY KEY, name TEXT NOT NULL, base_url TEXT NOT NULL, models TEXT NOT NULL, context_window INTEGER NOT NULL, created_at TEXT, updated_at TEXT)`, where `models` is a JSON array
  - `aylul_agent(slug TEXT PRIMARY KEY, name TEXT NOT NULL, provider TEXT REFERENCES aylul_provider(slug) ON DELETE SET NULL, default_model TEXT NOT NULL, deleted_at TEXT NULL, created_at TEXT, updated_at TEXT)`
  - `ALTER TABLE planning_thread ADD COLUMN profile TEXT NULL`
  - `UPDATE planning_thread SET profile = 'aylul' WHERE harness = 'aylul'` (the legacy mapping)
- Create: `crates/shadows-core/src/profiles/{mod.rs,model.rs,providers.rs,agents.rs,store.rs,contract.yaml}`
- Modify: `crates/shadows-core/src/app.rs` (compose `Profiles`, accessor `profiles()`, shared with `Sessions`), `lib.rs`, and `error.rs`. Add these variants:
  - `AYLUL_PROFILE_NOT_FOUND`
  - `AYLUL_PROVIDER_NOT_FOUND`
  - `PROVIDER_IN_USE`
  - `PROFILE_INVALID`
  - `SECRET_NAME_RESERVED`

  Map each one in `crates/shadows-http/src/failure/other.rs` (404, 404, 409, 400, 409) and in `crates/shadows-mcp/src/refusal.rs::code_of`.
- Modify: `crates/shadows-core/src/threads/{mod.rs,model.rs,store}`:
  - `create` takes `profile: Option<&str>`, which is required for harness `aylul` (the agent must exist and not be deleted) and refused otherwise
  - `PlanningThread.profile` and `TurnContext.profile`
  - the fork INSERT copies `profile`
  - the row loaders read it
  - `list_for_profile(slug)` returns threads newest first by `created_at`, then `id`
- Modify: `crates/shadows-core/src/secrets`:
  - `set_provider_key(command_id, provider, value)`
  - `remove_provider_key(command_id, provider)`
  - `provider_key_set(provider) -> Result<bool, CoreError>`
  - `pub(crate) async fn remove_scope_in(conn, scope)`, used inside a profiles deletion transaction
  - each of these is declared in the secrets contract
  - extend the reserved-name list
- Modify: `docs/codebase/README.md`, `docs/superpowers/specs/2026-09-26-application-core-design.md` §14.4 (twelve services), and CLAUDE.md's service count
- Test: `crates/shadows-core/tests/aylul_profiles.rs`, and `threads` tests

**Interfaces (produced):**
- `pub struct Provider { slug, name, base_url, models: Vec<String>, context_window: u32, key_set: bool, created_at, updated_at }`
- `pub struct ProviderInput { name, base_url, models: Vec<String>, context_window: Option<u32> }`
- `pub struct Agent { slug, name, provider, default_model, deleted: bool, created_at, updated_at }`
- `pub struct AgentInput { name, provider, default_model }`
- `impl Profiles` (all methods return `Result<_, CoreError>`):
  - `providers()`; `agents()` (live only) and `agent(slug)` (including deleted); `provider(slug)`
  - `create_provider`, `update_provider` and `delete_provider` (`delete_provider` also removes the provider key)
  - `create_agent`, `update_agent` and `delete_agent`. `create_agent` seeds the home before inserting, and seeding writes only absent files, so a replay is safe. `delete_agent` soft-deletes and removes the agent's variables.
  - `assistant_project() -> ProjectId`

- [x] **Step 1: Write the failing tests.**
  - `provider_slug_rules` (non-Latin name, 40-char name, collision suffixes)
  - `invalid_provider_input_is_refused`, covering:
    - no models, 21 models, a duplicate, a space, and the id `default`
    - a remote `http://` URL (refused) and a `localhost` one (accepted)
    - `context_window` out of range
  - `agent_default_must_be_a_provider_model`
  - `provider_in_use_cannot_be_deleted`
  - `removing_an_agents_default_model_is_refused`
  - `deleted_agent_slug_is_never_reused`
  - `delete_agent_removes_its_variables`
  - `delete_provider_removes_its_key`
  - `provider_key_is_write_only`
  - `reserved_agent_variable_is_refused`
  - `aylul_thread_requires_a_live_agent`
  - `planner_thread_refuses_an_agent`
  - `fork_keeps_profile`
  - `legacy_aylul_threads_map_to_aylul`
  - `threads_list_by_profile_only`
  - `replay_with_other_input_conflicts`
- [x] **Step 2:** Run `cargo test -p shadows-core --test aylul_profiles`. Expected: FAIL.
- [x] **Step 3:** Implement. The contract lists the methods and their rules.
- [x] **Step 4:** Run the same tests, plus `cargo test -p shadows-core --test storage_contract`, `cargo test -p shadows --test codemap`, `cargo test -p shadows-core --test contracts` and `cargo test -p shadows-mcp`. Expected: PASS.
- [x] **Step 5:** Commit `feat(core): AYLUL providers and agents; threads carry their agent`.

### Task 2: Opening by agent, model allowlist, `aylul.json` shrinks

**Files:**
- Split first: move the live-adapter cache out of `crates/shadows-core/src/harness/sessions.rs` into `harness/live.rs`, with no behaviour change. Commit it alone: `refactor(core): live adapters in their own file`.
- Modify: `crates/shadows-core/src/harness/assistant.rs` and `live.rs`:
  - Every AYLUL send checks the thread's agent (live, not deleted) and its provider, before a cached adapter is reused and before the generic availability check.
  - `delete_agent` closes the live adapters of its threads.
  - The opening builds the `ClaudeProfile`:
    - home `<home_root>/profiles/<agent slug>`
    - `ANTHROPIC_BASE_URL` set to the provider URL
    - `ANTHROPIC_MODEL` and the three `ANTHROPIC_DEFAULT_*_MODEL` set to `default_model`
    - the context-window pair set to the provider's `context_window`
    - the provider key as the token, passed explicitly
- Create: `crates/shadows-core/src/harness/profile_home.rs`:
  - The persona and settings templates move here from `crates/shadows/assets/aylul-profile/` as core assets.
  - `seed(home)` writes only absent files.
  - `check(home)`: refuses `modelOverrides` and any reserved `env` name.
  - `write_models(home, models, default)`: serialized per path, skipped when unchanged, temp file plus rename.
- Modify: `crates/shadows-agent/src/profile.rs`: remove `key_env` and the daemon fallback.
- Modify: `crates/shadows-core/src/harness/*`, so AYLUL `SessionChoices` drop `default`, and the current model is a provider model.
- Add `preference_key` and use it in `turns/store/turn.rs`, the opening read and `Harness::change_model`.
- Modify: `crates/shadows/src/aylul.rs`:
  - `AylulSection` becomes `{ home_root }` only, and an old key is refused with a message naming it
  - remove the provider env, `env_file`, `import_env` and `parse_dotenv`
  - core `AylulConfig` becomes `{ home_root, project_slug }`
  - `ensure_project` creates the hidden `aylul` project with directory `home_root`
- Modify: `crates/shadows/src/cli/mod.rs` and `daemon/mod.rs` (no import call; the new config)
- Modify: `crates/shadows-core/src/secrets` (drop `import_env_once` and `import_needed`; the marker table stays, unused)
- Modify: contracts and code-map rows
- Test:
  - `aylul_session.rs`:
    - `opening_uses_agent_and_provider`
    - `two_agents_share_one_provider_key`
    - `deleted_agent_with_live_adapter_is_not_found`
    - `deleting_last_agent_is_not_found_not_unavailable`
    - `missing_provider_key_is_named`
    - `reserved_variable_row_fails_opening`
    - `settings_env_base_url_fails_opening`
    - `model_overrides_fails_opening`
    - `settings_json_keeps_other_keys`
    - `concurrent_openings_keep_settings_valid`
    - `non_object_settings_json_fails_named`
    - `aylul_choices_have_no_default_model`
    - `remembered_model_is_per_agent` (write, read and `change_model`)
  - `profile.rs`: unit tests for the token rule
  - `aylul.rs`: `old_section_keys_are_refused_by_name`

- [x] **Step 1: Probe first.** The plan depends on it. Run the real adapter once with Shadows' own client capabilities, on a temp home whose `settings.json` has `"availableModels": ["MiniMax-M3", "MiniMax-M2.7"]`, with a fake key and `settingSources ["user"]`. Record in `docs/evidence/aylul/2026-10-19-available-models-probe.md`:
  - the `configOptions` model values
  - the current model
  - whether `default` is present

  Pass means both ids are present and the current model is one of them or `default`. Otherwise stop and report.
- [x] **Step 2:** Do the split commit.
- [x] **Step 3:** Write the failing tests listed above.
- [x] **Step 4:** Run `cargo test -p shadows-core -p fake-acp --features fake-acp/test-support --test aylul_session`, `cargo test -p shadows-agent profile` and `cargo test -p shadows --lib aylul`. Expected: FAIL.
- [x] **Step 5:** Implement.
- [x] **Step 6:** Run the same tests. Expected: PASS.
- [x] **Step 7:** Commit `feat(aylul): sessions open by agent on its provider's models`.

### Task 3: HTTP routes and the web

**Files:**
- Create: `crates/shadows-http/src/aylul_profiles.rs`, registered in `crates/shadows-http/src/lib.rs` and documented in `openapi.rs`. Each route calls one method, and every body uses the fixed-text extractor.
  - `GET` and `POST /api/aylul/providers`; `PUT` and `DELETE /api/aylul/providers/{slug}`
  - `PUT` and `DELETE /api/aylul/providers/{slug}/key`
  - `GET /api/aylul/agents`, returning `{project_id, agents}`; `POST`; `PUT` and `DELETE /api/aylul/agents/{slug}`
  - `GET /api/aylul/agents/{slug}/threads`
- Modify: `crates/shadows-http/src/aylul_env.rs`. The routes become `/api/aylul/agents/{slug}/env[/{name}]`, and `/api/aylul/env` is removed.
- Modify: the thread create route (`profile` field); regenerate `api/openapi.json` (`UPDATE_OPENAPI=1 cargo test -p shadows --test openapi`)
- Create:
  - `web/src/api/aylul-profiles.ts`: providers, agents, keys, env and threads. It uses agent-scoped query keys, and create, delete and retitle invalidate them. `aylul-env.ts` is removed.
  - `web/src/app/sidebar/workspace-switcher.tsx`:
    - a title dropdown with label, description and check, holding "Shadows: Plan and build projects" and "AYLUL: Your agents and their conversations"
    - the stored choice lives in `localStorage` (inside try/catch)
    - the page's thread or agent decides the workspace first
    - switching navigates to that workspace's last page, or its home
  - `web/src/app/sidebar/agent-list.tsx`:
    - live agents, each expandable to its conversations, with "New conversation" and "Agent settings"
    - a "Providers" link
    - empty states "Add a provider first" and "Add an agent to start"
  - `web/src/app/agents/provider-settings.tsx`: the providers list and one form (name, URL, an "API key" password field, a models editor, context window, delete with confirmation, "Changes apply to new and resumed conversations")
  - `web/src/app/agents/agent-settings.tsx` (name, a provider dropdown, a default-model dropdown with that provider's models, delete with confirmation), `agent-env.tsx` (the §10 section, scoped to one agent) and `contract.yaml`
- Modify:
  - `web/src/app/sidebar/sidebar.tsx`: the switcher goes in the header, with projects or agents below; in AYLUL, "New agent" replaces "New project"
  - remove `web/src/app/sidebar/aylul-entry.tsx` and its test, and the AYLUL section of the global Settings page
  - first send with a chosen model, in `web/src/app/conversation/draft.tsx`, `composer-bar.tsx`, `composer.tsx` and `useFirstSend`:
    - the draft under an agent shows the provider's models and keeps the chosen one
    - the first send creates the thread with `profile` and opens the session
    - it applies the chosen model and reads its efforts, then sends
  - `web/src/app/conversation/conversation.tsx`: the header names the agent, or "Deleted agent"
  - routes: `/aylul/providers`, `/aylul/agents/new`, `/aylul/agents/{slug}/settings` and `/aylul/agents/{slug}/new`. Existing conversation URLs stay `/projects/$projectId/threads/$threadId`, and the workspace is resolved from the thread's harness.
  - `docs/frontend/codebase/README.md`, and `schema.d.ts` (`gen:api`)
- Test:
  - Rust route tests:
    - `provider_crud_and_key_never_echoed`
    - `key_body_parse_error_does_not_echo`
    - `agent_crud_soft_delete`
    - `provider_in_use_is_409`
    - `env_routes_are_per_agent`
    - `reserved_name_is_409`
    - `thread_create_with_profile`
    - `threads_listed_per_agent`
  - Web:
    - `switcher_shows_both_with_check_on_current`
    - `switcher_choice_survives_reload`
    - `aylul_thread_url_opens_aylul_workspace`
    - `switching_navigates_to_that_workspace`
    - `shadows_workspace_has_no_aylul_entry`
    - `agent_list_shows_conversations_per_agent`
    - `provider_form_saves_and_clears_key`
    - `models_editor_rejects_duplicates_and_default`
    - `agent_form_lists_provider_models`
    - `first_send_keeps_chosen_model`
    - `deleted_agent_conversation_reads`
    - `empty_states_guide_to_provider_then_agent`

- [ ] **Step 1:** Write the failing tests.
- [ ] **Step 2:** Run `cargo test -p shadows-http aylul` and `npm --prefix web test -- aylul agent provider switcher first-send`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run the same tests, plus `cargo test -p shadows --test openapi` and `npm --prefix web run typecheck`. Expected: PASS.
- [ ] **Step 5:** Commit `feat(web): workspace switcher; AYLUL agents and providers`.

### Task 4: Gate (once)

- [ ] Run CI steps 1–6 from `CLAUDE.md`, the 100-column check and `web` gen:api/typecheck/lint/test/build. All must be green.
- [ ] Report the test counts and each file that passed 300 lines, with its one job. Do not push.

The controller runs the browser journey (spec §11 acceptance 1–7) on a copy of
the database and records it in `docs/evidence/aylul/`. Before that journey,
the local `aylul.json` is cut over to `{"home_root": ...}` by hand.
