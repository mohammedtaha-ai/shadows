# Task A3 independent review

Reviewed `29e73092b6ff7d0b5f89d7f67134069e77fd45c0..250a4d0f13d7b67c4da295f4a0acb2f059e60b65` against the A3 brief and harness-controls spec §12.2–12.3. Read the prepared review diff once, then inspected only the named implementation files and the process contract needed to assess cleanup.

Correction commit: `7954d9a1d3e1c45ec4d6f1b5324128e87f1b45b9`.

## Verdicts

- **Spec compliance: pass for the A3 slice, with a spec/plan ambiguity to resolve before A4.** `Sessions::open` reuses a live adapter, reads durable context, starts an ACP session, and sets `acceptEdits` (`src/planner/sessions.rs:98-160`). The mode matches the policy default in spec §12.4. `take_events` marks a turn active for idle reaping (`src/planner/sessions.rs:162-170,224-241`). `terminate` and `close_all` use process-tree termination and reaping (`src/planner/sessions.rs:189-222,250-255`). Fork persistence and the HTTP session route are later tasks, per the A3 brief and implementer report.
- **Code quality: pass after two in-scope corrections.** The original commit had a possible indefinite wait on a transport-closed process and discarded failed shutdown handles. The correction keeps the failed handle available and uses the managed termination path before replacement. Focused Windows gates pass.

## Findings and disposition

1. **P1 fixed: dead transport could block every thread's `open`.** Original `src/planner/sessions.rs:100-108` recognized `Connection::is_closed()` but called `ProcessHandle::wait()` without terminating. The process contract at `src/process/mod.rs:92-108` waits until the leader exits. A transport-closed adapter whose process remains alive therefore holds the global registry mutex indefinitely, blocking all opens, termination, and reaping. Correction calls `stop_handle` before removal; a termination error returns `OpenError::Start` and retains the handle for retry. The existing dead-connection replacement test passes, though it exercises a process that exits promptly rather than this exact live-process case.
2. **P1 fixed: `close_all` forgot handles that failed termination.** Original `src/planner/sessions.rs:195-210` cleared the registry even if `stop_handle` returned an error. Correction removes only successfully stopped handles and returns the first error. This preserves ownership for a retry and avoids presenting an unconfirmed close as complete. There is no deterministic failure-injection hook on `Sessions`, so this branch was statically checked rather than dynamically forced.
3. **P2 controller decision: opening a thread without a completed first turn, letting it idle, then reopening cannot resume the same harness session.** The id is held only in `Live` (`src/planner/sessions.rs:150-158`) and is deliberately not recorded until turn end by spec §12.3. Spec §12.2 says the next opening after idle resumes the same session. A3's idle test (`tests/sessions.rs:119-138`) pre-records a durable id, so it does not cover this case. These two spec statements need an explicit ruling; I did not persist an id early or change the reaper because that would silently choose between them.
4. **P2 A4 integration risk: event handoff needs a turn-scoped guarantee.** The reaper skips an entry whenever `events` is `None` (`src/planner/sessions.rs:228-230`), while `take_events` removes the receiver and `give_back_events` restores it (`:162-180`). If the future A4 turn exits before calling `give_back_events`, the adapter is never considered idle. A4 should use a cleanup guard or equivalent finalization on success, error, and cancellation. No current A3 caller runs a turn through Sessions.
5. **P3 intentional lock scope, monitor during A4.** A3's prescribed single mutex is held across storage reads, ACP initialize/session/mode calls, and process termination (`src/planner/sessions.rs:99-159,189-241`). A slow or hung adapter for thread A delays independent thread B. This follows the brief's explicit one-mutex design, so I did not replace it with per-thread coordination in this review. A4 should provide bounded setup/close behavior before exposing concurrent HTTP opens.

## Strengths

- The directory is validated before spawning (`src/planner/sessions.rs:112-119,259-272`); missing directories cannot silently run in the daemon's cwd.
- A setup error attempts managed tree termination and reaping (`src/planner/sessions.rs:141-148`), and the reaper preserves a handle when idle close fails (`:233-241`).
- Tests cover reuse, durable resume, idle close, dead process replacement, missing directory, active turn reaper skip, and explicit termination (`tests/sessions.rs`).
- `Connection::is_closed` (`src/agent/acp.rs:58-61`) observes transport closure before process exit, closing the detection gap found by the implementer.

## Verification

- Reviewer, Windows: `Remove-Item Env:RUST_LOG -ErrorAction SilentlyContinue; cargo test --test sessions`: **7 passed**.
- Reviewer, Windows: `cargo clippy --all-targets -- -D warnings`: **exit 0**.
- Reviewer, Windows: `UPDATE_CODEMAP=1 cargo test --test codemap`: **2 passed** and inventory regenerated.
- Reviewer, Windows: `cargo fmt --all -- --check` and `git diff --check`: **exit 0**.
- Implementer reports a full Windows `cargo test` exit 0. I did not repeat it. No real Claude adapter acceptance or Linux CI run was performed in this review.

## Follow-up after controller ruling

Binding spec §12.2 was amended in `ad6f5a1`: an idle close before the first turn finishes discards the unrecorded id, and the next open creates a new session. This resolves finding 3 above; its earlier ambiguity assessment is superseded by the amended spec. Added `idle_close_before_first_turn_discards_the_unrecorded_session` in `tests/sessions.rs`. It checks that no durable id exists, waits for the reaper with a bounded poll, then verifies a new opening and working prompt. No implementation change was needed.

Follow-up test commit: `9c89856667f8daa94b24afa5460bf26304f541ba`.

Reviewer follow-up on Windows: `Remove-Item Env:RUST_LOG -ErrorAction SilentlyContinue; cargo test --test sessions`: **8 passed**. `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --test codemap` (**2 passed**), and `git diff --check`: **exit 0**. The full suite was not repeated.
