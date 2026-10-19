# AYLUL environment UI - automated gate, 2026-10-19

This is an automated-gate record for `feat/aylul-env-ui`, starting at
`2bd7311a0e646bbd55740679b75692945d3900e7`. It does not establish live
Windows Credential Manager, browser, native application or production acceptance.
The controller's spec section 10 acceptance journey remains pending.

## Changes reviewed

| Task | Commit | Result |
|---|---|---|
| 1 | `1997614` | Secrets service encrypts profile variables; metadata-only reads and keyed fingerprints |
| 2 | `f37975e` | AYLUL resolves stored pairs at spawn; startup imports the old source once |
| 3 | `b960a60` | Settings adds, replaces and removes variables through generated HTTP types |
| 4 | This record's commit | Fresh whole-branch review, master-key race fix and automated gate |

The review covered canonical route declarations rather than independently treating
generated OpenAPI/TypeScript fields as authority. It checked crypto derivation,
row authentication, malformed envelopes, replay/conflict behavior, import permanence,
typed opening failures, process overrides and the Settings editor.

## Review fix

**Important: concurrent first starts could retain different master keys.**
Read/create/readback alone permits daemon A to return key A before daemon B
overwrites the credential with key B. A's subsequent ciphertext would then be
unreadable after restart.

`master_key.rs` now holds an OS file lock over the entire read/create/readback.
Every database/profile for the user shares
`%LOCALAPPDATA%/shadows/secrets-master-key.lock`. The file stores no key bytes
and must stay in place; close or process exit releases the OS lock. An unusable
lock path produces the existing unavailable-key behavior. This uses the standard
library's [File::lock](https://doc.rust-lang.org/std/fs/struct.File.html#method.lock),
available since Rust 1.89, without adding a dependency.

An unchanged-algorithm extraction allowed tests to use an explicit
`keyring::mock::MockCredential`. With another starter holding the temporary lock,
the regression test initially failed because credential access completed early:
**3 passed, 1 failed**. After locking, all **4 passed**. The other tests cover
creation/readback, wrong-length preservation and unreadable-entry preservation.
No test calls the production loader or the real credential backend.

No remaining Critical/Important issue was found. There are no deferred minors.
The review declined to certify real credential persistence/concurrency and the
browser journey because those are controller-owned, explicitly unexecuted checks.
No other reviewed input class was set aside.

## Automated verification

Windows checkout, Rust/Cargo 1.98.1, Node 24.15.0, npm 11.12.1. Task 4 Cargo builds
used the repository's `target/`, `CARGO_INCREMENTAL=0` and
`CARGO_PROFILE_DEV_DEBUG=0`. These local results are not remote CI results.

| Check | Result |
|---|---|
| `cargo fmt --all --check` | PASS |
| Exact CI find/awk Rust 100-character check | PASS after wrapping two overlong lines |
| `cargo clippy --workspace --all-targets --features fake-acp/test-support -- -D warnings` | PASS |
| `cargo test --workspace`, then failed/unreached workspace artifacts only | Initial FAIL; completed gate: 601 passed, 0 failed, 1 ignored |
| `cargo clippy --workspace -- -D warnings` | PASS |
| `cargo build` | PASS |
| `cargo tree -e features,no-dev --workspace` has no `test-support` | PASS: zero matches |
| `git diff --exit-code api/` | PASS |
| `npm --prefix web run gen:api` plus no generated diff | PASS |
| `npm --prefix web run typecheck` | PASS |
| `npm --prefix web run lint` | PASS |
| `npm --prefix web test` | PASS: 57 files, 285 tests |
| `npm --prefix web run build` | PASS; non-fatal large-chunk advisory |

Final Rust totals cover **91 executable targets: 601 passed, 0 failed,
1 ignored, 0 measured, 0 filtered**. Seven doctest targets ran **0 tests**
(0 passed/failed/ignored). The existing ignored test is
`migrate_prepared_dev_copy_only`, requiring a prepared SQLite backup through
`SHADOWS_DESIGN_MIGRATION_COPY`; its ignore was retained.

The initial workspace run completed 81 successful targets before its failed
storage target. After correction, the final counted continuation was:

