# Task 1 Report: Crate scaffold, `shadows serve`, structured tracing

## What was implemented

Created the `shadows` crate exactly as specified in the task brief:

- `Cargo.toml` — package manifest, edition 2024, `rust-version = "1.94"`, `publish = false`, dependencies and dev-dependencies verbatim from the brief.
- `src/error.rs` — `ErrorCode`, `FailureClass`, `RetryClass`, `AppFailure` (public-safe, `Display` + `Error`), `FailureReport` (internal causal chain, deliberately not `Serialize`).
- `src/config.rs` — `Config { db_path, bind, harness_path }`.
- `src/tracing.rs` — `init(verbose: bool)` using `tracing_subscriber::fmt` with `EnvFilter`.
- `src/lib.rs` — declares only `cli`, `config`, `error`, `tracing` modules (no other module scaffolded).
- `src/cli/mod.rs` — `serve(config: Config) -> anyhow::Result<()>`: binds a `TcpListener`, prints exactly one line (`shadows serve listening on http://{addr}`), logs via `tracing::info!`, mounts a minimal `/health` route, and calls `axum::serve`. Never opens a browser.
- `src/main.rs` — `clap`-derived CLI with a `serve` subcommand (`--db`, `--bind`, `--harness`, global `--verbose`); `anyhow` used only at this bootstrap boundary.
- `tests/serve_smoke.rs` — spawns the binary with `serve --db <tmp> --bind 127.0.0.1:0`, asserts the first stdout line starts with `shadows serve listening on http://127.0.0.1:`, kills the child.

No other modules (`storage/`, `runtime/`, `project/`, etc.) were created — `src/lib.rs` declares only the four modules Task 1 owns. `.gitignore` and `.gitattributes` were not modified. `sandbox/` was left untouched; no `[workspace]` table was added to `Cargo.toml`, so `sandbox/serve-stream-spike` and `sandbox/wal-validation` (each with their own `Cargo.toml`) are not swept in — confirmed by `cargo test`/`cargo clippy` only building the `shadows` crate and its dependency graph, with no errors or references to the sandbox crates.

## TDD evidence

**RED** — before the crate existed:

```
$ cargo test --test serve_smoke
error: could not find `Cargo.toml` in `E:\Globalprojects\shadows` or any parent directory
```

Expected failure: no `Cargo.toml`/crate existed yet at the repo root, so the test target could not even be resolved, let alone compiled. This matches the brief's Step 2 expectation ("FAIL — the crate does not exist yet, so this does not compile").

**GREEN** — after implementing Steps 3–9:

```
$ cargo test --test serve_smoke
running 1 test
test serve_prints_one_local_address_and_does_not_open_a_browser ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
```

Full suite afterward:

```
$ cargo test
unittests src\lib.rs  -> 0 tests, ok
unittests src\main.rs -> 0 tests, ok
tests\serve_smoke.rs  -> 1 passed; 0 failed
Doc-tests shadows     -> 0 tests, ok
```

Lint/format gates:

```
$ cargo clippy --all-targets -- -D warnings
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.87s   (no warnings/errors)

$ cargo fmt --check
(no output — clean, after running `cargo fmt` once to normalize the brief's
 code blocks to project rustfmt style; only whitespace/line-break changes to
 src/tracing.rs, src/main.rs, tests/serve_smoke.rs — no semantic change)
```

## Files changed

- `Cargo.toml` (new)
- `Cargo.lock` (new, generated)
- `src/main.rs` (new)
- `src/lib.rs` (new)
- `src/config.rs` (new)
- `src/tracing.rs` (new)
- `src/cli/mod.rs` (new)
- `src/error.rs` (new)
- `tests/serve_smoke.rs` (new)

## Self-review findings

- Completeness: every file and interface listed in the brief's "Files"/"Interfaces" sections was created; `serve` signature, `Config` fields, `tracing::init` signature, and `error` types match verbatim.
- Naming: matches the brief exactly (`AppFailure`, `ErrorCode`, `FailureClass`, `RetryClass`, `Config`, `serve`, `init`).
- YAGNI: no extra modules, no extra CLI subcommands, no extra routes beyond the brief's `/health`, no premature use of `sqlx`/`uuid`/`serde_json`/`tower-http` beyond declaring them as dependencies (they are required verbatim by the brief's `Cargo.toml` for later tasks; Task 1 itself doesn't need to use them yet, which is expected since this is scaffold-only).
- Test quality: the test spawns the real binary end-to-end (not a unit test double), asserts on the literal first stdout line contract the brief specifies, and cleans up the child process. Output is pristine — no clippy or fmt warnings anywhere in the crate.
- One deliberate deviation from a literal byte-for-byte copy of the brief's code blocks: ran `cargo fmt` to satisfy the brief's own Step 10 requirement ("cargo fmt --check … Expected: … clean"), which reformatted `src/tracing.rs`, `src/main.rs`, and `tests/serve_smoke.rs` whitespace only (e.g. multi-line `if`/struct-literal formatting, import ordering). No logic, names, or values changed.

## Concerns

None. `rustc`/`cargo` 1.96.0 on this machine satisfies the `rust-version = "1.94"` floor. `sandbox/serve-stream-spike` and `sandbox/wal-validation` were confirmed not to be swept into any workspace (no `[workspace]` table was added).

## Fix round 1

**Finding addressed (Important):** `src/cli/mod.rs` mounted a `/health` route that no test exercises, no client calls, and no spec section asks for — an operational decision the spec had not made. The `axum::serve` call and the running daemon were correctly kept (that part of the original review constraint was overruled by the coordinator as over-tight).

**Change:** removed the route, keeping an empty router:

```rust
// before
let app = axum::Router::new().route("/health", axum::routing::get(|| async { "ok" }));
axum::serve(listener, app).await?;

// after
let app = axum::Router::new();
axum::serve(listener, app).await?;
```

**Covering test and command:**

```
$ cargo test --test serve_smoke
running 1 test
test serve_prints_one_local_address_and_does_not_open_a_browser ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s
```

**Lint/format re-check** (to rule out an unused-import or dead-code warning from the removal):

```
$ cargo clippy --all-targets -- -D warnings
    Checking shadows v0.1.0 (E:\Globalprojects\shadows)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.93s
(no warnings/errors)

$ cargo fmt --check
(no output — clean)
```

No unused imports resulted; `axum::routing::get` was never imported separately (called fully-qualified inline), so nothing was left dangling.

**Commit:** `git commit -m "fix: drop the unspec'd /health route from shadows serve"` (see git log for SHA).
