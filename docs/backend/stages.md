# Core backend stages

This file tracks the existing Shadows Core backend only. See
[the documentation map](../README.md) for AYLUL and independent service work.

## Now

No Core product implementation is in progress. AYLUL's brain will run on Core
inside the same daemon ([brain design](../aylul/specs/2026-10-19-aylul-claude-code-brain-design.md));
its plan comes next.
The independent service baseline is tracked in
[code-intelligence stages](../code-intelligence/stages.md), not as a Core task.
Existing AppCore remains the composition root for this runtime only.

Next: the settings dialog's backend
([spec](specs/2026-10-21-settings-dialog-backend-design.md),
[plan](plans/2026-10-21-settings-dialog-backend.md)), on branch `improvements`.
Every task that changes the API waits for the [request](../shared/requests/2026-10-20-settings-dialog.md)
to be `accepted`; the two probes (B-6, B-8) change nothing the frontend sees and may run now.
Live keys: only OpenRouter and MiniMax, in `.env` (read its variable names, never its values;
never print, log or commit a key).

## Tasks

### B-1 · Provider presets and labels
- Status: done
- Plan: docs/backend/plans/2026-10-21-settings-dialog-backend.md#b-1--provider-presets-and-labels
- Read: crates/shadows-core/src/profiles/contract.yaml; spec §1; docs/codebase/README.md rows for `profiles/`
- Touch: crates/shadows-core/src/profiles/presets.rs (new), crates/shadows-core/migrations/0030_provider_presets.sql (new), crates/shadows-core/src/profiles/mod.rs, crates/shadows-core/src/profiles/model.rs, crates/shadows-core/src/profiles/providers.rs, crates/shadows-core/src/profiles/store.rs, crates/shadows-core/src/profiles/contract.yaml, crates/shadows-core/src/lib.rs, crates/shadows-http/src/aylul_profiles.rs, crates/shadows-http/src/aylul_profiles_tests.rs, crates/shadows-http/src/lib.rs, api/openapi.json, docs/codebase/README.md, crates/shadows-core/tests/aylul_profiles/providers.rs, crates/shadows-core/tests/profile_migration.rs, crates/shadows-core/tests/aylul_profiles.rs, crates/shadows-core/tests/aylul_session.rs, crates/shadows-http/src/aylul_env_tests.rs, crates/shadows/tests/openapi/read_only.rs (each builds a `ProviderInput`)
- Needs: the request `accepted`
- Done when: `presets_list_in_fixed_order`, `preset_supplies_url_and_name`, `preset_with_other_url_is_refused`, `custom_requires_name_and_url`, `label_is_trimmed_and_empty_is_null`, `unknown_preset_is_refused`, `china_presets_use_china_hosts`, `migration_marks_known_urls_with_their_preset`, `provider_presets_route_lists_all` pass; `cargo test -p shadows-core --test contracts`, `cargo test -p shadows --test codemap`, `cargo test -p shadows --test openapi` pass

### B-2 · Each model carries its context window
- Status: done
- Plan: docs/backend/plans/2026-10-21-settings-dialog-backend.md#b-2--each-model-carries-its-context-window
- Read: crates/shadows-core/src/profiles/contract.yaml; crates/shadows-core/src/harness/contract.yaml; spec §2; `Sessions::opening` in crates/shadows-core/src/harness/assistant.rs
- Touch: crates/shadows-core/migrations/0031_model_windows.sql (new), crates/shadows-core/src/profiles/model.rs, crates/shadows-core/src/profiles/providers.rs, crates/shadows-core/src/profiles/agents.rs, crates/shadows-core/src/profiles/store.rs, crates/shadows-core/src/profiles/contract.yaml, crates/shadows-core/src/harness/assistant.rs, crates/shadows-core/src/harness/contract.yaml, crates/shadows-core/src/lib.rs, crates/shadows-http/src/aylul_profiles_tests.rs, api/openapi.json, crates/shadows-core/tests/aylul_profiles/providers.rs, crates/shadows-core/tests/aylul_profiles.rs, crates/shadows-core/tests/profile_migration.rs, crates/shadows-core/tests/aylul_session/opening.rs, crates/shadows-core/tests/aylul_session.rs, crates/shadows-http/src/aylul_env_tests.rs, crates/shadows/tests/openapi/read_only.rs (each builds a `ProviderInput`)
- Needs: B-1 done
- Done when: `model_window_defaults_and_is_bounded`, `model_ids_keep_their_rules`, `migration_gives_models_the_provider_window`, `opening_uses_the_default_models_window`, `auto_compact_window_is_capped_or_omitted`, `removing_an_agents_default_model_is_refused`, `provider_in_use_cannot_be_deleted` pass; contracts and openapi tests pass

