# Desktop architecture branch: the whole gate before its PR

Date: 2026-10-26. Branch `desktop-architecture`, tree at commit `83574074`. Windows 11,
run from the main checkout over each workspace's existing build cache. This is task N-21
(D-12 of `docs/aylul/native/plans/2026-10-25-desktop-architecture.md`).

## First run: disk full

The first run failed `cargo test --workspace` and `cargo build -p shadows --bin shadows` at
link time with `LLVM ERROR: IO failure on output stream: no space on device`: drive E: had
35 MB free. `tests/companion/presence.spec.ts:91` ("plain mixed-direction text wraps") timed
out waiting for the "Mixed text" button in the same run. Deleting the older copies of test
binaries in `target/debug/deps` (334 in Core, 11 in the desktop workspace; `.exe`, `.pdb`,
`.d`, never the dependency cache) freed 32 GB. Every step below is from the second run, on
the same tree, where that browser test passed.

## Core (`.github/workflows/ci.yml`, Windows job)

| Step | Result |
|---|---|
| `cargo fmt --all --check` | pass |
| Line length (`crates/**/*.rs` at most 100 characters) | pass |
| `cargo clippy --workspace --all-targets --features fake-acp/test-support -- -D warnings` | pass |
| `cargo clippy --workspace -- -D warnings` | pass |
| `cargo build`; `cargo tree -e features,no-dev --workspace` has no `test-support` | pass |
| `cargo test --workspace` | pass: 838 passed, 0 failed, 3 ignored |
| `git diff --exit-code api/` | pass |

## AYLUL (`.github/workflows/aylul.yml`)

| Step | Result |
|---|---|
| `npm ci` (desktop, web) | pass |
| `npm run build` | pass |
| `cargo fmt --manifest-path src-tauri/Cargo.toml --all --check` | pass |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | pass |
| `cargo build -p shadows --bin shadows` | pass |
| `cargo test --manifest-path src-tauri/Cargo.toml` | pass: 181 passed, 0 failed, 2 ignored |
| `npm test` | pass: 9 |
| `npm run typecheck:tests` | pass |
| `npm run test:companion:unit` | pass: 53 |
| `npm run test:companion:production` | pass: 2 |
| Hook `cargo fmt`, `cargo clippy --all-targets -D warnings`, `cargo test` | pass: 12 tests |
| Hook `cargo build --release` | not run: no `--release` builds locally; CI runs it |
| Sidecar `npm ci`, `npm run typecheck` | pass |
| Sidecar `npm test` | pass: 86, with `AYLUL_HOOK_EXE` set to the debug hook so both round-trip tests ran |
| `npm run test:companion:browser` (separate job) | pass: 56 passed, 2 skipped |

## Web (`.github/workflows/ci.yml`, `web` job)

| Step | Result |
|---|---|
| `npm ci` | pass |
| `npm run gen:api`; `git diff --exit-code -- src/api` | pass |
| `npm run typecheck` | pass |
| `npm run lint` | pass |
| `npm test` | pass: 96 files, 618 tests |
| `npx vite build` | pass |

## Shrinking baselines

Each check that holds a baseline passed above, and an exact baseline fails on a stale entry,
so these counts are what the final tree has.

| Baseline | Entries |
|---|---|
| Native ratchet (`apps/desktop/src-tauri/tests/architecture/baseline.txt`) | 145: owner_edge 41, private_access 42, construction 37, managed_state 24, coverage 1; command parity 0 |
| Web architecture (`web/src/architecture.baseline.txt`) | 35: cache_write 14, public_entry 10, dependency 9, server_access 2; entry 0 |
| Companion boundaries (`apps/desktop/tests/companion/fixtures/boundaries/boundaries-baseline.txt`) | 0 |

## Not covered here, open for Mohammed

- The plan's section 8 item 4: real DPI transitions, monitor removal, suspend and resume,
  optimized resource measurements, and launch at OS sign-in. C-6 records these.
- A run against a real provider. None was made.
- The hook's release build and the browser suite on a clean CI runner: CI runs both.
