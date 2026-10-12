# Guided planning PR 3b (§24) — gate and acceptance, 2026-10-12

Branch `next/guided-planning-pr3b`. A measurement record, not design
authority. Codex executed Tasks 1–5 of the plan (commits `809185e` through
`d1ec47b`) and stopped on its usage limit inside Task 6 with one failing
test; Claude finished Task 6.

## What Claude fixed after Codex stopped

- `turns/contract.yaml` named `approval_authority` under
  `shared_in_transaction`, which is not a store function; the `contracts`
  test failed. The sentence moved into `check_running_turn_in`'s entry
  (`ce773ae`, with Codex's uncommitted contract edits).
- The Planner prompt said "Approval follows §24", which the Planner cannot
  read; it now names the button and the approval tool.

## Review

Read: `db/approval.rs`, `command/approver.rs`, migration 0024, the three
approval stores, the Planner core methods, the prompt and session lock.
`authorise_in` runs before `classify` in all three stores, so a revoked grant
or an ended turn cannot replay or write. The migration drops and recreates
`workflow_frozen_immutable` around the backfill; the recreated trigger is
byte-for-byte the 0012 one. Planner approvals use `writer.actor()` for
events and the PlanApproved entry. The waiver refusal sits inside the map
approval transaction.

## Gate

At `ce773ae` plus the prompt fix, `RUST_LOG` unset: `cargo fmt --check`,
both Clippy runs, the `test-support` tree check and `git diff --exit-code
api/` passed; `cargo test --workspace --no-fail-fast`: 490 passed, 0
failed, 1 ignored. Web: 259/259 across 50 files, typecheck, lint, build.

## Browser, on the person's own daemon

Port 4318 rebuilt from this branch, same database as PR 3's trial; project
"Book Catalogue"; Claude Code 2.1.292, adapter 0.81.1, Opus 5.5 Medium.

1. Asked for its tools, the Planner listed Shadows' 17 tools plus Read,
   Glob, Grep, ToolSearch, Agent, WebSearch, WebFetch, and said it cannot
   run commands or edit files.
2. Asked to add a CSV goal and approve it, it proposed the card, then
   noticed G3 already said the same and stopped to ask instead of
   approving. On "(a) edit G3 and approve it", it superseded its card,
   proposed the edit and called `vision_approve`. The card reads
   "Approved by the Planner at your request · your message"; the API row
   has `agent: planner:claude-code`, `principal: user:local` and the
   request entry at ordinal 7. The link opened the thread with
   `#entry-7` and scrolled to that message.
3. The PR 3 trial conversation's two map cards, approved by the button
   before this migration, read "Approved by you".

Not run in the browser: the waiver refusal and a button approval after 3b.
Both are pinned by `workspace_mcp/approval.rs`
(`the_planner_refuses_waiver_maps_but_the_person_approves`) and the
Task 2 storage tests.