### B-3 · Test a key and list its models
- Status: done
- Plan: docs/backend/plans/2026-10-21-settings-dialog-backend.md#b-3--test-a-key-list-its-models
- Read: spec §3; docs/evidence/2026-10-21-provider-presets-research.md; crates/shadows-core/src/profiles/contract.yaml; crates/shadows-core/src/secrets/contract.yaml (`resolve_env`)
- Touch: crates/shadows-core/src/profiles/probe.rs (new), crates/shadows-core/src/profiles/listing.rs (new), crates/shadows-core/tests/aylul_profiles/probe.rs (new), crates/shadows-core/Cargo.toml, Cargo.toml (only the `reqwest` entry), Cargo.lock, crates/shadows-core/src/profiles/mod.rs, crates/shadows-core/src/profiles/presets.rs, crates/shadows-core/src/profiles/contract.yaml, crates/shadows-core/tests/aylul_profiles.rs, crates/shadows-core/src/lib.rs, crates/shadows-core/src/harness/mod.rs, crates/shadows-core/src/harness/contract.yaml, crates/shadows-http/src/aylul_profiles.rs, crates/shadows-http/src/aylul_profiles_tests.rs, crates/shadows-http/src/lib.rs, api/openapi.json, docs/codebase/README.md
- Needs: B-2 done
- Done when: `tested_key_is_not_stored_or_echoed`, `refused_key_reads_key_refused`, `unreachable_host_reads_unreachable`, `odd_status_reads_unexpected`, `redirect_is_not_followed`, `oversized_body_is_unexpected`, `custom_without_model_list_is_ok_unlisted`, `anthropic_list_reads_max_input_tokens`, `minimax_list_has_no_windows`, `deepseek_list_reads_context_window`, `moonshot_list_reads_context_length`, `openrouter_list_keeps_tool_models`, `saved_test_uses_the_stored_key`, `provider_test_route_answers_outcome` pass; contracts, codemap, openapi pass; `cargo tree -e features,no-dev --workspace | grep test-support` prints nothing; acceptance: the ignored `live_keys_list_models` passes for `openrouter` and `minimax` with the keys from `.env` loaded into that one command's environment, printing only preset, `ok`, `reason` and the model count. Anthropic, DeepSeek, Moonshot and both China presets get no live test until Mohammed has their keys; their acceptance is the fixture tests only

### B-4 · Character colour on the agent
- Status: done
- Plan: docs/backend/plans/2026-10-21-settings-dialog-backend.md#b-4--character-colour-on-the-agent
- Read: spec §4; crates/shadows-core/src/profiles/contract.yaml
- Touch: crates/shadows-core/migrations/0032_agent_color.sql (new), crates/shadows-core/src/profiles/model.rs, crates/shadows-core/src/profiles/agents.rs, crates/shadows-core/src/profiles/store.rs, crates/shadows-core/src/profiles/contract.yaml, crates/shadows-http/src/aylul_profiles_tests.rs, api/openapi.json, crates/shadows-core/tests/aylul_profiles/agents.rs, crates/shadows-core/tests/profile_migration.rs, crates/shadows-core/tests/aylul_profiles.rs, crates/shadows-core/tests/aylul_session.rs, crates/shadows-http/src/aylul_env_tests.rs, crates/shadows/tests/openapi/read_only.rs (each builds an `AgentInput`)
- Needs: the request `accepted` (independent of B-1..B-3; migration number follows whatever is merged first)
- Done when: `color_id_shape_is_checked`, `unknown_but_well_formed_color_is_kept`, `color_defaults_to_purple`, `update_without_color_keeps_it`, `migration_colors_agents_purple` pass; contracts and openapi pass