| Workspace test artifact | Passed | Failed | Ignored |
|---|---:|---:|---:|
| `shadows-core/thread_session` | 2 | 0 | 0 |
| `shadows-core/workflow_rules` | 18 | 0 | 0 |
| `shadows-core/storage_contract` | 7 | 0 | 0 |
| `shadows-http` library | 7 | 0 | 0 |
| `shadows-index` library | 0 | 0 | 0 |
| `shadows-index/extract` | 3 | 0 | 0 |
| `shadows-mcp` library | 0 | 0 | 0 |
| `shadows-process` library | 0 | 0 | 0 |
| `shadows-process/tree_probe` | 0 | 0 | 0 |
| `shadows-process/containment` | 8 | 0 | 0 |

The single full gate began after the review fix. Its initial line-length check
found `secrets/store.rs` at 101 characters and `aylul_env.rs` at 103. Wrapping
the SQL/annotation retained identical route descriptions and generated artifacts.
Removing the overflow also let rustfmt format the previously skipped import block.

The first all-target Clippy attempt rejected `err().expect()` in a new profile
test. Explicit matching preserves the rule that a failed assertion cannot print
the successful environment vector. New turn assertions with non-secret operation
IDs use `expect_err`. An intermediate attempted `expect_err` on `OpenSession`
failed to compile because that type has no `Debug`; explicit matching fixed it.
The failed step was rerun without suppressing lints.

The unfiltered workspace test command stopped in `storage_contract`: **6 passed,
1 failed**. Its explicit schema inventory was missing only migration 0028's
`secret_env` and `secret_env_import`. The inventory/message were updated without
weakening the equality check; the failed binary then passed **7/7**.
Cargo's compiled test-artifact inventory identified nine unreached test targets.
Initial narrower-package continuations passed, but rebuilt feature graphs; those
runs remain additional evidence rather than final workspace coverage.
After the controller flagged that distinction, `cargo test --workspace --no-run
--message-format=json` refreshed the workspace artifacts after the schema edit.
The nine unreached executables plus corrected storage target were then invoked
directly, from each package's directory, after verifying their original workspace
executable paths and feature lists. All ten passed. Exact executable filenames
from the successful initial targets plus this continuation match **91/91**
workspace artifacts, with none missing or duplicated. Doctests ran separately
with `cargo test --workspace --doc`. Final totals exclude superseded/extra
attempts and count each logical workspace target once.

Focused checks from earlier task commits, as recorded by the executor:
Task 1 crypto 6, persistence 15, code map 1, contracts 1;
Task 2 profile 7, AYLUL session 11, binary startup 11, code map/contracts;
Task 3 HTTP 7, web 5, OpenAPI 6, code map/contracts and generated-type/typecheck checks.
These focused results are separate from the full-gate totals below.

## Files at or above 300 lines

Counts compare the branch base with the final checkout. No changed file newly
crosses 300. New runtime modules and test modules remain below 300.

| File | Base | Current | Single job and why the addition belongs |
|---|---:|---:|---|
| `Cargo.lock` | 4028 | 4232 | Generated dependency resolution; crypto/credential infrastructure belongs in the one lockfile |
| `api/openapi.json` | 9529 | 9791 | Generated HTTP protocol; new routes belong to this canonical export |
| `crates/shadows-core/src/app.rs` | 323 | 336 | AppCore composition; the eleventh service is composed here |
| `crates/shadows-core/src/harness/contract.yaml` | 756 | 772 | Harness obligations; encrypted opening failures belong to this owner |
| `crates/shadows-core/src/harness/sessions.rs` | 490 | 494 | Live thread adapter ownership; only shared-service/config/error plumbing was added |
| `crates/shadows-core/tests/aylul_session.rs` | 320 | 479 | Isolated AYLUL opening acceptance; replacement/removal use the same opening path |
| `crates/shadows-core/tests/design_outcomes.rs` | 466 | 467 | Design outcome acceptance; one required CoreParts fixture field |
| `crates/shadows-core/tests/design_parts.rs` | 490 | 491 | Design part acceptance; one required CoreParts fixture field |
| `crates/shadows/tests/cors.rs` | 316 | 317 | HTTP origin acceptance; one required CoreParts fixture field |
| `crates/shadows/tests/fixtures/app.rs` | 442 | 454 | In-process application fixture; deterministic injected keys belong to composition |
| `crates/shadows/tests/openapi.rs` | 314 | 318 | Canonical HTTP protocol acceptance; added route inventory belongs here |
| `crates/shadows/tests/planner_turn.rs` | 411 | 412 | Planner turn acceptance; one required CoreParts fixture field |
| `crates/shadows/tests/project_directory.rs` | 353 | 354 | Project directory acceptance; one required CoreParts fixture field |
| `crates/shadows/tests/protocol.rs` | 504 | 505 | HTTP capability acceptance; one required CoreParts fixture field retains its existing responsibility |
| `crates/shadows/tests/resync.rs` | 499 | 500 | Durable stream handoff acceptance; one required fixture field reaches 500 without adding a responsibility |
| `crates/shadows/tests/shutdown.rs` | 403 | 404 | Daemon shutdown acceptance; one required CoreParts fixture field |
| `docs/superpowers/specs/2026-09-26-application-core-design.md` | 538 | 539 | AppCore design authority; the service list belongs to its existing owner section |
| `web/src/api/contract.yaml` | 525 | 532 | Client transport obligations; new metadata-only calls belong to this surface |
| `web/src/api/schema.d.ts` | 6016 | 6239 | Generated client protocol types; retained as one generated artifact |

