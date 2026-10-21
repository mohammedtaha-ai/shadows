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
on probes; the probes change nothing the frontend sees and may run before the
request is accepted. The account server and devices are out of scope (spec §8);
every task keeps spec §8.2's rules (nothing assumes the daemon is only local).

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
- No API key appears in a test, a fixture, a log line, an error text or a
  commit. Tests use the synthetic value `test-key-not-real`.
- Live keys exist only for OpenRouter and MiniMax, in the repository's `.env`.
  A live run reads `.env`'s variable names (`cut -d= -f1 .env`), never prints
  its values, and loads it into one command's environment only
  (`set -a; . ./.env; set +a; cargo test ...`). Anthropic, DeepSeek, Moonshot
  and the China presets get no live run until Mohammed has their keys.

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

**Mechanical follow-ups:** `crates/shadows-core/tests/aylul_profiles.rs`,
`crates/shadows-core/tests/aylul_session.rs`, `crates/shadows-http/src/aylul_env_tests.rs`,
`crates/shadows/tests/openapi/read_only.rs` adapt `ProviderInput` fixtures.

The legacy migration fixture in `crates/shadows-core/tests/aylul_profiles/threads.rs`
excludes every migration from 0029 onward when constructing its pre-profiles database.

`crates/shadows/tests/openapi.rs` adds the preset route to its route inventory.

**Shapes**
- `ProviderPreset { id: String, name: String, region: Option<PresetRegion>,
  base_url: String }`, `PresetRegion = international | china` (snake_case); the
  listing details of §3.1 stay private to `presets.rs`. Seven presets, §1.1.
- `Provider` gains `preset: Option<String>`, `label: Option<String>`.
- `ProviderInput`: `name: Option<String>`, `base_url: Option<String>`,
  `preset: Option<String>`, `label: Option<String>` (models unchanged in B-1).

**Migration 0030:** `ALTER TABLE aylul_provider ADD COLUMN preset TEXT;`
`ADD COLUMN label TEXT;` then `UPDATE ... SET preset = '<id>' WHERE base_url = '<url>'`
for each of the seven rows of §1.1.

**Route**
- `GET /api/aylul/provider-presets` → `200 [ProviderPreset]`, in §1.1's order
  (`[{ id, name, region, base_url }]`).
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
  - rule: A China preset carries its China host; the region is part of the preset, not a separate field on the provider.
    tested_by: [china_presets_use_china_hosts]
- tests: `presets_list_in_fixed_order`, `china_presets_use_china_hosts`, `preset_supplies_url_and_name`,
  `preset_with_other_url_is_refused`, `custom_requires_name_and_url`,
  `label_is_trimmed_and_empty_is_null`, `unknown_preset_is_refused`,
  `migration_marks_known_urls_with_their_preset` (in `profile_migration.rs`),
  HTTP: `provider_presets_route_lists_all` (in `aylul_profiles_tests.rs`).

**Done when:** the nine tests pass:
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

**Mechanical follow-ups:** `crates/shadows-core/src/profiles/mod.rs` re-exports the
model types; `crates/fake-acp/src/prompts.rs` reports only the two context variables
so opening acceptance can assert what reached the child. Existing `ProviderInput`
fixtures in `crates/shadows-core/tests/{aylul_profiles.rs,aylul_session.rs}`,
`crates/shadows-http/src/aylul_env_tests.rs`, and
`crates/shadows/tests/openapi/read_only.rs` carry model objects.

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
  `crates/shadows-core/src/lib.rs`, `crates/shadows-core/src/harness/{mod.rs,contract.yaml}`,
  `crates/shadows-http/src/{aylul_profiles.rs,aylul_profiles_tests.rs,lib.rs}`,
  `api/openapi.json`, `docs/codebase/README.md`

