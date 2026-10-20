# Settings dialog backend plan

> **For builders:** each task is a `B-<n>` in [`../stages.md`](../stages.md). A
> `backend-builder` builds one task; one `backend-fixer` reviews the branch
> before the PR. Every task below changes what the frontend sees, so none starts
> before the [request](../../shared/requests/2026-10-20-settings-dialog.md) is
> `accepted`.

**Spec:** [`../specs/2026-10-21-settings-dialog-backend-design.md`](../specs/2026-10-21-settings-dialog-backend-design.md)
(sections cited as §n).

**Order:** what the Providers section needs first (B-1 presets and label, B-2
per-model windows, B-3 key test and model list), then the agent colour (B-4)
and the person (B-5). Usage (B-6, B-7) and plugins and skills (B-8, B-9) wait
on probes and a ruling.

## Global constraints

- `profiles` is the one service for every method here; an HTTP route reads the
  request, calls one method, shapes its answer (CLAUDE.md, adapters).
- Each task updates `crates/shadows-core/src/profiles/contract.yaml` (and any
  other service's contract it touches) in the same commit, in the shape of
  `docs/codebase/contracts/TEMPLATE.yaml`; a new file gets its row in
  `docs/codebase/README.md` (one job, no "and").
- A route change regenerates `api/openapi.json`:
  `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi`.
- Run only the tests the task names. `git add` each file.
- No API key appears in a test, a fixture, a log line or an error text. Tests
  use the synthetic value `test-key-not-real`.

---

## B-1 · Provider presets and labels

**Files**
- Create: `crates/shadows-core/src/profiles/presets.rs` (the compiled preset
  table), `crates/shadows-core/migrations/0030_provider_presets.sql`
- Modify: `crates/shadows-core/src/profiles/{mod.rs,model.rs,providers.rs,store.rs,contract.yaml}`,
  `crates/shadows-core/src/lib.rs` (re-export `ProviderPreset`),
  `crates/shadows-http/src/{aylul_profiles.rs,aylul_profiles_tests.rs,lib.rs}`,
  `api/openapi.json`, `docs/codebase/README.md`
- Test: `crates/shadows-core/tests/aylul_profiles/providers.rs`,
  `crates/shadows-core/tests/profile_migration.rs`

**Shapes**
- `ProviderPreset { id: String, name: String, base_url: String }` (the listing
  details of §3.1 stay private to `presets.rs`).
- `Provider` gains `preset: Option<String>`, `label: Option<String>`.
- `ProviderInput`: `name: Option<String>`, `base_url: Option<String>`,
  `preset: Option<String>`, `label: Option<String>` (models unchanged in B-1).

**Migration 0030:** `ALTER TABLE aylul_provider ADD COLUMN preset TEXT;`
`ADD COLUMN label TEXT;` then `UPDATE ... SET preset = '<id>' WHERE base_url = '<url>'`
for each of the five rows of §1.1.

**Route**
- `GET /api/aylul/provider-presets` → `200 [ProviderPreset]`, in §1.1's order.
  Tag `aylul`. No body, no command.

**Contract lines (`profiles/contract.yaml`)**
- shapes: `ProviderPreset: { source: presets.rs, note: compiled in; no row }`;
  `Provider.preset`, `Provider.label`.
- functions: `provider_presets: List the compiled presets in their fixed order.`
- obligations:
  - rule: A preset supplies the provider's base URL and name; a differing sent value is refused.
    tested_by: [preset_supplies_url_and_name, preset_with_other_url_is_refused]
  - rule: Custom requires a name and a URL passing the URL rule.
    tested_by: [custom_requires_name_and_url]
  - rule: A label is optional, trimmed, at most 64 characters; empty is null.
    tested_by: [label_is_trimmed_and_empty_is_null]
  - rule: An unknown preset id is refused.
    tested_by: [unknown_preset_is_refused]
- tests: `presets_list_in_fixed_order`, `preset_supplies_url_and_name`,
  `preset_with_other_url_is_refused`, `custom_requires_name_and_url`,
  `label_is_trimmed_and_empty_is_null`, `unknown_preset_is_refused`,
  `migration_marks_known_urls_with_their_preset` (in `profile_migration.rs`),
  HTTP: `provider_presets_route_lists_five` (in `aylul_profiles_tests.rs`).

**Done when:** the eight tests pass:
`cargo test -p shadows-core --test aylul_profiles`,
`cargo test -p shadows-core --test profile_migration`,
`cargo test -p shadows-http aylul_profiles`,
`cargo test -p shadows-core --test contracts`, `cargo test -p shadows --test codemap`,
`cargo test -p shadows --test openapi`.

## B-2 · Each model carries its context window

**Files**
- Create: `crates/shadows-core/migrations/0031_model_windows.sql`
- Modify: `crates/shadows-core/src/profiles/{model.rs,providers.rs,agents.rs,store.rs,contract.yaml}`,
  `crates/shadows-core/src/harness/{assistant.rs,contract.yaml}`,
  `crates/shadows-core/src/lib.rs` (re-export `ProviderModel`, `ProviderModelInput`),
  `crates/shadows-http/src/aylul_profiles_tests.rs`, `api/openapi.json`
- Test: `crates/shadows-core/tests/aylul_profiles/providers.rs`,
  `crates/shadows-core/tests/profile_migration.rs`,
  `crates/shadows-core/tests/aylul_session/opening.rs`

**Shapes**
- `ProviderModel { id: String, context_window: u32 }`;
  `ProviderModelInput { id: String, context_window: Option<u32> }`.
- `Provider.models: Vec<ProviderModel>`; `ProviderInput.models: Vec<ProviderModelInput>`.
  `context_window` leaves `Provider` and `ProviderInput`.

**Migration 0031:** rewrite `aylul_provider.models` from a JSON array of ids to a
JSON array of `{ "id", "context_window" }` using the row's `context_window`
(SQLite `json_each` / `json_group_array`, ordered by the array index `key`),
then drop the `context_window` column (or leave it unused if the table
rebuild is costlier; say which in the commit).

**Opening (§2.3):** `CLAUDE_CODE_MAX_CONTEXT_TOKENS` = the default model's
window; `CLAUDE_CODE_AUTO_COMPACT_WINDOW` = `min(window, 1_000_000)`, omitted
when the window is below 100,000. `write_models` still receives ids.

**Contract lines**
- `profiles`: shapes `ProviderModel`, `ProviderModelInput`; obligations:
  - rule: Each model's window defaults to 200,000 and must be 1,000..=10,000,000.
    tested_by: [model_window_defaults_and_is_bounded]
  - rule: Model ids stay unique, 1..=20 per provider, and never `default`.
    tested_by: [model_ids_keep_their_rules]
  - rule: Migration gives every existing model its provider's window, in order.
    tested_by: [migration_gives_models_the_provider_window]
- `harness`: obligation:
  - rule: An AYLUL opening sets the context variables from the default model's window; the auto-compact window stays inside 100,000..=1,000,000.
    tested_by: [opening_uses_the_default_models_window, auto_compact_window_is_capped_or_omitted]

**Done when:** those five tests pass, plus today's `removing_an_agents_default_model_is_refused`,
`provider_in_use_cannot_be_deleted`, the contracts, codemap and openapi tests.

## B-3 · Test a key, list its models

**Files**
- Create: `crates/shadows-core/src/profiles/probe.rs` (the outbound listing
  requests), `crates/shadows-core/src/profiles/listing.rs` (reading the five
  answer formats), `crates/shadows-core/tests/aylul_profiles/probe.rs`
- Modify: `crates/shadows-core/Cargo.toml` (`reqwest` with its rustls TLS
  feature: infrastructure), `Cargo.toml` only if the workspace entry must gain
  the TLS feature, `crates/shadows-core/src/profiles/{mod.rs,presets.rs,contract.yaml}`,
  `crates/shadows-core/tests/aylul_profiles.rs` (add `mod probe;`),
  `crates/shadows-core/src/lib.rs`,
  `crates/shadows-http/src/{aylul_profiles.rs,aylul_profiles_tests.rs,lib.rs}`,
  `api/openapi.json`, `docs/codebase/README.md`

**Shapes**
- `ProviderTest { preset: Option<String>, base_url: Option<String>, api_key: String }`
  (exactly one of `preset` / `base_url`).
- `SavedProviderTest { api_key: Option<String> }` (absent: the stored key).
- `ProviderTestOutcome { ok: bool, models: Vec<ListedModel>, listed: bool,
  reason: Option<TestRefusal>, status: Option<u16>, message: Option<String> }`;
  `ListedModel { id: String, context_window: Option<u32> }`;
  `TestRefusal = key_refused | unreachable | unexpected` (snake_case).
- The `api_key` field's `Debug` is redacted; the request type is never logged.

**Routes**
- `POST /api/aylul/providers/test`, body `ProviderTest` → `200 ProviderTestOutcome`;
  `400` for a bad body, an unknown preset, both or neither of `preset`/`base_url`,
  a URL failing the rule, or an empty key.
- `POST /api/aylul/providers/{slug}/test`, body `SavedProviderTest` →
  `200 ProviderTestOutcome`; `404` unknown slug; `409` (`AylulKeyMissing`) when
  no key is given and none is stored.
- Neither route writes a row, a command or an event.

**Methods:** `test_provider(ProviderTest)`, `test_saved_provider(slug, Option<String>)`.
Tests point a Custom `base_url` at a local `tokio` listener on `127.0.0.1` that
answers fixed bodies (the loopback http rule allows it); the preset URLs are
covered by `listing.rs` unit tests over fixture bodies written from §3.1.

**Contract lines**
- functions: `test_provider: Ask the provider for the key's models without storing the key.`,
  `test_saved_provider: Same, with the stored key when none is given.`
- obligations:
  - rule: The key is never stored, logged or returned, and appears in no outcome text.
    tested_by: [tested_key_is_not_stored_or_echoed]
  - rule: 401 or 403 is key_refused; connection failure, timeout or 5xx is unreachable; anything else is unexpected.
    tested_by: [refused_key_reads_key_refused, unreachable_host_reads_unreachable, odd_status_reads_unexpected]
  - rule: A redirect is not followed.
    tested_by: [redirect_is_not_followed]
  - rule: A body over 8 MiB is unexpected.
    tested_by: [oversized_body_is_unexpected]
  - rule: A Custom 404 on the model list is ok with listed false.
    tested_by: [custom_without_model_list_is_ok_unlisted]
  - rule: Each preset's answer reads into ids and windows as §3.1 states; OpenRouter keeps tool-capable models only.
    tested_by: [anthropic_list_reads_max_input_tokens, minimax_list_has_no_windows, deepseek_list_reads_context_window, moonshot_list_reads_context_length, openrouter_list_keeps_tool_models]
- agreements:
  - between: [test_saved_provider, harness opening]
    must: Read the provider key from scope provider/<slug>, name PROVIDER_API_KEY.
    tested_by: [saved_test_uses_the_stored_key]

**Done when:** the thirteen tests above pass, plus HTTP `provider_test_route_answers_outcome`, plus contracts, codemap and openapi.
Acceptance: Mohammed tests one real key per preset in the browser; any answer
that differs from §3.1 amends the spec and the parser (closes §3's OPEN).

## B-4 · Character colour on the agent

**Files**
- Create: `crates/shadows-core/migrations/0032_agent_color.sql`
  (`ALTER TABLE aylul_agent ADD COLUMN character_color TEXT NOT NULL DEFAULT 'purple';`)
- Modify: `crates/shadows-core/src/profiles/{model.rs,agents.rs,store.rs,contract.yaml}`,
  `crates/shadows-http/src/aylul_profiles_tests.rs`, `api/openapi.json`
- Test: `crates/shadows-core/tests/aylul_profiles/agents.rs`,
  `crates/shadows-core/tests/profile_migration.rs`

**Shapes:** `Agent.character_color: String` (required);
`AgentInput.character_color: Option<String>`. OpenAPI shows a string with a
pattern, not an enum.

**Contract lines**
- obligations:
  - rule: A colour id is 1..=32 of a-z, 0-9, -, starting with a letter; Core keeps no list of colours.
    tested_by: [color_id_shape_is_checked, unknown_but_well_formed_color_is_kept]
  - rule: Absent on create is purple; absent on update keeps the stored colour; replay compares the normalized value.
    tested_by: [color_defaults_to_purple, update_without_color_keeps_it]
  - rule: Existing agents migrate to purple.
    tested_by: [migration_colors_agents_purple]
- agreements:
  - between: [profiles::Agent.character_color, web and character renderer]
    must: Core stores any well-formed id; the clients own the colour list and draw purple for an id they do not know.
    holds: true
    tested_by: [unknown_but_well_formed_color_is_kept]

**Done when:** the five tests pass, plus contracts and openapi.

## B-5 · The person

**Files**
- Create: `crates/shadows-core/src/profiles/person.rs` (the person's record
  rules), `crates/shadows-core/migrations/0033_aylul_person.sql`,
  `crates/shadows-core/tests/aylul_profiles/person.rs`
- Modify: `crates/shadows-core/src/profiles/{mod.rs,model.rs,store.rs,contract.yaml}`,
  `crates/shadows-core/src/harness/{assistant.rs,contract.yaml}`,
  `crates/shadows-core/tests/aylul_profiles.rs`, `crates/shadows-core/tests/aylul_session/opening.rs`,
  `crates/shadows-core/src/lib.rs`,
  `crates/shadows-http/src/{aylul_profiles.rs,aylul_profiles_tests.rs,lib.rs}`,
  `api/openapi.json`, `docs/codebase/README.md`

**Table:** `aylul_person (id INTEGER PRIMARY KEY CHECK (id = 1), name TEXT,
aylul_calls_me TEXT, language TEXT, theme TEXT NOT NULL, updated_at TEXT NOT NULL)`.

**Shapes:** `Person { name: Option<String>, aylul_calls_me: Option<String>,
language: Option<String>, theme: Theme, updated_at: Option<String> }`;
`PersonInput` the same without `updated_at`; `Theme = dark | light | system`.

**Routes**
- `GET /api/me` → `200 Person` (defaults before the first write: all `null`,
  `theme: "system"`).
- `PUT /api/me`, body `{ command_id, name, aylul_calls_me, language, theme }` →
  `200 Person`; `400` bad value; `409` replay with other input.

**Opening (§5.2):** `aylul_setup` gets `append: Some(..)` with one line naming
the person when `aylul_calls_me` or `name` is set.

**Contract lines**
- functions: `person: Read the person's record or its defaults.`,
  `set_person: Validate and replace the record under one command.`
- obligations:
  - rule: Names are trimmed, at most 64 characters, without control characters or line breaks; empty is null.
    tested_by: [person_names_are_checked]
  - rule: Language is a short tag shape, not a list; theme is dark, light or system.
    tested_by: [language_tag_shape_is_checked, theme_values_are_checked]
  - rule: Replay requires the same normalized input.
    tested_by: [person_replay_with_other_input_conflicts]
- harness obligation:
  - rule: An AYLUL opening appends the name AYLUL calls the person, or nothing when none is set.
    tested_by: [opening_names_the_person, opening_without_person_appends_nothing]

**Done when:** the six tests pass, plus HTTP `me_round_trips` and contracts,
codemap, openapi.

## B-6 · Probe: what the adapter reports of usage

Not product code: a recorded run, then the spike is deleted (CLAUDE.md).
Enable `unstable_end_turn_token_usage` on `agent-client-protocol` in a probe
commit, run one AYLUL turn against each of two providers (Mohammed's keys, run
by him), log the raw `PromptResponse.usage` and `UsageUpdate.cost`, write
`docs/evidence/<date>-acp-usage-probe.md`, revert the probe. Its answer amends
spec §6 and writes B-7's contract.

## B-7 · Record and report usage

Blocked on B-6. Contract to be fixed by its answer. The intended shape, for the
frontend's planning only:
- `GET /api/aylul/usage?from=<rfc3339>&to=<rfc3339>` →
  `{ by_provider: [{ provider_slug, provider_name, label, requests, tokens_in,
  tokens_out, cost: number | null, currency: string | null }], by_model:
  [{ model, provider_slug, requests, tokens_in, tokens_out, cost, currency }] }`,
  each list ordered by tokens descending then slug or model.
- Rows are written in the turn's ending transaction (`turns`), read by
  `profiles::usage`; a deleted provider's rows keep their slug and name.

## B-8 · Probe: skills and plugins under an isolated home

Blocked on Mohammed's ruling on spec §7.1. A recorded run: one skill in
`<home>/skills/<name>/SKILL.md` and one local plugin enabled through
`settings.json`, opened through ACP with `settingSources: ["user"]`; does the
session list them? Evidence file, then the spike goes.

## B-9 · Plugins and skills library

Blocked on B-8. Intended routes, for planning only:
`GET /api/aylul/extensions` → `{ plugins: [Item], skills: [Item] }`;
`PUT /api/aylul/extensions/{kind}/{id}` `{ enabled, command_id }` → `Item`;
`POST /api/aylul/extensions` `{ kind, path, command_id }` → `201 Item`, where
`Item = { id, name, description, enabled }` and `kind = plugin | skill`.
