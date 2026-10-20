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
Every task waits for the [request](../shared/requests/2026-10-20-settings-dialog.md)
to be `accepted`.

## Tasks

### B-1 · Provider presets and labels
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-settings-dialog-backend.md#b-1--provider-presets-and-labels
- Read: crates/shadows-core/src/profiles/contract.yaml; spec §1; docs/codebase/README.md rows for `profiles/`
- Touch: crates/shadows-core/src/profiles/presets.rs (new), crates/shadows-core/migrations/0030_provider_presets.sql (new), crates/shadows-core/src/profiles/mod.rs, crates/shadows-core/src/profiles/model.rs, crates/shadows-core/src/profiles/providers.rs, crates/shadows-core/src/profiles/store.rs, crates/shadows-core/src/profiles/contract.yaml, crates/shadows-core/src/lib.rs, crates/shadows-http/src/aylul_profiles.rs, crates/shadows-http/src/aylul_profiles_tests.rs, crates/shadows-http/src/lib.rs, api/openapi.json, docs/codebase/README.md, crates/shadows-core/tests/aylul_profiles/providers.rs, crates/shadows-core/tests/profile_migration.rs, crates/shadows-core/tests/aylul_profiles.rs, crates/shadows-core/tests/aylul_session.rs, crates/shadows-http/src/aylul_env_tests.rs, crates/shadows/tests/openapi/read_only.rs (each builds a `ProviderInput`)
- Needs: the request `accepted`
- Done when: `presets_list_in_fixed_order`, `preset_supplies_url_and_name`, `preset_with_other_url_is_refused`, `custom_requires_name_and_url`, `label_is_trimmed_and_empty_is_null`, `unknown_preset_is_refused`, `migration_marks_known_urls_with_their_preset`, `provider_presets_route_lists_five` pass; `cargo test -p shadows-core --test contracts`, `cargo test -p shadows --test codemap`, `cargo test -p shadows --test openapi` pass

### B-2 · Each model carries its context window
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-settings-dialog-backend.md#b-2--each-model-carries-its-context-window
- Read: crates/shadows-core/src/profiles/contract.yaml; crates/shadows-core/src/harness/contract.yaml; spec §2; `Sessions::opening` in crates/shadows-core/src/harness/assistant.rs
- Touch: crates/shadows-core/migrations/0031_model_windows.sql (new), crates/shadows-core/src/profiles/model.rs, crates/shadows-core/src/profiles/providers.rs, crates/shadows-core/src/profiles/agents.rs, crates/shadows-core/src/profiles/store.rs, crates/shadows-core/src/profiles/contract.yaml, crates/shadows-core/src/harness/assistant.rs, crates/shadows-core/src/harness/contract.yaml, crates/shadows-core/src/lib.rs, crates/shadows-http/src/aylul_profiles_tests.rs, api/openapi.json, crates/shadows-core/tests/aylul_profiles/providers.rs, crates/shadows-core/tests/aylul_profiles.rs, crates/shadows-core/tests/profile_migration.rs, crates/shadows-core/tests/aylul_session/opening.rs, crates/shadows-core/tests/aylul_session.rs, crates/shadows-http/src/aylul_env_tests.rs, crates/shadows/tests/openapi/read_only.rs (each builds a `ProviderInput`)
- Needs: B-1 done
- Done when: `model_window_defaults_and_is_bounded`, `model_ids_keep_their_rules`, `migration_gives_models_the_provider_window`, `opening_uses_the_default_models_window`, `auto_compact_window_is_capped_or_omitted`, `removing_an_agents_default_model_is_refused`, `provider_in_use_cannot_be_deleted` pass; contracts and openapi tests pass

### B-3 · Test a key and list its models
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-settings-dialog-backend.md#b-3--test-a-key-list-its-models
- Read: spec §3; docs/evidence/2026-10-21-provider-presets-research.md; crates/shadows-core/src/profiles/contract.yaml; crates/shadows-core/src/secrets/contract.yaml (`resolve_env`)
- Touch: crates/shadows-core/src/profiles/probe.rs (new), crates/shadows-core/src/profiles/listing.rs (new), crates/shadows-core/tests/aylul_profiles/probe.rs (new), crates/shadows-core/Cargo.toml, Cargo.toml (only the `reqwest` entry), Cargo.lock, crates/shadows-core/src/profiles/mod.rs, crates/shadows-core/src/profiles/presets.rs, crates/shadows-core/src/profiles/contract.yaml, crates/shadows-core/tests/aylul_profiles.rs, crates/shadows-core/src/lib.rs, crates/shadows-http/src/aylul_profiles.rs, crates/shadows-http/src/aylul_profiles_tests.rs, crates/shadows-http/src/lib.rs, api/openapi.json, docs/codebase/README.md
- Needs: B-2 done
- Done when: `tested_key_is_not_stored_or_echoed`, `refused_key_reads_key_refused`, `unreachable_host_reads_unreachable`, `odd_status_reads_unexpected`, `redirect_is_not_followed`, `oversized_body_is_unexpected`, `custom_without_model_list_is_ok_unlisted`, `anthropic_list_reads_max_input_tokens`, `minimax_list_has_no_windows`, `deepseek_list_reads_context_window`, `moonshot_list_reads_context_length`, `openrouter_list_keeps_tool_models`, `saved_test_uses_the_stored_key`, `provider_test_route_answers_outcome` pass; contracts, codemap, openapi pass; `cargo tree -e features,no-dev --workspace | grep test-support` prints nothing

