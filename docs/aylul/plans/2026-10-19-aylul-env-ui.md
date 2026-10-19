# AYLUL environment from the UI: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Mohammed adds, replaces and removes AYLUL's environment variables in
the web client's Settings page. Values are stored AES-256-GCM encrypted in
`shadows.db` and decrypted only when an AYLUL adapter spawns.

**Architecture:** A new `secrets` service in `shadows-core`, the eleventh
service, owns the tables, the encryption and the resolution. The binary keeps a
32-byte master key in the Windows Credential Manager and passes it to
`AppCore::start`. One `Arc<Secrets>` is shared by `AppCore` and `Sessions`.
The AYLUL opening (`harness/assistant.rs`) resolves the profile's pairs at each
spawn. The env file becomes a one-time import run by the binary after start.

**Tech Stack:** Rust (SQLx 0.9, `aes-gcm`, `hkdf`, `hmac`, `sha2`, `rand`,
`keyring` 3.6.3 `windows-native`), axum in `shadows-http`, React in `web/`.

**Spec:** [AYLUL brain §10](../specs/2026-10-19-aylul-claude-code-brain-design.md)
(read §5, §7 and §10; §10 was amended after a Codex review and is binding).

## Global Constraints

- A value never appears in an HTTP response, SSE or durable event,
  `command_record`, `tracing` line, error text, `Debug` output, test assertion
  message or evidence file. Errors name the variable only. Tests compare with
  `assert!(a == b)` and a message that has no value, never `assert_eq!` on a
  value.
- Only `crates/shadows` touches `keyring`. The core never calls the credential
  store.
- There is no plaintext fallback. Without a master key, routes and the AYLUL
  opening return `SECRETS_UNAVAILABLE`.
- Names: `^[A-Z_][A-Z0-9_]{0,127}$`. `CLAUDE_CONFIG_DIR`, `ANTHROPIC_API_KEY`
  and `ANTHROPIC_AUTH_TOKEN` are refused. `key_env` is checked by the same rule
  when the section loads. Values: 1..=32768 bytes, no NUL.
- Keys: the AEAD key is `HKDF-SHA256(master, info="shadows/secrets/aead/v1")`
  and the fingerprint key is `HKDF-SHA256(master, info="shadows/secrets/fingerprint/v1")`.
  The nonce is 12 bytes from `rand::rngs::OsRng` per write. AAD is
  `profile` + `\0` + `name`. Each row stores `format = 1`.
- Public service methods take `command_id: String` and return
  `Result<_, CoreError>`, as `Projects::create` does. Each method builds its
  `CommandContext` with `user_command`. Classify, mutate and record happen in
  one `write_txn`. The scope kind is `aylul_env`, the scope key is
  `profile/name`, and the kinds are `secret_env.set` and `secret_env.remove`.
  The outcome holds only `{name, updated_at}`. The `set` fingerprint params are
  `{profile, name, value_mac}`, where `value_mac` is the hex
  HMAC-SHA256(fingerprint key, value).
- Listing orders by `name` explicitly.
- No MCP tool.
- The project rules apply: the 300/500-line file rule (`sessions.rs` is 490
  and `web/src/api/client.ts` is 549, so neither may grow), contracts updated
  in the same commit, a code-map row per new module, and Rust lines under 100
  columns.
- Build into the project's `target/` only. Each commit runs only the tests it
  touches. The full gate runs once, at the end.

## Review Focus

1. A database opened with another master key: `list_env` still works, and a
   new AYLUL turn gets `SECRET_UNREADABLE` naming the variable, not a panic.
2. A replayed `set` with the same `command_id` and the same value replays. The
   same id with another value is `CommandConflict`.
3. The import never runs twice. After every variable is removed, a restart
   with `.env` still present does not re-import it, and the marker is checked
   before the file is opened.
4. With `key_env` removed and the daemon variable unset, a new AYLUL
   conversation gets HTTP `AYLUL_KEY_MISSING` naming `API_KEY`.
5. A malformed or wrongly typed JSON body (`{"value": 12345}`) gets a fixed
   error message that does not contain the input.

---

### Task 1: `secrets` service (tables, crypto, methods, errors)