### B-5 · The person (name, what AYLUL calls them, language, theme)
- Status: done
- Plan: docs/backend/plans/2026-10-21-settings-dialog-backend.md#b-5--the-person
- Read: spec §5; crates/shadows-core/src/profiles/contract.yaml; crates/shadows-core/src/harness/contract.yaml; `aylul_setup` in crates/shadows-core/src/harness/assistant.rs
- Touch: crates/shadows-core/src/profiles/person.rs (new), crates/shadows-core/migrations/0033_aylul_person.sql (new), crates/shadows-core/tests/aylul_profiles/person.rs (new), crates/shadows-core/src/profiles/mod.rs, crates/shadows-core/src/profiles/model.rs, crates/shadows-core/src/profiles/store.rs, crates/shadows-core/src/profiles/contract.yaml, crates/shadows-core/src/harness/assistant.rs, crates/shadows-core/src/harness/contract.yaml, crates/shadows-core/tests/aylul_profiles.rs, crates/shadows-core/tests/aylul_session/opening.rs, crates/shadows-core/src/lib.rs, crates/shadows-http/src/aylul_profiles.rs, crates/shadows-http/src/aylul_profiles_tests.rs, crates/shadows-http/src/lib.rs, api/openapi.json, docs/codebase/README.md
- Needs: the request `accepted` (Mohammed ruled 2026-10-21: language and theme live in Core, shared by browser and desktop)
- Done when: `person_names_are_checked`, `language_tag_shape_is_checked`, `theme_values_are_checked`, `person_replay_with_other_input_conflicts`, `opening_names_the_person`, `opening_without_person_appends_nothing`, `me_round_trips` pass; contracts, codemap, openapi pass

### B-6 · Probe: what the ACP adapter reports of usage
- Status: done
- Plan: docs/backend/plans/2026-10-21-settings-dialog-backend.md#b-6--probe-what-the-adapter-reports-of-usage
- Read: spec §6; docs/evidence/2026-10-21-provider-presets-research.md ("What Shadows itself can see of usage")
- Touch: Cargo.toml (probe commit only, reverted), crates/shadows-agent/src/acp.rs (probe commit only, reverted), docs/evidence/<date>-acp-usage-probe.md (new)
- Needs: nothing; uses the OpenRouter and MiniMax keys in `.env` (names only read; never printed or committed)
- Done when: the evidence file records the raw `PromptResponse.usage` and `UsageUpdate.cost` (no key, no prompt text) for one turn on OpenRouter and one on MiniMax, the probe is reverted, and spec §6's OPEN is closed

### B-7 · Record and report usage
- Status: done
- Reviewed: steps released by the controller, 2026-10-21
- Plan: docs/backend/plans/2026-10-21-settings-dialog-backend.md#b-7--record-and-report-usage
- Read: spec section 6; docs/evidence/2026-10-21-acp-usage-probe.md; crates/shadows-core/src/profiles/contract.yaml; crates/shadows-core/src/turns/contract.yaml; crates/shadows-core/src/harness/contract.yaml; docs/codebase/README.md usage/turn/harness owners
- Touch: Cargo.toml, crates/shadows-agent/src/acp.rs, crates/shadows-agent/src/events.rs, crates/fake-acp/src/main.rs, crates/fake-acp/src/prompts.rs, crates/fake-acp/src/session.rs, crates/shadows-core/migrations/0034_aylul_usage.sql (new), crates/shadows-core/src/profiles/usage.rs (new), crates/shadows-core/src/profiles/usage_store.rs (new), crates/shadows-core/src/profiles/mod.rs, crates/shadows-core/src/profiles/contract.yaml, crates/shadows-core/src/turns/store/usage.rs (new), crates/shadows-core/src/turns/store/mod.rs, crates/shadows-core/src/turns/store/turn.rs, crates/shadows-core/src/turns/store/operation.rs, crates/shadows-core/src/turns/turn.rs, crates/shadows-core/src/turns/record.rs, crates/shadows-core/src/turns/contract.yaml, crates/shadows-core/src/harness/assistant.rs, crates/shadows-core/src/harness/live.rs, crates/shadows-core/src/harness/sessions.rs, crates/shadows-core/src/harness/model.rs, crates/shadows-core/src/harness/contract.yaml, crates/shadows-core/src/lib.rs, crates/shadows-core/tests/aylul_usage.rs (new), crates/shadows-core/tests/acp_connection.rs, crates/shadows-http/src/aylul_profiles.rs, crates/shadows-http/src/aylul_profiles_tests.rs, crates/shadows-http/src/lib.rs, crates/shadows/tests/openapi.rs, api/openapi.json, docs/codebase/README.md, docs/backend/stages.md
- Needs: B-6 done; the request `accepted`
- Done when: prompt_returns_raw_turn_usage, usage_counts_cache_once, usage_ends_atomically_once, usage_snapshot_survives_provider_changes, usage_refusal_cancel_and_missing_are_honest, usage_costs_require_trusted_turn_delta, usage_periods_group_and_sort, usage_route_is_read_only pass; touched turn/cancellation tests, contracts, codemap, generated OpenAPI, fmt and focused clippy pass; controller review releases blocker