The already-large generated artifacts/owner documents retain their existing
ownership. `protocol.rs` already exceeded 500, while `resync.rs` reaches it through
a mechanical fixture field; neither gains another domain or warrants a new split
for this change. `web/src/api/client.ts` is unchanged at 549 lines.

## Rulings and deviations

- Task 4 originally had no commit title. A docs-only title was initially chosen;
  the verified runtime fix warrants `fix(aylul): serialize master-key creation and record UI gate`.
- Browser acceptance belongs to the controller. This file records automated
  facts before that journey, adapting the plan's post-journey evidence timing.
  Its cost is that this record cannot establish live acceptance.
- No Shadows MCP or Rust LSP tool is connected. Named source files were read
  for signatures after the code maps/contracts, as the user allowed.
- CoreParts injection required mechanical test-fixture updates and exhaustive
  adapter error arms in Task 1; HTTP statuses arrived with those arms.
- Task 1 linking exhausted disk. Automatic approval review rejected broad cache
  deletion; narrower `cargo clean -p shadows-core` recovered generated outputs.
  A second cleanup of workspace packages during Task 2 recovered debug duplicates.
  Subsequent builds disable incremental/debug data and retain the normal target.
- Task 2 preserves optional startup input as `Option<(AylulConfig, Option<PathBuf>)>`.
  Non-Windows key loading explicitly fails rather than selecting a mock fallback.
- Task 2 used `cargo test -p shadows --lib aylul` for startup tests to avoid linking
  unrelated integration binaries. The final workspace gate remains unfiltered.
- After the workspace command stopped, only its failed/unreached targets were
  rerun. A first logical-name coverage check was insufficient after narrower
  builds changed features. Exact workspace artifacts were refreshed and run;
  the cost of omitting that correction would have been incomplete gate evidence.
- Task 3's HTTP fixture uses an inert test-only agent adapter because all cases
  stop before spawn. Shipping adapter dependencies remain core-only.
- The read-only OpenAPI fixture configures AYLUL with a synthetic key, exercising
  successful env GETs without weakening the no-write check. Three new routes
  were added to the route inventory; duplicate map rows were corrected.
- The new per-user lock is an implementation of the specified concurrency intent.
  If its directory is unavailable the service stays unavailable; it does not
  overwrite credentials or fall back to plaintext.
- Dependencies `aes-gcm`, `hkdf`, `hmac`, `rand` and `keyring` are infrastructure:
  cryptography, OS randomness and native credential access. Product dotenv parsing
  remains local. Task 4 adds no dependency.
- The unrelated external edit in `CLAUDE.md` is preserved unstaged/uncommitted.

## Controller-owned live acceptance - NOT RUN

1. **NOT RUN:** add `API_KEY` in Settings, move the import source away,
   restart the daemon and obtain an AYLUL reply on a database copy.
2. **NOT RUN:** byte-search that live database, WAL and SHM for the submitted value.
3. **NOT RUN:** replace it and confirm a new live conversation uses the replacement.
4. **NOT RUN:** remove it without a daemon fallback and observe
   `AYLUL_KEY_MISSING` naming `API_KEY`.

Automated synthetic database/adapter checks cover corresponding boundaries, but
do not substitute for this browser/keyring journey. No real `.env` file,
`C:\Users\Mohammed\.claude` or user Claude state was accessed. No push, PR,
merge, branch switch or production action was performed.
