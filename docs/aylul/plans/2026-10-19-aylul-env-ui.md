# AYLUL environment from the UI: implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Mohammed adds, replaces and removes AYLUL's environment variables in
the web client's Settings page. Values are stored AES-256-GCM encrypted in
`shadows.db` and decrypted only when an AYLUL session spawns.

**Architecture:** A new `secrets` service in `shadows-core`, the eleventh
service, owns the table, the encryption and the resolution. The binary keeps a
32-byte master key in the Windows Credential Manager and passes it to
`AppCore::start`. The AYLUL opening (`harness/assistant.rs`) asks `secrets`
for the profile's pairs at each spawn. The env file becomes a one-time import.

**Tech Stack:** Rust (SQLx 0.9, `aes-gcm`, `keyring`, `rand`), axum routes in
`shadows-http`, React in `web/`.

**Spec:** [AYLUL brain §10](../specs/2026-10-19-aylul-claude-code-brain-design.md#10-environment-variables-from-the-ui-slice-2-decided-2026-10-19)
(read §5, §7 and §10).

## Global Constraints

- A value never appears in an HTTP response, SSE or durable event, `tracing`
  line, error text, `Debug` output, test assertion message or evidence file.
  Errors name the variable only.
- The core never calls the credential store. Only `crates/shadows` touches
  `keyring`.
- There is no plaintext fallback: without a master key, the routes return a
  named error and AYLUL's opening fails with it.
- Names match `^[A-Za-z_][A-Za-z0-9_]{0,127}$`. `CLAUDE_CONFIG_DIR`,
  `ANTHROPIC_API_KEY` and `ANTHROPIC_AUTH_TOKEN` are refused. Values are
  1..=32768 bytes.
- The associated data is `profile` + `\0` + `name`. The nonce is 12 random
  bytes per write.
- Listing orders by `name` explicitly (no implicit `SELECT` order, no `rowid`).
- No MCP tool for any of this.
- Every mutating route carries `command_id` and builds its context with
  `user_command` (`crates/shadows-core/src/app.rs`). The fingerprint covers
  `profile` and `name` only, never the value, so a replay with another value
  is a `CommandConflict`.
- The project rules apply: files under 300/500 lines, the contract is updated
  in the same commit, the code map gets a row for every new module, and lines
  stay under 100 columns.
- Build into the project's `target/` only. Each commit runs only the tests it
  touches. The full gate runs once, at the end.

## Review Focus

1. A database copied to another machine (another master key): listing still
   works, and the spawn fails with "aylul env cannot be decrypted", not a panic.
2. Two quick replaces of the same name: the second wins, and there is one row.
3. The `.env` import runs only when the profile has no rows. A restart after
   Mohammed removes every variable does not re-import a file that is still
   there. A marker row records that the import happened.
4. `key_env` removed from the UI while the daemon variable is also unset:
   the next AYLUL turn fails with `aylul key env API_KEY is not set`.
5. A value that contains `=`, quotes, spaces or newlines reaches the child
   byte for byte.

---

### Task 1: `secrets` service (store, crypto, methods)

**Files:**
- Create: `crates/shadows-core/migrations/0028_secret_env.sql`
- Create: `crates/shadows-core/src/secrets/{mod.rs,model.rs,store.rs,crypto.rs,contract.yaml}`
- Modify: `crates/shadows-core/src/app.rs` (hold `Secrets`, accessor `secrets()`, `StartConfig.secret_key`)
- Modify: `crates/shadows-core/src/lib.rs`, `crates/shadows-core/Cargo.toml` (`aes-gcm`, `rand`)
- Modify: `docs/codebase/README.md` (one owner row per new module)
- Test: `crates/shadows-core/tests/secret_env.rs`

**Interfaces (produced):**
- `pub struct SecretKey([u8; 32])` with `SecretKey::from_bytes([u8;32])`.
  `Debug` prints `SecretKey(..)`. No `Clone` into logs, and no `Display`.
- `StartConfig.secret_key: Option<SecretKey>`
- `pub struct EnvEntry { pub name: String, pub updated_at: String }`
- `pub enum SecretError { NoKey, BadName(String), Reserved(String), BadValue(String), Decrypt(String), Storage(..) }`.
  Every variant carries a name, never a value.
- `impl Secrets`:
  - `pub async fn list_env(&self, profile: &str) -> Result<Vec<EnvEntry>, SecretError>`
  - `pub async fn set_env(&self, cmd: CommandContext, profile: &str, name: &str, value: &str) -> Result<(), SecretError>`
  - `pub async fn remove_env(&self, cmd: CommandContext, profile: &str, name: &str) -> Result<(), SecretError>`
  - `pub(crate) async fn resolve_env(&self, profile: &str) -> Result<Vec<(String, String)>, SecretError>`
  - `pub async fn import_env_once(&self, profile: &str, pairs: Vec<(String, String)>) -> Result<usize, SecretError>`
- Table `secret_env(profile TEXT, name TEXT, nonce BLOB, ciphertext BLOB, updated_at TEXT, PRIMARY KEY(profile, name))`
  and `secret_env_import(profile TEXT PRIMARY KEY, imported_at TEXT)`.

- [ ] **Step 1: Write the failing tests** in `tests/secret_env.rs`:
  - `set_then_resolve_round_trips`: set `API_KEY` to `a=b "c" d\ne`, then `resolve_env` returns exactly that.
  - `list_returns_names_in_order_without_values`: set `B` and `A`; the list is `["A","B"]`, and a serialized list does not contain either value.
  - `replace_keeps_one_row`, `remove_drops_the_row`.
  - `a_row_moved_to_another_name_fails_to_decrypt`: copy the nonce and ciphertext from `X` to `Y` in SQL; `resolve_env` gives `Decrypt("Y")`.
  - `another_key_cannot_decrypt`: reopen the same DB with a different `SecretKey`; you get `Decrypt`, and no panic.
  - `bad_and_reserved_names_are_refused`: `1A`, `A-B`, a 129-char name, and `ANTHROPIC_AUTH_TOKEN`.
  - `empty_and_oversized_values_are_refused`.
  - `no_key_refuses_set_and_resolve`.
  - `import_runs_once`: import 2 pairs gives 2; remove both, import again gives 0.
  - `the_db_file_never_holds_the_value`: set a distinctive value, close, then read the db bytes; the value is absent.
- [ ] **Step 2:** Run `cargo test -p shadows-core --test secret_env`. Expected: FAIL (no module).
- [ ] **Step 3: Implement.**
  - `crypto.rs` holds `seal(key, aad, plain) -> (nonce, ct)` and `open(...)` on `aes_gcm::Aes256Gcm`.
  - `store.rs` holds the SQL (upsert on the primary key, ordered select).
  - `mod.rs` holds validation and the methods.
  - The contract lists the methods, their obligations (the no-value-out rule) and their tests.
- [ ] **Step 4:** Run the same test, plus `cargo test -p shadows --test codemap` and `cargo test -p shadows-core --test contracts`. Expected: PASS.
- [ ] **Step 5:** Commit `feat(core): secrets service stores AYLUL variables encrypted`.

### Task 2: AYLUL spawn reads stored variables; master key and import at start

**Files:**
- Modify: `crates/shadows-agent/src/profile.rs`: `ClaudeProfile.env_file` is replaced by spawn-time pairs. Add `spawn_env_with(&self, stored: Vec<(String,String)>)`. Remove the file read and `ProfileError::EnvFile`. `parse_dotenv` moves to the binary.
- Modify: `crates/shadows-agent/src/claude.rs`: `process_spec_with(cwd, Option<(&ClaudeProfile, Vec<(String,String)>)>)`, or an equivalent that passes the pairs in.
- Modify: `crates/shadows-core/src/harness/assistant.rs`: `opening` calls `resolve_env(&config.profile.name)` before `process_spec_with`. A `SecretError` becomes `OpenError::Start(error.to_string())`.
- Modify: `crates/shadows/src/aylul.rs`: keep `env_file` in `AylulSection` as the import source. `ensure_project` reads it with `parse_dotenv` and calls `import_env_once`.
- Create: `crates/shadows/src/master_key.rs` with `pub fn load_or_create() -> anyhow::Result<SecretKey>` on `keyring::Entry::new("shadows", "secrets-master-key")`. The key is stored hex-encoded and created with `rand` when absent.
- Modify: `crates/shadows/src/cli/*` (serve and daemon). When an AYLUL section is present, call `load_or_create` and pass `secret_key` into `StartConfig`. On failure, log the error with no key bytes, and the daemon still starts with `secret_key: None`.
- Modify: the `harness`, `daemon` and `projects` contracts where their lines change. Add a code-map row for `master_key`.
- Test: the `profile.rs` unit tests (rewrite the env-file tests as stored-pair tests, keeping the same ordering and override assertions), `crates/shadows-core/tests/aylul_session.rs` (a stored `API_KEY` reaches the fake adapter's env, a removed key gives the named error), and `crates/shadows/src/aylul.rs` tests (`parse_dotenv` cases move here, import-once).

- [ ] **Step 1:** Write the failing tests listed above. Add `a_replaced_value_reaches_the_next_session_without_restart` in `aylul_session.rs`.
- [ ] **Step 2:** Run `cargo test -p shadows-agent profile` and `cargo test -p shadows-core -p fake-acp --features fake-acp/test-support --test aylul_session`. Expected: FAIL.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** Run the same tests, plus `cargo test -p shadows aylul`. Expected: PASS. Do not run a test that writes to the real Credential Manager. `load_or_create` is checked in the browser journey.
- [ ] **Step 5:** Commit `feat(aylul): sessions read encrypted stored variables; env file imports once`.

### Task 3: HTTP routes and the Settings section

**Files:**
- Create: `crates/shadows-http/src/aylul_env.rs`, with:
  - `GET /api/aylul/env` returning `[{name, updated_at}]`, or 404 `AYLUL_NOT_CONFIGURED` when there is no AYLUL section
  - `PUT /api/aylul/env/{name}` with body `{command_id, value}`, returning 204
  - `DELETE /api/aylul/env/{name}` with body `{command_id}`, returning 204
  - `SecretError` maps to 400 for name/value errors and 503 `SECRETS_UNAVAILABLE` for `NoKey`. No error body carries a value.
- Modify: `crates/shadows-http/src/lib.rs` (mount), `openapi.rs`, `api/openapi.json`
- Create: `web/src/app/settings/aylul-env.tsx` and `aylul-env.test.tsx`
- Modify: `web/src/app/settings/settings-page.tsx` (render after `ActiveProjects`), `web/src/app/settings/contract.yaml`, `web/src/api/schema.d.ts` (regenerated with `npm run gen:api`), `docs/frontend/codebase/README.md`
- Test: `crates/shadows-http` route test (PUT then GET shows the name and not the value; a bad name gives 400), plus the web tests.

The web section is titled "AYLUL environment" and shown only when `GET /api/aylul/env` succeeds. It has:
- one row per name with `updated_at`, "Replace" (opens a password input) and "Remove"
- an add form with a name input and a password input
- after save, the input clears and the value is never rendered
- copy: "Values are encrypted and never shown again."

- [ ] **Step 1:** Write the failing tests:
  - Rust: `put_then_list_shows_name_only`, `bad_name_is_400`, `without_aylul_list_is_404`.
  - Web: `lists_names_without_values`, `adding_clears_the_input`, `hidden_when_aylul_absent`.
- [ ] **Step 2:** Run `cargo test -p shadows-http aylul_env` and `npm --prefix web test -- aylul-env`. Expected: FAIL.
- [ ] **Step 3:** Implement. The routes call one `Secrets` method each and hold no rule.
- [ ] **Step 4:** Run the same tests, plus `npm --prefix web run typecheck` and `git diff --stat api/`. Expected: PASS, and `openapi.json` changes only by the three routes.
- [ ] **Step 5:** Commit `feat(web): add, replace and remove AYLUL variables from Settings`.

### Task 4: Gate (once)

- [ ] Run CI steps 1–6 from `CLAUDE.md`, the 100-column check and `web` gen:api/typecheck/lint/test/build. All must be green.
- [ ] Report the test counts. Do not push.

The controller runs the browser journey (spec §10 acceptance 1–4) on a copy of
the database, then writes `docs/evidence/aylul/2026-10-19-aylul-env-ui.md`.