### B-8 · Probe: skills and plugins under an isolated agent home
- Status: done
- Plan: docs/backend/plans/2026-10-21-settings-dialog-backend.md#b-8--probe-skills-and-plugins-under-an-isolated-home
- Read: spec §7
- Touch: docs/evidence/<date>-agent-home-extensions-probe.md (new); probe code reverted
- Needs: nothing (scope ruled 2026-10-21: one shared library, enabled per agent); a MiniMax or OpenRouter key from `.env` for the session
- Done when: the evidence file says whether a skill and a local plugin load in an ACP session under `CLAUDE_CONFIG_DIR`, and whether one agent's home can enable an item another agent's home does not, and spec §7's OPEN is closed

### B-9 · Plugins and skills: shared library, enabled per agent
- Status: done
- Reviewed: steps released by the controller, 2026-10-21
- Plan: docs/backend/plans/2026-10-21-settings-dialog-backend.md#b-9--plugins-and-skills-shared-library-enabled-per-agent
- Read: spec section 7; docs/evidence/2026-10-21-agent-home-extensions-probe.md; crates/shadows-core/src/profiles/contract.yaml; crates/shadows-core/src/harness/contract.yaml; crates/shadows-core/src/secrets/contract.yaml; docs/codebase/README.md profile/home owners
- Touch: crates/shadows-core/migrations/0035_aylul_extensions.sql (new), crates/shadows-core/src/profiles/extensions.rs (new), crates/shadows-core/src/profiles/extension_model.rs (new), crates/shadows-core/src/profiles/extension_store.rs (new), crates/shadows-core/src/profiles/extension_files.rs (new), crates/shadows-core/src/profiles/mod.rs, crates/shadows-core/src/profiles/store.rs, crates/shadows-core/src/profiles/contract.yaml, crates/shadows-core/src/harness/extension_home.rs (new), crates/shadows-core/src/harness/mod.rs, crates/shadows-core/src/harness/assistant.rs, crates/shadows-core/src/harness/profile_home.rs, crates/shadows-core/src/harness/contract.yaml, crates/shadows-core/src/lib.rs, crates/shadows-core/src/error.rs, crates/shadows-core/src/db/error.rs, crates/shadows-http/src/failure/other.rs, crates/shadows-http/src/failure/storage.rs, crates/shadows-core/tests/aylul_extensions.rs (new), crates/shadows-core/tests/aylul_extensions/fixture.rs (new), crates/shadows-core/tests/aylul_extensions/home.rs (new), crates/shadows-core/tests/aylul_extensions/live.rs (new), crates/shadows-core/tests/aylul_session/homes.rs, crates/shadows-core/tests/aylul_session/opening.rs, crates/shadows-http/src/aylul_extensions.rs (new), crates/shadows-http/src/aylul_extensions_tests.rs (new), crates/shadows-http/src/lib.rs, crates/shadows-mcp/src/refusal.rs, crates/shadows/tests/openapi.rs, api/openapi.json, docs/codebase/README.md, docs/backend/stages.md
- Needs: B-8 done; B-7 migration allocation; the request `accepted`
- Done when: extension_import_copies_and_redacts_source, extension_input_is_contained, extension_commands_replay_and_conflict, extension_enablement_is_per_agent, extension_deletions_remove_enablement_atomically, opening_projects_only_enabled_extensions, opening_removes_disabled_cached_extensions, extension_projection_is_atomic, extension_routes_round_trip and ignored live_agent_extension_isolation pass; contracts, codemap, generated OpenAPI, fmt and focused clippy pass; controller review releases blocker

### B-A2 · Prepare Core's scoped context contract
- Status: stopped (2026-10-19: its Hermes integration plan was deleted, Git `226d0a3`; AYLUL's work profile will define its own need)
- Plan: none
- Read: projects/threads/plans/design contracts; architecture section 1.7; approval section 24.6; integration T3.
- Touch: the required shared request/owner specification first; implementation paths only after that contract is accepted.
- Needs: accepted caller/sharing/authority contract and counterpart native/UI request.
- Done when: the scoped projection and its refusal/revocation/identity tests are agreed without assigning AYLUL or LSP runtime ownership to AppCore.
