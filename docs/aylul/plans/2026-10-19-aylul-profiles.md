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
- A thread carries its agent in `profile`. The AYLUL opening resolves the agent
  and then its provider, writes `availableModels` and `model` into the agent
  home's `settings.json`, and builds the spawn environment.
- `aylul.json` keeps only `home_root`.

**Tech Stack:** Rust (SQLx 0.9, serde_json), axum, React in `web/`. No new
dependency.

**Spec:** [AYLUL brain §11](../specs/2026-10-19-aylul-claude-code-brain-design.md)
(read §7, §10 and §11).

## Global Constraints

- No provider key in any response, event, log, error, `Debug`, `settings.json`
  or test message. §10's secret rules hold unchanged.
- The provider key is the secret `PROVIDER_API_KEY` under the scope
  `provider/<slug>`. `key_env` and the daemon-variable fallback are removed.
- Slugs (provider and agent, separate namespaces): `^[a-z][a-z0-9-]{0,31}$`,
  made from the name and never changed. On a collision the slug gets `-2`,
  `-3`, and so on. The agent slug `aylul` is allowed and reuses the existing
  home.
- Provider `models`: 1..=20 ids, each 1..=128 printable characters with no
  spaces, and no duplicates. `context_window` is 1000..=10000000 (default
  200000). `base_url` must be `http(s)://`.
- An agent's `default_model` is one of its provider's models.
  - Deleting a provider that agents use is refused with `PROVIDER_IN_USE`,
    naming the agents.
  - Removing a model that is some agent's default is refused the same way.
- `settings.json`: only `availableModels` and `model` are written. Every other
  key and its order are kept. If the file is not a JSON object, the opening
  fails with a named error, and the file is never overwritten.
- `default` is dropped from AYLUL model choices only. Planner choices are
  unchanged.
- Remembered model and effort use the `harness_preference` key
  `aylul:<agent slug>`.
- Commands follow the project pattern: public methods take `command_id` and
  return `CoreError`; `user_command`; one `write_txn`.
- Product logic is ours (CLAUDE.md): no new crate.
- File rules: 300/500 lines, contracts and code-map rows updated in the same
  commit, Rust lines under 100 columns, project `target/` only, only the
  touched tests per commit, and the full gate once at the end.

## Review Focus

1. A conversation whose agent was deleted: its history renders, and sending
   gives `AYLUL_PROFILE_NOT_FOUND`, not a panic or a Planner fallback.
2. Mohammed edited an agent's `settings.json` (permissions, persona keys).
   After a spawn his keys are still there, and only `availableModels` and
   `model` changed.
3. A provider's models are edited while a conversation is open. The open
   adapter keeps its menu, and a new conversation shows the new list.
4. The workspace choice survives a reload. A deep link to an AYLUL
   conversation opens the AYLUL workspace even if Shadows was the last choice.
5. Two agents share one provider, and the key is replaced once. Both agents'
   next new conversations use the new key.

---

### Task 1: `profiles` service (providers and agents); threads carry an agent

**Files:**
- Create: `crates/shadows-core/migrations/0029_aylul_profiles.sql`, with:
  - `aylul_provider(slug TEXT PRIMARY KEY, name TEXT NOT NULL, base_url TEXT NOT NULL, models TEXT NOT NULL, context_window INTEGER NOT NULL, created_at TEXT, updated_at TEXT)`, where `models` is a JSON array
  - `aylul_agent(slug TEXT PRIMARY KEY, name TEXT NOT NULL, provider TEXT NOT NULL REFERENCES aylul_provider(slug) ON DELETE RESTRICT, default_model TEXT NOT NULL, created_at TEXT, updated_at TEXT)`
  - `ALTER TABLE planning_thread ADD COLUMN profile TEXT NULL`
- Create: `crates/shadows-core/src/profiles/{mod.rs,model.rs,providers.rs,agents.rs,store.rs,contract.yaml}`
- Modify: `crates/shadows-core/src/app.rs` (compose `Profiles`, accessor `profiles()`, shared with `Sessions`), `lib.rs`, and `error.rs`, adding:
  - `AYLUL_PROFILE_NOT_FOUND`
  - `AYLUL_PROVIDER_NOT_FOUND`
  - `PROVIDER_IN_USE`
  - `PROFILE_INVALID`
- Modify: `crates/shadows-core/src/threads/{mod.rs,model.rs,store}`:
  - `create` takes `profile: Option<&str>`, which is required for harness `aylul` and refused otherwise
  - `PlanningThread.profile: Option<String>`, with no way to change it
  - `list_for_profile(slug)`, newest first by `created_at`, then `id`
- Modify: `crates/shadows-core/src/secrets`, so provider keys work:
  - `set_provider_key(command_id, provider, value)`
  - `provider_key_set(provider) -> bool`
  - `remove_provider_key`
  - all of them use the scope `provider/<slug>`
- Modify: `docs/codebase/README.md`, `docs/superpowers/specs/2026-09-26-application-core-design.md` §14.4 (twelve services), and CLAUDE.md's service count
- Test: `crates/shadows-core/tests/aylul_profiles.rs`, and `threads` tests