**Files:**
- Create: `crates/shadows-core/migrations/0028_secret_env.sql`, with tables:
  - `secret_env(profile TEXT, name TEXT, format INTEGER NOT NULL, nonce BLOB, ciphertext BLOB, updated_at TEXT, PRIMARY KEY(profile, name))`
  - `secret_env_import(profile TEXT PRIMARY KEY, imported_at TEXT, stored INTEGER, skipped TEXT)`
- Create: `crates/shadows-core/src/secrets/{mod.rs,model.rs,store.rs,crypto.rs,contract.yaml}`
- Modify: `crates/shadows-core/src/app.rs`: `StartConfig.secret_key: Option<SecretKey>`, an `Arc<Secrets>` in `CoreParts` and `AppCore`, the accessor `secrets()`, and the same Arc passed into `Sessions` via `SessionsConfig`. Do not add lines to `sessions.rs` beyond the field.
- Modify: `crates/shadows-core/src/error.rs`: `CoreError` variants and `ErrorCode`s `SECRET_NAME_INVALID`, `SECRET_VALUE_INVALID`, `SECRET_UNREADABLE`, `SECRETS_UNAVAILABLE`, `AYLUL_KEY_MISSING`, `AYLUL_NOT_CONFIGURED`. Each carries a name at most.
- Modify: `crates/shadows-core/src/lib.rs`, `crates/shadows-core/Cargo.toml`
- Modify: `docs/codebase/README.md` (owner rows), `docs/superpowers/specs/2026-09-26-application-core-design.md` §14.4 (the service list goes to eleven), and the CLAUDE.md architecture line ("holds ten services" becomes eleven, with `secrets` named)
- Test: unit tests inside `secrets/` for crypto and `resolve_env`, plus `crates/shadows-core/tests/secret_env.rs` for the public methods

**Interfaces (produced):**
- `pub struct SecretKey` (32 bytes). Its constructor is `from_bytes([u8; 32])`. It derives `Clone`. `Debug` prints `SecretKey(..)`. No `Display`.
- `pub struct EnvEntry { pub name: String, pub updated_at: String }`
- `pub struct ImportReport { pub stored: usize, pub skipped: Vec<String> }` (names only)
- `impl Secrets`:
  - `pub async fn list_env(&self, profile: &str) -> Result<Vec<EnvEntry>, CoreError>`
  - `pub async fn set_env(&self, command_id: String, profile: &str, name: &str, value: &str) -> Result<EnvEntry, CoreError>`
  - `pub async fn remove_env(&self, command_id: String, profile: &str, name: &str) -> Result<(), CoreError>`
  - `pub async fn import_needed(&self, profile: &str) -> Result<bool, CoreError>`
  - `pub async fn import_env_once(&self, profile: &str, pairs: Vec<(String, String)>) -> Result<Option<ImportReport>, CoreError>`. It returns `None` when the marker already exists. One `write_txn` re-checks the marker, encrypts and stores every valid pair (the last duplicate wins), and writes the marker.
  - `pub(crate) async fn resolve_env(&self, profile: &str) -> Result<Vec<(String, String)>, CoreError>`

- [ ] **Step 1: Write the failing tests.**
  - Unit tests: `seal_open_round_trips`, `open_with_other_aad_fails`, `open_with_other_key_fails`, `short_nonce_is_an_error_not_a_panic`, `unknown_format_is_an_error`, `hkdf_keys_differ`.
  - In `tests/secret_env.rs`:
    - `stored_ciphertext_is_not_the_value` (read the row with SQL after `set_env`)
    - `list_names_in_order_without_values`
    - `replace_keeps_one_row`
    - `remove_drops_the_row`
    - `replay_same_value_replays`
    - `replay_other_value_conflicts`
    - `moved_row_is_unreadable`
    - `other_master_key_lists_but_cannot_resolve`
    - `lower_case_bad_and_reserved_names_are_refused` (`api_key`, `1A`, `A-B`, 129 chars, `ANTHROPIC_AUTH_TOKEN`)
    - `empty_nul_and_oversized_values_are_refused`
    - `no_key_refuses_every_method`
    - `import_runs_once_even_after_all_removed`
    - `import_skips_bad_names_and_keeps_last_duplicate`
    - `failed_import_leaves_no_rows_and_no_marker`
    - `value_absent_from_db_wal_shm_and_command_record`
