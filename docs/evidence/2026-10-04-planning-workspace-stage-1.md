# Planning workspace Stage 1 — Windows integration evidence

**Date:** 2026-10-04, Asia/Aden (+03:00). Logs use UTC; the browser run
started at 2026-10-03T22:12:59Z, which is 2026-10-04 locally.
**Branch:** `codex/shared-contracts-planning`.
**Basis:** reviewed Task 3 `bceda16e01759c32e9414edd2df869e42f845a39` plus
the Task 4 changes committed with this record.
**Scope:** workspace Stage 1 only, under §18.2–18.3 and its
[implementation plan](../superpowers/plans/2026-10-03-planning-workspace-stage-1.md).
This record is evidence, not design authority or a whole-branch review.

## Retry correction

`PartEditor` cleared its command attempt after a committed POST but before
its authoritative GET succeeded. A failing GET left the old revision; another
Save sent a new command ID and conflicted with the already committed write.
The new request-aware app regression failed specifically on unequal retry
command IDs (RED), then passed after retaining the attempt through the read
(GREEN). The complete retry body and Arabic text are retained. The successful
creation path still clears its completed attempt. No Core/API rule changed.

## Migration evidence

- A disposable database migrated only through 0012 was populated with an
  archived plan, Frozen version, task JSON with significant whitespace and a
  durable event. `workspace_migrations_preserve_old_frozen_bytes_and_reopened_links`
  opened it through the real SQLx `Storage::open`, applying 0013–0015.
- The recorded plan/workflow/task/event fields, including raw JSON strings,
  compared equal before and after. `PRAGMA foreign_key_check` returned no rows;
  direct mutation of the old Frozen version remained refused.
- Workspace edits then created ordered parts, a child and outcome references
  to two parts and the old plan. Reopen preserved ordering, ancestry, element
  revisions, links and the original command's replay result.
- The running development daemon's configured database was
  `C:/Users/Mohammed/AppData/Local/shadows-dev/shadows.sqlite3`. A read-only
  connection used Python's SQLite `Connection.backup`, including committed WAL
  data, to prepare a separate copy. The original daemon was not stopped and
  neither its database nor the repository debug executable was replaced.
- That source copy was at migration **12**, with **23 old tables, zero
  projects/plans/workflows and two events**. It is not populated user-plan
  migration evidence. The guarded manual test `migrate_prepared_dev_copy_only`
  ran explicitly and passed; it is ignored by the ordinary suite because it
  requires this prepared backup. The copy reached **15**; every old table's
  row count and sorted complete-row hash matched, with zero foreign-key faults.
- An initial manual-test failure concerned Windows' `\\?\` canonical path,
  not migration content. The test retains its canonical containment check and
  passes a normal path to Storage using the existing `dunce` dependency.

## Integrated HTTP and browser checks

The actual router tests demonstrate bounded **51-outcome** pagination, atomic
refusal of a mixed edit, and project SSE replay, immutable command deduplication,
live mixed changes, resume from the recorded cursor and foreign-project
isolation. Existing plan-stream tests still pass.

The Web hook test uses two independent QueryClients/connections: all three
workspace slices refresh, duplicate sequence numbers do not refetch, reconnect
uses the last sequence, catch-up refreshes authoritative data, and project
switch/unmount closes the appropriate connection. App tests exercise actual
Part/Outcome editors through disconnect, replay, stale Save and explicit Reload;
Arabic unsaved input survives. Existing Vision and plan-invalidation tests ran.

For the real browser trial, the agent drove two Codex in-app browser tabs:

- An ordinary daemon built in the existing C: scratch Cargo target listened
  on **127.0.0.1:4319** with `--debug`, a disposable `browser.sqlite3` and
  `--allow-origin http://127.0.0.1:5174`. Vite used `VITE_SHADOWS_URL` pointing
  to 4319 and `--host 127.0.0.1 --port 5174 --strictPort`.
- The browser DB was a second copy, seeded with synthetic legacy Frozen-plan
  fixtures. No ACP planning turn or account credentials were used. One old
  plan was opened while unassigned, then linked; another remained archived.
- Saved Vision, two root parts and a child; associated an archived plan.
  Renamed/reparented the child and reordered the roots through the UI.
- Created a Roadmap outcome referencing both parts; linked both existing
  plans. Inspected acceptance text and references after authoritative reload.
- In each Vision/Part/Outcome editor, another tab's committed change arrived
  through SSE. A dirty tab kept its input and stale Save returned
  `REVISION_CONFLICT`; explicit Reload read the saved values.
- Opened the old Frozen URL, observed archived read-only state without an
  Approve button, unarchived/rearchived it and verified list updates. The
  workspace Include archived filter showed/hid the archived plan correctly.
- Stopped/restarted the isolated daemon. The other tab's dirty outcome text
  survived the actual outage and reconnected; refreshed deep links retained
  identity, parent, root order and stored design text. A controlled outcome
  page reload retained its `view=roadmap&outcome=…` URL.
- Read-only post-trial SQLite assertions confirmed the archived Frozen row
  exactly matches its deterministic seed, root order and renamed child/parent
  match, two part/two plan references remain, workspace revision is **9**,
  and foreign-key faults are **0**. No outcome completion state is inferred.

The first two daemon log files were empty under inherited `RUST_LOG=warn`.
The final isolated restart removed it: the recorded log is **85,826 bytes**,
shows recovery `interrupted=0 anomalies=0`, both project subscriptions resuming
at `after=13`, catch-up and HTTP reads/edits. It contains **0 WARN/ERROR lines**.
The final reloaded browser tab's console error/warning snapshot was empty;
expected network failures during the deliberate outage are not claimed absent.
Both owned trial processes and tabs were closed; the original daemon remained.

## Commit verification

Recorded gate commands: `cargo fmt --all --check`; all-targets Clippy with
`fake-acp/test-support`; `cargo test --workspace`; ordinary Clippy; production
feature tree; API diff; Web typecheck, lint, test, build; `git diff --check`.

The Rust suite passed **385 tests**, with **1 manual backup test ignored**;
that test separately ran and passed against the guarded copy. Web passed
**179 tests in 36 files**. Production features contain no `test-support` and
there is no generated API change. The Web production build and final diff
check passed. Vite retains its existing large-chunk advisory (main chunk
1,031.38 kB); no performance improvement is claimed.

Two failed gate attempts are retained: Cargo could not overwrite the running
scratch daemon on Windows; stopping the owned trial allowed Rust tests to run.
Web typecheck found the new fake saved value inferred `kind` too narrowly;
typing it as the generated `PartView` corrected the fixture. Successful
unchanged Rust checks were preserved while Web checks resumed after this
test-only correction. Neither failure was described as a product regression.

Raw logs, DB manifests, scratch scripts and screenshots are preserved in
`.superpowers/sdd/2026-10-03-planning-workspace-stage-1/` for the handoff.
Representative screenshots: `task-4-runtime/map-after-restart.jpg` and
`task-4-runtime/roadmap-after-restart.jpg`.

## Limits and handoff

- This is an **agent-operated Windows browser trial**, not Mohammed's personal
  acceptance. Full Stage 1 independent whole-branch review is still pending;
  no reviewer agent was dispatched, respecting the user's latest steering.
- No Linux run, performance benchmark, very-deep hierarchy stress or concurrent
  SQLite snapshot interleaving trial is claimed.
- Stage 2 agreements/bindings, automatic Planner context, executor/manager
  and specialized diagrams remain outside this slice. No push/PR/merge ran.