**Interfaces (produced):**
- `pub struct Provider { slug, name, base_url, models: Vec<String>, context_window: u32, key_set: bool, created_at, updated_at }`
- `pub struct ProviderInput { name, base_url, models: Vec<String>, context_window: Option<u32> }`
- `pub struct Agent { slug, name, provider, default_model, created_at, updated_at }`
- `pub struct AgentInput { name, provider, default_model }`
- `impl Profiles`:
  - `providers() -> Vec<Provider>` and `agents() -> Vec<Agent>`, both ordered by `name`, then `slug`
  - `provider(slug)` and `agent(slug)`
  - `create_provider(command_id, ProviderInput)`, `update_provider(command_id, slug, ProviderInput)` and `delete_provider(command_id, slug)`
  - `create_agent(command_id, AgentInput)`, `update_agent(command_id, slug, AgentInput)` and `delete_agent(command_id, slug)`
  - Deleting removes that scope's `secret_env` rows in the same transaction.
  - All return `Result<_, CoreError>`.

- [ ] **Step 1: Write the failing tests.**
  - `provider_slug_from_name_and_dash_two`
  - `invalid_provider_input_is_refused` (no models, 21, a duplicate, a space, a bad URL, `context_window` out of range)
  - `agent_default_must_be_a_provider_model`
  - `provider_in_use_cannot_be_deleted`
  - `removing_an_agents_default_model_is_refused`
  - `delete_agent_removes_its_secrets`
  - `delete_provider_removes_its_key`
  - `provider_key_is_write_only` (`key_set` flips; no method returns the value)
  - `aylul_thread_requires_an_agent`
  - `planner_thread_refuses_an_agent`
  - `threads_list_by_profile_only`
  - `replay_with_other_input_conflicts`
- [ ] **Step 2:** Run `cargo test -p shadows-core --test aylul_profiles`. Expected: FAIL.
- [ ] **Step 3:** Implement. The contract lists the methods and their rules.
- [ ] **Step 4:** Run the same tests, plus `cargo test -p shadows-core --test storage_contract`, `cargo test -p shadows --test codemap` and `cargo test -p shadows-core --test contracts`. Expected: PASS.
- [ ] **Step 5:** Commit `feat(core): AYLUL providers and agents; threads carry their agent`.

### Task 2: Opening by agent, model allowlist, `aylul.json` shrinks

**Files:**
- Modify: `crates/shadows-core/src/harness/assistant.rs`. `opening` reads the thread's agent, then its provider, then the provider key and the agent's variables. It builds the `ClaudeProfile` at spawn:
  - home `<home_root>/profiles/<agent slug>`
  - `ANTHROPIC_BASE_URL` set to the provider URL
  - `ANTHROPIC_MODEL` and the three `ANTHROPIC_DEFAULT_*_MODEL` set to `default_model`
  - the context-window pair set to the provider's `context_window`
  - the provider key as the token
- Create: `crates/shadows-core/src/harness/profile_home.rs`, with:
  - `seed(home)`: the persona and settings templates move here from the binary; only absent files are written; it is called when an agent is created
  - `write_models(home, models, default)`: reads `settings.json` as a `serde_json::Map`, sets the two keys and writes it back atomically (temp file + rename)
- Modify: `crates/shadows-agent/src/profile.rs`: remove `key_env` and the daemon fallback. The token is passed in explicitly.
- Modify: `crates/shadows-core/src/harness/*`, where AYLUL's `SessionChoices` are built: drop the model `default`. Remembered settings use `aylul:<slug>`.
- Modify: `crates/shadows/src/aylul.rs`:
  - `AylulSection` becomes `{ home_root }` only, with `deny_unknown_fields`
  - remove the provider env, `env_file`, `import_env` and `parse_dotenv`
  - `AylulConfig` holds `home_root`
- Modify: `crates/shadows/src/cli/mod.rs` and `daemon/mod.rs` (no import call)
- Modify: `crates/shadows-core/src/secrets` (drop `import_env_once` and `import_needed`; the marker table stays, unused, because migrations never drop)
- Modify: contracts and code-map rows
- Test:
  - `aylul_session.rs`:
    - `opening_uses_agent_and_provider`
    - `two_agents_share_one_provider_key`
    - `deleted_agent_is_profile_not_found`
    - `missing_provider_key_is_named`
    - `settings_json_keeps_other_keys`
    - `non_object_settings_json_fails_named`
    - `aylul_choices_have_no_default_model`
    - `remembered_model_is_per_agent`
  - `profile.rs`: unit tests for the token rule
  - `aylul.rs`: `old_section_keys_are_refused`