- [ ] **Step 2:** Run `cargo test -p shadows-core secrets` and `cargo test -p shadows-core --test secret_env`. Expected: FAIL.
- [ ] **Step 3: Implement.**
  - `crypto.rs`: `seal(&AeadKey, aad, plain) -> Result<Sealed, CryptoError>` and `open(&AeadKey, aad, &Sealed) -> Result<String, CryptoError>`. The nonce length is checked before conversion.
  - `store.rs`: all SQL (upsert on the primary key, ordered select, marker).
  - `mod.rs`: validation, the command flow and the methods.
  - The contract lists the methods, the no-value-out obligation and the tests.
- [ ] **Step 4:** Run the same tests, plus `cargo test -p shadows --test codemap` and `cargo test -p shadows-core --test contracts`. Expected: PASS.
- [ ] **Step 5:** Commit `feat(core): secrets service stores AYLUL variables encrypted`.

### Task 2: AYLUL spawn, typed errors, master key and import

**Files:**
- Modify: `crates/shadows-agent/src/profile.rs`:
  - remove `ClaudeProfile.env_file`, the file read and `ProfileError::EnvFile`
  - add `spawn_env_with(&self, stored: &[(String, String)]) -> Result<Vec<(String, String)>, ProfileError>`, keeping the same order and override rules
  - move `parse_dotenv` and its tests to `crates/shadows/src/aylul.rs`
- Modify: `crates/shadows-agent/src/claude.rs`: `process_spec_with` takes the stored pairs with the profile.
- Modify: `crates/shadows-core/src/harness/assistant.rs`: `opening` calls `self.secrets.resolve_env(&config.profile.name)` and passes the pairs.
- Modify: `crates/shadows-core/src/harness/sessions.rs`: add `OpenError::Env(CoreError)`, nothing else.
- Modify: the turn-start path that maps `OpenError` to `CoreError`, so `Env` keeps its own code instead of `HarnessStartFailed`.
- Modify: `crates/shadows-http/src/failure/mod.rs`: map the six new codes to statuses. These are client errors with a fixed, name-only message:
  - `AYLUL_KEY_MISSING` and `SECRET_UNREADABLE` → 409
  - `SECRETS_UNAVAILABLE` → 503
  - `AYLUL_NOT_CONFIGURED` → 404
  - name and value errors → 400
- Modify: `crates/shadows/src/aylul.rs`:
  - `AylulSection` keeps `env_file` as the import source
  - `prepare` validates `key_env` by the name rule and returns `(AylulConfig, Option<PathBuf>)`
  - new `pub async fn import_env(core, config, path)`. It calls `import_needed` before opening the file, then `parse_dotenv`, then `import_env_once`. It logs the stored count and the skipped names, and returns `()`. Errors are logged without values and never propagated.
- Create: `crates/shadows/src/master_key.rs` with `pub fn load_or_create() -> anyhow::Result<SecretKey>`:
  - use `keyring::Entry::new("shadows", "secrets-master-key")` and `get_secret`
  - on `NoEntry`, create the key with `OsRng`, `set_secret` it, then `get_secret` again and use what is stored
  - any other error, or a stored length that is not 32, is an error, and the entry is never overwritten
- Modify: `crates/shadows/src/cli/mod.rs` (serve) and `crates/shadows/src/daemon/mod.rs:74` (daemon):
  - when AYLUL is configured, call `load_or_create`
  - on error, log it and continue with `secret_key: None`
  - after `ensure_project`, await `import_env` inline; it cannot fail the start
- Modify: `crates/shadows/Cargo.toml` (`keyring = { version = "3.6.3", features = ["windows-native"] }`, `rand`), `crates/shadows/src/lib.rs` (`mod master_key`), and the contracts and code-map rows for `harness`, `daemon` and `master_key`
- Test:
  - `profile.rs`: the stored-pair versions of the old env-file tests, with the same order and override assertions
  - `crates/shadows-core/tests/aylul_session.rs`:
    - `stored_key_reaches_the_adapter`: fake-acp checks the env and returns only a boolean
    - `replaced_value_reaches_a_new_adapter`
    - `removed_key_is_aylul_key_missing`
    - `unreadable_row_is_secret_unreadable`
  - `crates/shadows/src/aylul.rs`: `parse_dotenv` cases, `lower_case_key_env_is_refused`, and `import_reads_no_file_when_marker_exists` (point at a missing path after the marker; no error)