**Mechanical follow-ups:** `crates/shadows-core/src/profiles/providers.rs` extracts
its existing URL check for reuse by the probe; `crates/shadows-core/tests/aylul_profiles/threads.rs`
shares its existing core fixture with the saved-key test; `crates/shadows/tests/openapi.rs`
adds both routes to the inventory. `Cargo.lock` records the infrastructure dependency.
Mohammed's follow-up ruling: reserve provider slug `test` using the next free numeric suffix, remove mutation aliases, leave existing rows unchanged; add `crates/shadows-core/src/profiles/store.rs`, `crates/shadows-core/tests/{aylul_profiles/providers.rs,aylul_session.rs,aylul_session/opening.rs}`, `crates/shadows-http/src/aylul_env_tests.rs` and `crates/shadows/tests/openapi/read_only.rs` to the touch list for reservation tests and mechanical fixture updates.
`docs/backend/specs/2026-10-21-settings-dialog-backend-design.md` updates the
OPEN trigger after the authorized OpenRouter and MiniMax live checks pass.

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

**Methods:** `Profiles::test_provider(ProviderTest)` (the key comes in the request; nothing is
resolved). `Harness::test_saved_provider(slug, Option<String>)`: the stored-key path lives in
the harness, because `secrets.resolve_env` may be called only there
(`secrets/contract.yaml`, not_the_caller's). It resolves scope `provider/<slug>` the way
`harness/assistant.rs` does at opening, then calls the same probe function in
`profiles/probe.rs` (`pub(crate) async fn probe(base_url, api_key) -> ProviderTestOutcome`),
so both paths share one probe. Amended 2026-10-21 after Codex found the conflict.
Tests point a Custom `base_url` at a local `tokio` listener on `127.0.0.1` that
answers fixed bodies (the loopback http rule allows it); the preset URLs are
covered by `listing.rs` unit tests over fixture bodies written from §3.1.

**Contract lines**
- functions (profiles): `test_provider: Ask the provider for the key's models without storing the key.`
- functions (harness): `test_saved_provider: Same, with the stored key when none is given.`
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
  - between: [harness test_saved_provider, harness opening] (in `harness/contract.yaml`)
    must: Read the provider key from scope provider/<slug>, name PROVIDER_API_KEY, through secrets.resolve_env.
    tested_by: [saved_test_uses_the_stored_key]

**Done when:** the thirteen tests above pass, plus HTTP `provider_test_route_answers_outcome`, plus contracts, codemap and openapi.

**Acceptance (live):** an `#[ignore]` test `live_keys_list_models` in
`tests/aylul_profiles/probe.rs` reads the OpenRouter and MiniMax keys from the
environment (the `.env` names; the test skips a preset whose variable is
absent) and asserts `ok: true` with at least one model. It prints only the
preset id, `ok`, `reason` and the model count. Run:
`set -a; . ./.env; set +a; cargo test -p shadows-core --test aylul_profiles live_keys_list_models -- --ignored`.
Any answer that differs from §3.1 amends the spec and the parser. Anthropic,
DeepSeek, Moonshot and both China presets are accepted on fixtures only and
keep §3's OPEN until Mohammed has their keys.

## B-4 · Character colour on the agent

**Files**
- Create: `crates/shadows-core/migrations/0032_agent_color.sql`
  (`ALTER TABLE aylul_agent ADD COLUMN character_color TEXT NOT NULL DEFAULT 'purple';`)
- Modify: `crates/shadows-core/src/profiles/{model.rs,agents.rs,store.rs,contract.yaml}`,
  `crates/shadows-http/src/aylul_profiles_tests.rs`, `api/openapi.json`
- Test: `crates/shadows-core/tests/aylul_profiles/agents.rs`,
  `crates/shadows-core/tests/profile_migration.rs`

**Mechanical follow-ups:** `AgentInput` fixture constructors in
`crates/shadows-core/tests/{aylul_profiles.rs,aylul_session.rs}`,
`crates/shadows-http/src/aylul_env_tests.rs`, and
`crates/shadows/tests/openapi/read_only.rs` supply an absent color.
The color agreement anchors its unchanged rule to the real `Agent` and
`AgentInput` symbols: the contract checker accepts Core symbols, not field paths
or external client labels.

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

**Mechanical follow-up:** `crates/shadows/tests/openapi.rs` adds GET and PUT `/api/me`
to the explicit route inventory.

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
commit, run one AYLUL turn on OpenRouter and one on MiniMax with the keys from
`.env` (rules above), log the raw `PromptResponse.usage` and `UsageUpdate.cost`
(no key, no prompt text), write `docs/evidence/<date>-acp-usage-probe.md` naming
the probe commit, then revert the probe. No spending limit applies (Mohammed,
2026-10-21). Its answer amends
spec §6 and writes B-7's contract.

## B-7 · Record and report usage

**Status:** Released by the controller, 2026-10-21.
Needs B-6 done and the accepted request.
Evidence: [B-6](../../evidence/2026-10-21-acp-usage-probe.md); spec §6.

**Files / Touch (exact files)**
- Create: `crates/shadows-core/migrations/0034_aylul_usage.sql`,
  `crates/shadows-core/src/profiles/usage.rs` (period/report rules),
  `crates/shadows-core/src/profiles/usage_store.rs` (usage report SQL),
  `crates/shadows-core/src/turns/store/usage.rs` (turn usage SQL),
  `crates/shadows-core/tests/aylul_usage.rs` (end-to-end recording/reporting).
- Modify: `Cargo.toml` (enable only `unstable_end_turn_token_usage` in addition
  to current ACP features), `crates/shadows-agent/src/{acp.rs,events.rs}`,
  `crates/fake-acp/src/{main.rs,prompts.rs,session.rs}` (synthetic usage replies),
  `crates/shadows-core/src/profiles/{mod.rs,contract.yaml}`,
  `crates/shadows-core/src/turns/{contract.yaml,turn.rs}`,
  `crates/shadows-core/src/turns/store/{mod.rs,turn.rs,operation.rs}`,
  `crates/shadows-core/src/harness/{assistant.rs,live.rs,sessions.rs,model.rs,contract.yaml}`,
  `crates/shadows-core/src/turns/record.rs`,
  `crates/shadows-core/src/lib.rs`,
  `crates/shadows-core/tests/acp_connection.rs` (changed prompt result fixture),
  `crates/shadows-http/src/{aylul_profiles.rs,aylul_profiles_tests.rs,lib.rs}`,
  `crates/shadows/tests/openapi.rs`, `api/openapi.json`,
  `docs/codebase/README.md`, `docs/backend/stages.md`.
- Read before editing: profiles, turns and harness contracts; agent ACP prompt
  response and UsageUpdate dispatch; `turns/turn.rs`'s terminal ownership;
  `turns/store/{turn.rs,operation.rs}`'s start/end transactions. If a required
  mechanical caller is absent from this list, add its exact path in this
  section before editing, under the existing small-fix rule. No web/native files.

**Mechanical follow-ups:** `crates/shadows-core/src/testing/turn.rs` adds the
absent routing snapshot to NewTurn; `crates/shadows-core/src/harness/mod.rs`
re-exports the snapshot. `crates/shadows/tests/openapi/read_only.rs` supplies the
new GET's required period for its database-version assertion.
`crates/shadows-agent/src/{usage.rs,acp_tests.rs}` are new responsibility splits
from the growing ACP file; `crates/shadows-agent/src/lib.rs` registers usage.
These are B-7 Touch follow-ups of the changed field, route and file-size rule.
The timestamp index also covers julianday(recorded_at); the report narrows SQL
candidates and filters exact RFC3339 fractional boundaries in Rust.

**Interfaces**
- Keep ACP stop reasons intact. Return a `PromptResult { end: TurnEnd,
  usage: Option<TurnUsage> }` from `Connection::prompt` instead of discarding
  PromptResponse.usage. TurnUsage preserves input, output, cache-read,
  cache-write and total counts. Missing stays None; no context-counter fallback.
  Notifications remain dispatched before the prompt response is handed over.
- Carry the last optional cost with the existing usage notification and maintain
  a session-scoped baseline for the cumulative Claude SDK USD amount. A trusted
  per-turn cost is the nonnegative final-minus-prior amount, never the full
  session cumulative amount per turn; missing/reset/decreasing values stay None.
  Never apply a different session's baseline or a stale notification to a turn.
- `Profiles::usage(from, to) -> UsageReport`: validate RFC3339 endpoints,
  normalize to UTC and require from < to. Query a half-open interval [from,to).
  `GET /api/aylul/usage?from=<rfc3339>&to=<rfc3339>` (tag aylul) returns 200:
  `{ by_provider: [{ provider_slug, provider_name, label, requests, tokens_in,
  tokens_out, cost, currency }], by_model: [{ model, provider_slug, requests,
  tokens_in, tokens_out, cost, currency }] }`. Bad/missing periods are 400;
  storage failure is the existing 500 ErrorBody. No command and no write on GET.
- Use u64 counts after checked conversion from SQLite integers. tokens_in is
  inputTokens + cachedReadTokens + cachedWriteTokens; tokens_out is outputTokens.
  Never sum UsageUpdate.used (context) or add totalTokens a second time.
- Only actual terminal AYLUL invocations appear; requests counts those invocations,
  not provider HTTP calls or context updates. Missing usage remains nullable in
  storage; token totals sum reported components only (empty sum is zero).
  Historical pre-migration turns are not backfilled with invented measurements.
- Cost/currency are null for every preset except anthropic, including both
  MiniMax regions and OpenRouter. B-6 did not verify OpenRouter billing. For
  anthropic only, accept a finite nonnegative USD turn delta. A group's cost is
  null if any included row lacks trusted cost; never display a partial charge
  as a complete total or mix currencies.
- Both lists sort by tokens_in + tokens_out descending, then provider_slug;
  by_model adds model as the final tie-break. Group providers by slug and models
  by (provider_slug, model). Name/label come from the latest recorded snapshot
  in the interval, tie-broken by operation id; current provider state is not read.

**Tables / transaction boundaries**
- Migration 0034 creates `aylul_usage` with operation_id TEXT PRIMARY KEY,
  thread_id, agent_slug, provider_slug, provider_name, label, preset, model,
  nullable nonnegative INTEGER input_tokens/output_tokens/cached_read_tokens/
  cached_write_tokens/total_tokens, nullable cost REAL/currency TEXT,
  recorded_at TEXT. CHECK cost/currency are both null or both valid; index
  (recorded_at, provider_slug, model, operation_id). No cascading FK to the
  mutable provider, agent or thread: historical reports survive deletion.
- Add nullable AYLUL routing snapshot columns to agent_invocation (agent slug,
  provider slug/name/label/preset). Opening pins credential-free routing metadata
  beside OpenSession; record.rs passes that actual adapter snapshot into NewTurn,
  and start_turn copies it with the requested model in its invocation transaction.
  Do not rejoin mutable provider/agent configuration at turn end, or attribute a
  cached adapter to a provider selected only for the next opening. Mechanical
  NewTurn fixture additions must be enumerated in Touch before editing. No key
  or path is part of the snapshot; secrets resolution remains in the harness.
- The winning terminal transaction in operation.rs inserts usage with the
  operation status/event. Completed, explicitly cancelled and refused replies
  may carry measured usage; transport loss/forced termination/recovery must not
  invent tokens or cost. A missing-response row uses the frozen snapshot with
  unknown components. Preserve the existing terminal CAS/claim arbitration;
  insert nothing for a loser, replay, pending refusal or non-AYLUL turn.
- Put insert/read helpers in domain store modules, not db/mod.rs. Failed usage
  insert must roll back the status/event as well. Do not put a second write after
  mark_operation_completed/cancelled/failed. Runtime-recovered rows remain
  unmeasured; do not reinterpret Interrupted as a completed provider request.

**Contract lines**
- profiles functions: usage; usage_store.rs owns aggregation only. Harness pins
  the nonsecret adapter routing snapshot and turns stores it; do not reach a
  different service's store to reconstruct attribution at reporting time.
- turns shared_in_transaction: record_usage_in in its usage store; describe
  snapshot-at-start and atomic exactly-once terminal insertion with operation id.
- harness agreement: end-turn counts travel through the same connection used by
  turns; context counters remain independent. Update obligations with the named
  tests below, and map each new module's single responsibility in the code map.

**Failing tests first, with assertions**
1. `prompt_returns_raw_turn_usage`: fake ACP sends fixed raw counts and two
   context updates; assert PromptResult keeps each component once, and missing
   usage stays None. Run it red in acp_connection before changing the adapter.
2. `usage_counts_cache_once`: input=10, read=20, write=30, output=4, total=64;
   assert tokens_in=60, tokens_out=4, requests=1 regardless of context updates.
3. `usage_ends_atomically_once`: observe a completed AYLUL turn, assert one row;
   replay/competing terminal call adds none. Force the usage insert to fail with
   a test SQLite trigger; assert terminal status/event also roll back.
4. `usage_snapshot_survives_provider_changes`: rename/relabel or switch provider
   after start, then delete the old provider after the turn; assert its original
   attribution remains, and it is not joined onto today's provider record.
5. `usage_refusal_cancel_and_missing_are_honest`: synthetic refused/cancelled
   replies with measured usage count once; no-response cancellation has nullable
   counters, not fake zero measurements; non-AYLUL/pending refusals add no row.
6. `usage_costs_require_trusted_turn_delta`: successive cumulative costs 0.2 and
   0.5 in one Anthropic session yield 0.2 and 0.3, not 0.2 and 0.5; fresh sessions
   reset baseline. OpenRouter/MiniMax/China/Custom, NaN, negative, reset or missing
   costs yield null; a mixed known/unknown group also has null cost/currency.
7. `usage_periods_group_and_sort`: RFC3339 offset normalization, [from,to)
   endpoints, deterministic ties, two providers sharing one model, metadata
   snapshots and empty period; assert both empty lists with no records.
8. HTTP `usage_route_is_read_only`: 200 report, 400 malformed/missing/reversed
   period, unchanged database version; the GET inventory includes the route.

Run each failing focused test, implement its path, rerun it green. Then:
`cargo test -p shadows-core --test acp_connection`,
`cargo test -p shadows-core --test aylul_usage`,
`cargo test -p shadows-http aylul_profiles`, contracts, codemap,
`UPDATE_OPENAPI=1 cargo test -p shadows --test openapi`, fmt and clippy for
changed packages plus fake-acp with its test-support feature. Commands are
serial, only root target/, no npm/workspace tests or live credentials needed.

**Done when:** all eight named checks above and the existing turn/cancellation
checks they touch pass; contracts/codemap/OpenAPI match; counts and cost policy
match B-6; no secret or path enters usage rows/responses; one task commit after
the controller has released the task. Record verification in the task commit.

## B-8 · Probe: skills and plugins under an isolated home

Scope ruled (spec §7.1): one shared library, enabled per agent. A recorded run
with a MiniMax or OpenRouter key from `.env`: two agent homes; home A has one
skill in `<home>/skills/<name>/SKILL.md` and one local plugin enabled through
its `settings.json`, home B has neither. Open each through ACP with
`settingSources: ["user"]`: does A's session list both, and B's neither? Record
how the plugin had to be declared (local marketplace, `enabledPlugins`, a flag).
Evidence file naming the probe commit, then the spike goes.

## B-9 · Plugins and skills: shared library, enabled per agent

**Status:** steps written; blocked pending controller review. Needs B-8 done,
B-7's migration 0034 allocated, and the accepted request. Evidence:
[B-8](../../evidence/2026-10-21-agent-home-extensions-probe.md); spec §7.

**Files / Touch (exact files)**
- Create: `crates/shadows-core/migrations/0035_aylul_extensions.sql`,
  `crates/shadows-core/src/profiles/extensions.rs` (library service rules),
  `crates/shadows-core/src/profiles/extension_model.rs` (public item shapes),
  `crates/shadows-core/src/profiles/extension_store.rs` (library SQL),
  `crates/shadows-core/src/profiles/extension_files.rs` (owned library copies),
  `crates/shadows-core/src/harness/extension_home.rs` (per-agent projection),
  `crates/shadows-core/tests/aylul_extensions.rs`,
  `crates/shadows-http/src/aylul_extensions.rs`,
  `crates/shadows-http/src/aylul_extensions_tests.rs`.
- Modify: `crates/shadows-core/src/profiles/{mod.rs,store.rs,contract.yaml}`,
  `crates/shadows-core/src/harness/{mod.rs,assistant.rs,profile_home.rs,contract.yaml}`,
  `crates/shadows-core/src/lib.rs`,
  `crates/shadows-core/tests/aylul_session/{homes.rs,opening.rs}`,
  `crates/shadows-http/src/lib.rs`, `crates/shadows/tests/openapi.rs`,
  `api/openapi.json`, `docs/codebase/README.md`, `docs/backend/stages.md`.
- Read profiles/harness/secrets contracts; extension evidence; profile_home.rs's
  per-home serialization and atomic settings replacement; profiles/store.rs's
  agent tombstone transaction. No new dependencies, web or native edits.

**Interfaces**
- `ExtensionKind = plugin | skill`; `Item { id, name, description }` and
  `AgentItem = Item + { enabled }`; `ExtensionLibrary { plugins, skills }`,
  `AgentExtensions { plugins, skills }`. id is a generated UUID string, never
  a local path or machine-derived identifier. Library listing contains no
  enabled flag; agent listing contains the entire library with enabled bool.
- Profiles methods: extensions(), add_extension(command_id, kind, path),
  delete_extension(command_id, kind, id), agent_extensions(slug),
  set_agent_extension(command_id, slug, kind, id, enabled). Publish these names
  and obligations in profiles/contract.yaml; validation and command rules stay
  in Core. The path is a daemon-side folder input, never echoed in an error,
  command record payload, response or library metadata. No URL/Git fetching.
- Routes, all tag aylul:
  - GET /api/aylul/extensions -> 200 ExtensionLibrary.
  - POST /api/aylul/extensions { command_id, kind, path } -> 201 Item.
  - DELETE /api/aylul/extensions/{kind}/{id}?command_id= -> 204.
  - GET /api/aylul/agents/{slug}/extensions -> 200 AgentExtensions.
  - PUT /api/aylul/agents/{slug}/extensions/{kind}/{id}
    { command_id, enabled } -> 200 AgentItem.
  Bad bodies/kind/source manifests are 400, missing item or deleted/unknown
  agent 404, replay fingerprint mismatch or duplicate intrinsic kind/name 409;
  existing storage/unavailable conventions cover 500/503. Add typed
  ExtensionNotFound and ExtensionNameConflict variants/codes through CoreError,
  StorageError and failure mappings; use ProfileInvalid for invalid source
  content, with no filesystem location in the client-facing message. All
  errors retain ErrorBody. No error handler matches on free-form SQL text.
- List deterministically by name then id, independently in each kind. Metadata
  is imported once from SKILL.md frontmatter or .claude-plugin/plugin.json;
  it is read from the library on later use, not the person's source folder.
  Require a valid intrinsic Claude skill/plugin name and nonempty description;
  reject duplicate (kind, intrinsic name) to avoid two commands fighting for one
  name. A successful import creates one immutable item, disabled for every agent.
- Normalize inputs before fingerprinting. Import fingerprint is a SHA-256 of
  kind plus the normalized source locator; never store the plaintext path in the
  command payload. The immutable tree has a separate content digest. Classify
  command replay before reopening the source and return the originally recorded
  Item even after source/item deletion. Reusing that command for another source
  or kind conflicts; changing files at the same locator does not turn a replay
  into a new import. A new import command revalidates/copies today's contents.
  Toggle fingerprints include agent, kind, id and enabled; delete replay is 204.

**Tables / library copy / deletion**
- Migration 0035 creates aylul_extension(id TEXT PRIMARY KEY, kind TEXT,
  name TEXT, description TEXT, digest TEXT, created_at TEXT,
  UNIQUE(kind,name), CHECK kind IN ('plugin','skill')) and
  aylul_agent_extension(agent_slug TEXT, extension_id TEXT, enabled INTEGER,
  PRIMARY KEY(agent_slug,extension_id), CHECK enabled IN (0,1), foreign keys
  to agent/extension as appropriate). Missing enablement means false.
- extension_store.rs owns all SQL, including remove_agent_extensions_in in
  the existing delete_agent write (declare shared_in_transaction). Deleting an
  item removes every enablement row in its own command transaction. The agent
  tombstone remains; toggle validation rechecks live agent and item inside
  the same write after replay classification, so deletion races cannot enable
  a tombstone. No secrets resolution is added outside harness.
- Copy accepted source trees into daemon-owned home_root/library/<id>/ through
  a staging directory; use deterministic traversal and SHA-256 (already in
  Core). Reject symlinks/junctions/reparse points and escapes, nonregular files,
  invalid manifests and a source containing the destination. Bound imports to
  10000 files/64 MiB so a folder input cannot exhaust the daemon. Do not follow
  references to outside files or execute plugin scripts at import. For local
  plugin payloads retain supported components; copy, do not install from Git.
- Publish a complete immutable directory before its DB row becomes visible;
  commit row/command together. A failed DB write cleans only that unreferenced
  owned directory; an interrupted orphan is inert and may be cleaned on a later
  import, never exposed as an item. DB deletion is authoritative: stale files
  are never discoverable by new openings. Defer physical removal while an
  opening is reading a snapshot, or use copied snapshots so no active reader
  sees partial content. Never recursively delete a user source folder.

**Projection at the next opening**
- AYLUL opening reads the library/enablement snapshot through Profiles and
  serializes projection under the same per-agent home lock as write_models.
  A cached already-open adapter retains its starting configuration; switches
  take effect on its next actual open, not on every cached open_session call.
- extension_home.rs builds a complete daemon-owned snapshot inside the agent
  home: enabled skills at skills/<intrinsic-name>/SKILL.md, and a local directory
  marketplace containing only enabled plugin payloads with a valid owner/name
  manifest and relative plugin sources. Use one stable reserved marketplace
  name aylul-managed and enabledPlugins <plugin>@aylul-managed keys.
- Merge only the managed settings keys into settings.json. Retain unrelated
  env/permissions/model options and user's other marketplace/plugin entries.
  Record the previously managed names in a daemon-owned manifest; turn removed
  managed plugin entries explicitly false before removing their payloads.
  Reconcile their managed installed/cache entries as needed; a warm cache must
  not resurrect a disabled item. Do not wipe the whole plugins or skills tree.
  Refuse a pre-existing unowned skill-name or marketplace-name collision rather
  than overwrite user content. Stage/replace complete files atomically; failed
  projection refuses the opening and never spawns with mixed enablement.
- No --plugin-dir flag or global CLI changes: B-8 proved directory sources in
  extraKnownMarketplaces plus enabledPlugins under settingSources ["user"].
  Internal paths in settings are runtime configuration, never API metadata.
  Snapshot generation and enablement revision must be consistent across
  simultaneous openings/toggles/deletes. Do not update an active adapter's
  prompt/tools or leak another agent's items through shared plugin caches.

**Contract lines**
- profiles functions: the five methods above; obligations: machine-independent
  ids, no returned paths, copied immutable library, replay/conflict rules, whole
  library per-agent switches; shared_in_transaction remove_agent_extensions_in.
- harness obligation: only enabled managed items appear at actual opening;
  toggled-off/deleted items leave the next opening even with a warm cache;
  preserve unrelated home settings. Agreement with profiles deletion and
  snapshot read. Tests below own each rule. Add every new module's one-job row.

**Failing tests first, with assertions**
1. `extension_import_copies_and_redacts_source`: import one synthetic skill
   and plugin; assert UUID id/name/description only, copied bytes still present
   after deleting the source, all agents initially disabled, no absolute source
   in response/command record/error or durable event.
2. `extension_input_is_contained`: invalid frontmatter/manifest, duplicate
   intrinsic name, unsupported kind, link/junction escape and over-limit tree
   fail without a published DB row, command success or partially visible item.
3. `extension_commands_replay_and_conflict`: same import command returns the
   same id after source removal and item deletion; changed source locator/kind conflicts;
   toggle exact replay writes once, changed enabled conflicts; delete replay 204.
4. `extension_enablement_is_per_agent`: A enables skill+plugin, B enables neither;
   library lists both without enabled, both agent views list both with different
   bools. Unknown/deleted agents and mismatched kind/id refuse without writes.
5. `extension_deletions_remove_enablement_atomically`: delete item -> all agent
   enablement removed; delete agent -> its rows removed with tombstone. Inject
   a failing transaction and assert neither half commits; race toggle vs delete
   never leaves an enabled removed item or a live row under a tombstone.
6. `opening_projects_only_enabled_extensions`: two homes and copied library;
   inspect skill files, local marketplace sources and enabledPlugins; each home
   sees only its enabled payloads, user settings/unowned files preserved.
7. `opening_removes_disabled_cached_extensions`: seed prior managed plugin
   installed/cache metadata, toggle off/delete item, reopen; assert managed key
   false, stale managed commands/skills absent and unrelated cache untouched.
8. `extension_projection_is_atomic`: parallel opens while enablement changes
   leave one valid complete snapshot per opening; injected file failure spawns
   nothing and retains prior valid settings. User-name collisions refuse safely.
9. HTTP `extension_routes_round_trip`: 201 import, 200 library/agent/toggle,
   204 delete and replay; 400/404/409 shapes; no machine paths in all answers.
10. Ignored `live_agent_extension_isolation`: use only an authorized .env key
    injected into one command; harmless say-hi fixture; ACP commands contain
    skill/plugin in A and neither in B. Reopen A after disabling both, with its
    warm cache retained: neither marker may appear. Log only marker presence,
    versions and outcomes; no model text/keys. This extends the B-8 fresh-home
    evidence rather than claiming that B-8 tested warm-cache removal.

Run each test red, build its vertical path, run it green. Then
`cargo test -p shadows-core --test aylul_extensions`,
`cargo test -p shadows-core --test aylul_session`,
`cargo test -p shadows-http aylul_extensions`, contracts, codemap,
`UPDATE_OPENAPI=1 cargo test -p shadows --test openapi`, fmt and focused
clippy. Run the ignored live check separately with the command-only key policy.
All cargo commands serial; existing root target/ only; no npm/workspace gate.

**Done when:** the ten checks above pass, the accepted five routes are generated
in OpenAPI, atomic deletion agreements and code-map rows match implementation,
no native/web file changes, and the controller has released this task after
reviewing these steps. A plugin execution/security policy beyond normal local
Claude loading is not implied by discovery; no remote install is introduced.
