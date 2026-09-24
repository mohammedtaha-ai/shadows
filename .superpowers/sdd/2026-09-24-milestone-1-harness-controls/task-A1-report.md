# Task A1 report — 2026-09-24

## Status

DONE_WITH_CONCERNS. A1 is implemented and verified on Windows. This task does not establish a live ACP session or Linux behavior; those are later integration work and were not claimed here.

Commit: `a093401e963c1b2022650aa3927bed430f8281c0` on `milestone-1/harness-controls`. The report resides under the repository's ignored `.superpowers/` task area and is not in the commit. The tracked working tree was clean immediately after the commit.

## Implementation and inherited work

The working tree already held uncommitted A1 changes when this pass began. I preserved and reviewed them before editing. The inherited changes pinned `@agentclientprotocol/claude-agent-acp` to 0.81.1 in `harness/claude/package.json` and its lockfile; added the daemon-owned harness README and node_modules ignore rule; pinned Rust `agent-client-protocol` to 2.2.0 with `unstable_session_fork` and enabled `tokio-util` compat; added Node, adapter, and Claude paths to `Config`; added absolute/existing-path validation and adapter package version lookup; exposed piped child stdin/stdout/stderr and `take_stdio`; added a `tree_probe` echo mode and focused tests; and updated the generated code inventory. Existing `ProcessSpec` constructors passed `pipe_stdin: false`.

I fixed `take_stdio` so a missing channel returns `None` without consuming another channel. The full suite initially exposed two `serve_smoke` failures: the inherited CLI change made `--node` and `--adapter` required, while those tests still launched without them. I updated the smoke fixtures to provide existing explicit paths. Clippy then found a collapsible conditional in the inherited process change; I simplified it.

The task brief named `src/cli/mod.rs` for the flags, but the actual Clap entry point was `src/main.rs`. The first inherited implementation added flags there. On the user's correction, I moved Clap definitions, path parsers, and dispatch to `src/cli/args.rs`, re-exported it from `src/cli/mod.rs`, and reduced `src/main.rs` to parse and run. The code map now names the CLI module's daemon startup responsibility.

## Files

- Adapter packaging: `.gitignore`, `harness/README.md`, `harness/claude/package.json`, `harness/claude/package-lock.json`.
- Rust dependencies: `Cargo.toml`, `Cargo.lock`.
- Runtime and CLI: `src/config.rs`, `src/main.rs`, `src/cli/mod.rs`, `src/cli/args.rs`, `src/process/mod.rs`, `src/agent/claude.rs`, `src/bin/tree_probe.rs`.
- Tests and map: `tests/containment.rs`, `tests/harness_config.rs`, `tests/serve_smoke.rs`, `docs/codebase/README.md`, `docs/codebase/inventory.md`.

## Verification

All commands ran on Windows in `E:\Globalprojects\shadows`. `RUST_LOG` was removed from the command environment for test runs because the inherited shell value `warn` breaks the debug-log test.

1. `cargo test --test containment --test harness_config`: 8 + 3 passed, 0 failed.
2. First `cargo test`: the prior suites passed, then both `serve_smoke` tests failed because required `--node` and `--adapter` flags were absent. Fixed the fixtures.
3. `cargo test --test serve_smoke`: 2 passed, 0 failed.
4. An intermediate complete `cargo test`: 109 passed, 0 failed.
5. First `cargo clippy --all-targets -- -D warnings`: failed on `clippy::collapsible_if` in `src/process/mod.rs`. Fixed it.
6. After moving CLI definitions, `UPDATE_CODEMAP=1 cargo test --test codemap` regenerated inventory but its ownership test rejected a comma-separated pair of reference files. Fixed the README row to reference `src/cli/args.rs`.
7. Final `cargo test --test codemap`: 2 passed, 0 failed. Final `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, and `git diff --check`: all exit 0.
8. Final `cargo test`: 109 passed, 0 failed (including 2 code-map and 2 serve-smoke tests).
9. `npm ls --depth=0` in `harness/claude`: reports installed `@agentclientprotocol/claude-agent-acp@0.81.1` and exits 0.

The inherited work had no recoverable output showing the prescribed initial red test run, so I cannot claim a verified TDD red/green sequence. The observed smoke failures and their subsequent passing rerun are direct regression evidence, but not the original planned red step.

## Self-review and concerns

- Compared the produced fields and behavior against the A1 brief and design §12.2. `Config` has three explicit paths; CLI parsing applies `config::harness_path` to each; `adapter_version` reads the package two directories above the entry and returns `unknown` on read/parse failure; interactive stdio is taken once; previous process constructors keep null stdin.
- The adapter's stderr is left available to the ACP caller for its later debug-log forwarding. A1 does not start the adapter; A2 owns the ACP session integration.
- The pinned Rust crate is added but not yet called by product code in A1. The large lockfile change follows from that dependency.
- No live Claude/ACP acceptance run or Linux CI run was performed here. Do not treat these Windows test results as those validations.
