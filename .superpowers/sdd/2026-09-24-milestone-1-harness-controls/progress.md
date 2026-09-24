# SDD ledger — plan: docs/superpowers/plans/2026-09-24-milestone-1-harness-controls.md

Scope: Phase A only, Tasks A1-A5. Task 0 is already complete at b98b565. Branch milestone-1/harness-controls, base b98b565.
Ruling: Use Luna/high for implementers and reviewers — explicit user request overrides the plan's Opus instruction — risk: some integration work may need escalation.
Ruling: Work on the existing dedicated, clean milestone-1/harness-controls branch — the plan explicitly names it for Phase A — risk: no separate linked worktree for this run.
Ruling: Generate A1-A5 briefs by their headings because the skill task-brief script only recognizes numeric Task N headings — risk: extraction must be checked against the plan.

Preflight scan (spec §12.2-12.3 is authority):
| Tasks | Producer / consumer or shared file | Finding |
|---|---|---|
| A1 → A2 | process stdio and harness paths feed ClaudeAdapter and ACP Connection | Aligned; A2 leaves old stream-json caller intact. |
| A1 → A3 | Config paths and process handle feed Sessions | Aligned; A3 owns session lifecycle. |
| A1 → A4 | Config and process changes feed Planner migration | Aligned; A4 removes old invocation. |
| A2 → A3 | Connection, Opened, HarnessEvent feed Sessions | Aligned; A3 must retain event receiver ownership. |
| A2 → A4 | HarnessEvent and ACP prompt replace stream-json events | Aligned; A4 collector uses final tool title. |
| A3 → A4 | Sessions::open and terminate support turn/stop | Aligned; cancellation ownership remains in PlannerTurn. |
| A3 → A5 | session resume and shutdown lifecycle support restart test | Aligned. |
| A4 → A5 | completed turn records session ID; A5 tests recovery | Aligned. |
| A1 self | Tests named stdio/version match implementation files | Aligned. |
| A2 self | Fake ACP and client tests cover one connection; old caller stays | Aligned. |
| A3 self | idle/close/reopen tests match Sessions API | Aligned. |
| A4 self | stop/crash/permission tests match turn rewrite | Aligned; test migration broad. |
| A5 self | recovery test and real run evidence depend on working daemon | Aligned; browser run is an acceptance gate before Phase B. |

Progress:
Baseline: cargo test failed before edits in tests/debug_log.rs:104 (missing debug-level-probe); cargo test --test debug_log repeated the same failure. Other suites were not reached.
Ruling: Proceed with Phase A; keep the baseline failure visible and investigate when the touched CLI/tracing path or final gate gives more evidence — risk: full-suite status is unavailable until resolved.
Baseline diagnosis: Env:RUST_LOG=warn overrides tracing debug filter; clearing it makes cargo test --test debug_log pass 1/1 before edits. Agent A1 notified.
Task A1: dispatched to /root/a1_implement (gpt-6-luna, high), base b98b565; report task-A1-report.md.
Ruling A1: serve flags live in src/main.rs in current code, so implementation may touch that entry point instead of plan-named src/cli/mod.rs; preserving actual ownership is more important than stale path text. Cost if wrong: code map/CLI integration review will catch it.
Task A1: Luna/high implementer was stopped by user before report/commit. Working tree contains uncommitted A1 changes; no A1 completion line.
Ruling: Sol/medium takes over A1 from the existing uncommitted tree, preserves that work and verifies it before committing — explicit user request.
Ruling A1 update (user correction): Prefer the plan's CLI ownership split. Agent instructed to move Clap argument definitions/parsers from main.rs into focused cli module code, leaving main.rs thin; this supersedes earlier acceptance of main.rs placement. Risk: extra module/code-map work, covered by A1 review.
Task A1 implementer: DONE_WITH_CONCERNS, commit a093401e963c1b2022650aa3927bed430f8281c0; report task-A1-report.md. Final Windows cargo test 109/109; clippy/fmt/codemap pass. No live ACP/Linux; original RED artifact absent due takeover.
Review package: review-b98b565..a093401.diff (1 commit).
Ruling A1 review: CLAUDE.md project rule says a reviewer fixes what it finds, overriding the generic read-only task-reviewer template. Reviewer may fix/commit focused findings and must report exact diff/test evidence; controller verifies.
Task A1: complete (a093401e963c1b2022650aa3927bed430f8281c0). Independent review PASS spec and quality; reviewer focused Windows 11/11 and diff check pass, no fix commit. Original RED artifact unavailable; live ACP/Linux left to later gates.
Task A2: dispatched to /root/a2_implement (gpt-6-sol, medium), base a093401e963c1b2022650aa3927bed430f8281c0; report task-A2-report.md.
Task A2 implementer: DONE, commit a5df0c4f74100ecc5fd77847fa2258da664a34e2. Report task-A2-report.md: ACP 11/11, full Windows 120/120, clippy/fmt/codemap pass; real adapter not run.
Review package: review-a093401..a5df0c4.diff (1 commit).
Task A2: complete (implementer a5df0c4f74100ecc5fd77847fa2258da664a34e2; reviewer fix 29e73092b6ff7d0b5f89d7f67134069e77fd45c0). Review PASS spec and quality after fixes. Reviewer Windows ACP 12/12, one lib test, codemap 2/2, clippy/fmt/diff check; no full suite after fix, real adapter/Linux still pending.
Task A3: dispatched to /root/a3_implement (gpt-6-sol, medium), base 29e73092b6ff7d0b5f89d7f67134069e77fd45c0; report task-A3-report.md.
Task A3 implementer: DONE_WITH_CONCERNS, commit 250a4d0f13d7b67c4da295f4a0acb2f059e60b65. Report task-A3-report.md: Sessions 7/7, full Windows cargo test exit 0, clippy/fmt/codemap pass; Planner wiring and real ACP left for A4/A5.
Review package: review-29e7309..250a4d0.diff (1 commit).
Task A3 reviewer correction: 7954d9a1d3e1c45ec4d6f1b5324128e87f1b45b9, PASS A3 slice after fixing dead transport cleanup and close_all handle retention; Sessions 7/7 focused, codemap 2/2, clippy/fmt.
Ruling: §12.3 forbids persisting session id before first turn end. Therefore §12.2 idle reopen resumes only a durably recorded session; pre-first-turn idle close discards unrecorded id and next opening creates a new session. Spec clarified in ad6f5a1. Cost if wrong: a session opened without a turn then left idle restarts, but no conversation content is lost.
Task A3 reviewer follow-up: add deterministic pre-first-turn idle close/new session test after ruling, then re-report.
A4 handoff risk: event receiver must be returned on every turn exit path to avoid permanent reaper skip; ensure setup/close bounds while global Sessions mutex is held.
Task A3: complete (implementer 250a4d0f13d7b67c4da295f4a0acb2f059e60b65; reviewer fix 7954d9a1d3e1c45ec4d6f1b5324128e87f1b45b9; spec clarification ad6f5a1; follow-up test 9c89856667f8daa94b24afa5460bf26304f541ba). Review PASS after fixes/ruling. Reviewer Windows Sessions 8/8, codemap 2/2, fmt/clippy.
Task A4: dispatched to /root/a4_implement (gpt-6-sol, medium), base 9c89856667f8daa94b24afa5460bf26304f541ba; report task-A4-report.md.
Task A4: complete in the cloud session (Linux gate 131/131, clippy/fmt/codemap, web 48/48); report task-A4-report.md. Not run on Windows or against the real adapter — A5 run pending.
