# Task A2 independent review

Reviewed `a093401e963c1b2022650aa3927bed430f8281c0..a5df0c4f74100ecc5fd77847fa2258da664a34e2` against the A2 brief, binding spec §12.2–12.3, and `docs/evidence/harness/ACP_PROBE.md`. The reviewer correction is `29e73092b6ff7d0b5f89d7f67134069e77fd45c0`.

## Verdicts

- **Spec compliance: pass for the A2 deliverable after correction.** `Connection::open` takes managed process stdio and initializes ACP (`src/agent/acp.rs:59-117`). `start_session` sends new, resume, or fork followed by resume and returns the last option set (`src/agent/acp.rs:120-160`). Permission requests select `reject_once` or `reject_always`, report the tool title, and never select an allow option (`src/agent/acp.rs:77-98`). Updates preserve message IDs, tool title/status changes, usage metadata, and option sets (`src/agent/acp.rs:227-259`). The transport EOF integration case and fork, permission, tool, usage, and model cases pass (`tests/acp_connection.rs:43-201`).
- **Code quality: pass after two focused fixes.** The ACP connection remains in its assigned module, the fixture uses the pinned ACP crate, and the existing stream-json caller remains intact for A4. The changed module has one responsibility despite approaching 300 lines (`src/agent/acp.rs:1`, `docs/codebase/README.md:23`).

## Findings, prioritized

1. **P2, fixed — refused turns exposed Rust variant names instead of ACP reasons.** The original catch-all `format!("{other:?}")` returned `MaxTokens`, although §12.3 and the A2 brief require the `max_tokens` reason. The new integration test failed with `left: Refused("MaxTokens")`, `right: Refused("max_tokens")`. The mapping now explicitly returns `max_tokens`, `max_turn_requests`, and `refusal` (`src/agent/acp.rs:192-199`; `tests/acp_connection.rs:204-217`).
2. **P2, fixed — a server RPC message could be mistaken for transport closure.** The original classifier matched the words `connection closed` or `transport closed` in any RPC message. A unit test proved `Error::new(-32603, "connection closed by policy")` was classified `Closed`. The pinned library exposes `is_incoming_transport_closed`, which checks its structured EOF marker; classification now uses that marker and keeps ordinary server errors as `Rpc` (`src/agent/acp.rs:209-224`, `src/agent/acp.rs:281-290`). The existing process-exit integration test still confirms `Closed` on a real child exit (`tests/acp_connection.rs:105-115`).

## Scope and remaining evidence

- A2 exposes raw session choices; it does not itself apply the spec's `acceptEdits` opening default or persist `PermissionRefused` in thread storage. These are integration responsibilities in later Phase A tasks. If the controller intended A2 alone to meet those full §12.2 behaviors, reconcile that with the narrower A2 brief before starting the next task.
- This review exercised the crate-based fake agent on Windows. It did not run the real Claude adapter, Linux, or the full suite. The implementer reported the pre-correction full suite at 120/120; that count is not independent review evidence.

## Review verification

- RED: `cargo test --test acp_connection a_refused_prompt_names_the_acp_stop_reason` failed as expected (`MaxTokens` versus `max_tokens`).
- RED: `cargo test --lib a_server_error_mentioning_connection_closed_is_still_an_rpc_error` failed as expected.
- GREEN after correction, with `Env:RUST_LOG` cleared: `cargo test --test acp_connection` — 12 passed, 0 failed; targeted library test — 1 passed, 0 failed.
- `UPDATE_CODEMAP=1 cargo test --test codemap` — 2 passed; generated inventory updated. `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, and `git diff --check` exited 0.