### B-4 · Character colour on the agent
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-settings-dialog-backend.md#b-4--character-colour-on-the-agent
- Read: spec §4; crates/shadows-core/src/profiles/contract.yaml
- Touch: crates/shadows-core/migrations/0032_agent_color.sql (new), crates/shadows-core/src/profiles/model.rs, crates/shadows-core/src/profiles/agents.rs, crates/shadows-core/src/profiles/store.rs, crates/shadows-core/src/profiles/contract.yaml, crates/shadows-http/src/aylul_profiles_tests.rs, api/openapi.json, crates/shadows-core/tests/aylul_profiles/agents.rs, crates/shadows-core/tests/profile_migration.rs, crates/shadows-core/tests/aylul_profiles.rs, crates/shadows-core/tests/aylul_session.rs, crates/shadows-http/src/aylul_env_tests.rs, crates/shadows/tests/openapi/read_only.rs (each builds an `AgentInput`)
- Needs: the request `accepted` (independent of B-1..B-3; migration number follows whatever is merged first)
- Done when: `color_id_shape_is_checked`, `unknown_but_well_formed_color_is_kept`, `color_defaults_to_purple`, `update_without_color_keeps_it`, `migration_colors_agents_purple` pass; contracts and openapi pass

### B-5 · The person (name, what AYLUL calls them, language, theme)
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-settings-dialog-backend.md#b-5--the-person
- Read: spec §5; crates/shadows-core/src/profiles/contract.yaml; crates/shadows-core/src/harness/contract.yaml; `aylul_setup` in crates/shadows-core/src/harness/assistant.rs
- Touch: crates/shadows-core/src/profiles/person.rs (new), crates/shadows-core/migrations/0033_aylul_person.sql (new), crates/shadows-core/tests/aylul_profiles/person.rs (new), crates/shadows-core/src/profiles/mod.rs, crates/shadows-core/src/profiles/model.rs, crates/shadows-core/src/profiles/store.rs, crates/shadows-core/src/profiles/contract.yaml, crates/shadows-core/src/harness/assistant.rs, crates/shadows-core/src/harness/contract.yaml, crates/shadows-core/tests/aylul_profiles.rs, crates/shadows-core/tests/aylul_session/opening.rs, crates/shadows-core/src/lib.rs, crates/shadows-http/src/aylul_profiles.rs, crates/shadows-http/src/aylul_profiles_tests.rs, crates/shadows-http/src/lib.rs, api/openapi.json, docs/codebase/README.md
- Needs: the request `accepted`; Mohammed's answer on language/theme living in Core (report question 2)
- Done when: `person_names_are_checked`, `language_tag_shape_is_checked`, `theme_values_are_checked`, `person_replay_with_other_input_conflicts`, `opening_names_the_person`, `opening_without_person_appends_nothing`, `me_round_trips` pass; contracts, codemap, openapi pass

### B-6 · Probe: what the ACP adapter reports of usage
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-settings-dialog-backend.md#b-6--probe-what-the-adapter-reports-of-usage
- Read: spec §6; docs/evidence/2026-10-21-provider-presets-research.md ("What Shadows itself can see of usage")
- Touch: Cargo.toml (probe commit only, reverted), crates/shadows-agent/src/acp.rs (probe commit only, reverted), docs/evidence/<date>-acp-usage-probe.md (new)
- Needs: Mohammed to run the probe turns with his own keys
- Done when: the evidence file records the raw `PromptResponse.usage` and `UsageUpdate.cost` for two providers, the probe is reverted, and spec §6's OPEN is closed

### B-7 · Record and report usage
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-settings-dialog-backend.md#b-7--record-and-report-usage
- Read: spec §6 after B-6
- Touch: written after B-6
- Needs: B-6 done; Mohammed's answer on cost for non-Anthropic providers (report question 3)
- Done when: written after B-6

### B-8 · Probe: skills and plugins under an isolated agent home
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-settings-dialog-backend.md#b-8--probe-skills-and-plugins-under-an-isolated-home
- Read: spec §7
- Touch: docs/evidence/<date>-agent-home-extensions-probe.md (new); probe code reverted
- Needs: Mohammed's ruling on spec §7.1's scope (report question 4)
- Done when: the evidence file says whether a skill and a local plugin load in an ACP session under `CLAUDE_CONFIG_DIR`, and spec §7's OPEN is closed

### B-9 · Plugins and skills library
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-settings-dialog-backend.md#b-9--plugins-and-skills-library
- Read: spec §7 after B-8
- Touch: written after B-8
- Needs: B-8 done
- Done when: written after B-8

### B-A2 · Prepare Core's scoped context contract
- Status: stopped (2026-10-19: its Hermes integration plan was deleted, Git `226d0a3`; AYLUL's work profile will define its own need)
- Plan: none
- Read: projects/threads/plans/design contracts; architecture section 1.7; approval section 24.6; integration T3.
- Touch: the required shared request/owner specification first; implementation paths only after that contract is accepted.
- Needs: accepted caller/sharing/authority contract and counterpart native/UI request.
- Done when: the scoped projection and its refusal/revocation/identity tests are agreed without assigning AYLUL or LSP runtime ownership to AppCore.