- [ ] **Step 1:** Write the failing tests listed above.
- [ ] **Step 2:** Run `cargo test -p shadows-agent profile`, `cargo test -p shadows-core -p fake-acp --features fake-acp/test-support --test aylul_session` and `cargo test -p shadows aylul`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run the same tests. Expected: PASS. No test touches the real Credential Manager; the browser journey checks `load_or_create`.
- [ ] **Step 5:** Commit `feat(aylul): sessions read encrypted stored variables; env file imports once`.

### Task 3: HTTP routes and the Settings section

**Files:**
- Create: `crates/shadows-http/src/aylul_env.rs`, with:
  - `GET /api/aylul/env` returning `[{name, updated_at}]`, or `AYLUL_NOT_CONFIGURED`
  - `PUT /api/aylul/env/{name}` with body `{command_id, value}`, returning `{name, updated_at}`
  - `DELETE /api/aylul/env/{name}?command_id=...`, returning 204
  - The body is read with a local JSON extractor whose rejection is the fixed text "the request body is not valid JSON for this route", so the existing `rejections_as_error_bodies` never sees serde's text.
  - Each handler calls one `Secrets` method; the profile is the AYLUL profile name from the core's config.
- Modify: `crates/shadows-http/src/lib.rs` (mount) and `openapi.rs`. Regenerate `api/openapi.json` with `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi`.
- Create: `web/src/api/aylul-env.ts`, holding `listAylulEnv`, `setAylulEnv` and `removeAylulEnv`. They use `unwrap`/`unwrapEmpty` from `./error`, and the query key is `['aylul-env']`. It is a new file because `client.ts` is past 500 lines; its one job is AYLUL environment calls. Add its row to `docs/frontend/codebase/README.md`.
- Create: `web/src/app/settings/aylul-env.tsx` and `aylul-env.test.tsx`
- Modify: `web/src/app/settings/settings-page.tsx` (render after `ActiveProjects`), `web/src/app/settings/contract.yaml`, and `web/src/api/schema.d.ts` (`npm --prefix web run gen:api`)
- Test: route tests next to the existing `shadows-http` route tests, plus the web tests.

The web section is titled "AYLUL environment". The copy reads: "Values are
encrypted and never shown again. Changes apply to new AYLUL conversations."

- It is hidden only on `AYLUL_NOT_CONFIGURED`. Any other error is shown with the existing error line.
- Each row shows a name and `updated_at`, with "Replace" (a password input) and "Remove".
- The add form has a name input (upper-cased as typed) and a password input.
- After a save, the input clears and the value is never rendered.

- [ ] **Step 1: Write the failing tests.**
  - Rust:
    - `put_then_list_shows_name_only`
    - `put_response_has_no_value`
    - `bad_name_is_400`
    - `numeric_value_body_error_does_not_echo` (sends `{"command_id":"c","value":918273645}`; the body does not contain `918273645`)
    - `without_aylul_list_is_404_aylul_not_configured`
    - `turn_without_key_is_aylul_key_missing`
  - Web: `lists_names_without_values`, `adding_clears_the_input`, `hidden_only_when_not_configured`, `shows_unavailable_error`.
- [ ] **Step 2:** Run `cargo test -p shadows-http aylul_env` and `npm --prefix web test -- aylul-env`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run the same tests, plus `cargo test -p shadows --test openapi` and `npm --prefix web run typecheck`. Expected: PASS.
- [ ] **Step 5:** Commit `feat(web): add, replace and remove AYLUL variables from Settings`.

### Task 4: Gate (once)

- [ ] Run CI steps 1–6 from `CLAUDE.md`, the 100-column check and `web` gen:api/typecheck/lint/test/build. All must be green.
- [ ] Report the test counts and each file that passed 300 lines, with its one job. Do not push.

The controller runs the browser journey (spec §10 acceptance 1–4) on a copy of
the database, then writes `docs/evidence/aylul/2026-10-19-aylul-env-ui.md`.