- [ ] **Step 1: Probe first.** The plan depends on it. Run the real adapter once on a temp home whose `settings.json` has `"availableModels": ["MiniMax-M3", "MiniMax-M2.7"]`, with a fake key and `settingSources ["user"]`. Record its `configOptions` model values in `docs/evidence/aylul/2026-10-19-available-models-probe.md`. If it does not list exactly `default` plus those ids, stop and report; do not build on a guess.
- [ ] **Step 2:** Write the failing tests listed above.
- [ ] **Step 3:** Run `cargo test -p shadows-core -p fake-acp --features fake-acp/test-support --test aylul_session`, `cargo test -p shadows-agent profile` and `cargo test -p shadows --lib aylul`. Expected: FAIL.
- [ ] **Step 4:** Implement.
- [ ] **Step 5:** Run the same tests. Expected: PASS.
- [ ] **Step 6:** Commit `feat(aylul): sessions open by agent on its provider's models`.

### Task 3: HTTP routes and the web

**Files:**
- Create: `crates/shadows-http/src/aylul_profiles.rs`. Each route calls one method and holds no rule:
  - `GET` and `POST /api/aylul/providers`; `PUT` and `DELETE /api/aylul/providers/{slug}`
  - `PUT` and `DELETE /api/aylul/providers/{slug}/key` (the body `{command_id, value}` is never echoed)
  - `GET` and `POST /api/aylul/agents`; `PUT` and `DELETE /api/aylul/agents/{slug}`
  - `GET /api/aylul/agents/{slug}/threads`
- Modify: `crates/shadows-http/src/aylul_env.rs`. The routes become `/api/aylul/agents/{slug}/env[/{name}]`, and `/api/aylul/env` is removed. Body-parse errors stay fixed text.
- Modify: the thread create route (`profile` field) and `openapi.rs`; regenerate `api/openapi.json` (`UPDATE_OPENAPI=1 cargo test -p shadows --test openapi`)
- Create:
  - `web/src/api/aylul-profiles.ts`: providers, agents, keys, env and threads. `aylul-env.ts` is removed.
  - `web/src/app/sidebar/workspace-switcher.tsx`: a title dropdown with label, description and check, holding "Shadows: Plan and build projects" and "AYLUL: Your agents and their conversations". The choice is kept in `localStorage` (inside try/catch), and a route under AYLUL wins over the stored choice.
  - `web/src/app/sidebar/agent-list.tsx`:
    - the agents, each expandable to its conversations like a project, with "New conversation" and "Agent settings"
    - a "Providers" link at the end
    - empty states "Add a provider first" and "Add an agent to start"
  - `web/src/app/agents/provider-settings.tsx`: the providers list and one form (name, URL, an "API key" password field, a models editor, context window, delete with confirmation)
  - `web/src/app/agents/agent-settings.tsx`: one form for "New agent" and "Agent settings" (name, a provider dropdown, a default-model dropdown with that provider's models, delete with confirmation), plus `agent-env.tsx` (the §10 env section, scoped to one agent)
- Modify:
  - `web/src/app/sidebar/sidebar.tsx`: the switcher goes in the header, with projects or agents below; in AYLUL, "New agent" replaces "New project"
  - remove `web/src/app/sidebar/aylul-entry.tsx` and its test, and `web/src/app/settings/aylul-env.tsx` from the global Settings page
  - `web/src/app/conversation/draft.tsx`: a new conversation under an agent creates its thread with that `profile`, and the model menu before the session exists shows the provider's models
  - `web/src/app/conversation/conversation.tsx`: the header names the agent
  - the routes `/aylul/providers`, `/aylul/agents/new`, `/aylul/agents/{slug}/settings` and `/aylul/agents/{slug}/new`
  - contracts, `docs/frontend/codebase/README.md`, and `schema.d.ts` (`gen:api`)
- Test:
  - Rust route tests:
    - `provider_crud_and_key_never_echoed`
    - `agent_crud`
    - `provider_in_use_is_409`
    - `env_routes_are_per_agent`
    - `thread_create_with_profile`
    - `threads_listed_per_agent`
  - Web:
    - `switcher_shows_both_with_check_on_current`
    - `switcher_choice_survives_reload`
    - `aylul_route_opens_aylul_workspace`
    - `shadows_workspace_has_no_aylul_entry`
    - `agent_list_shows_conversations_per_agent`
    - `provider_form_saves_and_clears_key`
    - `models_editor_rejects_duplicates`
    - `agent_form_lists_provider_models`
    - `new_conversation_under_agent_sets_profile`
    - `empty_states_guide_to_provider_then_agent`

- [ ] **Step 1:** Write the failing tests.
- [ ] **Step 2:** Run `cargo test -p shadows-http aylul` and `npm --prefix web test -- aylul agent provider switcher`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run the same tests, plus `cargo test -p shadows --test openapi` and `npm --prefix web run typecheck`. Expected: PASS.
- [ ] **Step 5:** Commit `feat(web): workspace switcher; AYLUL agents and providers`.

### Task 4: Gate (once)

- [ ] Run CI steps 1–6 from `CLAUDE.md`, the 100-column check and `web` gen:api/typecheck/lint/test/build. All must be green.
- [ ] Report the test counts and each file that passed 300 lines, with its one job. Do not push.

The controller runs the browser journey (spec §11 acceptance 1–5) on a copy of
the database and records it in `docs/evidence/aylul/`.
