# Local delivery implementation program

- Approved with review corrections by Mohammed on 2026-11-09; reordered into
  slices the same day, after his review of the first plans.
- Planning branch: `codex/local-delivery-plans`, based on `e63002ad`.
- Implementation has not started.
- Owner: [approved 17.0](../../core/specs/2026-10-01-task-contracts-and-evidence-design.md#170-approved-local-delivery-revision-2026-11-09).
- [API/model agreement](../../shared/requests/2026-11-09-local-delivery.md).

## How this program is worked

The work moves one slice at a time. A slice is built, tried on a real Laravel
project, and only then is the next slice detailed. A later slice may change
because of what the earlier one showed.

1. **Hamoud details the slice.** He reads the code from `AppCore`, checks the
   draft plan against it, and writes each task of the slice in
   [stages.md](../stages.md) in the shape `docs/backend/AGENTS.md` gives. A
   task names its steps in order, every file it creates or changes, every type
   with its fields, and every function with its inputs, its return value, its
   errors and its behavior. The builder writes the bodies and decides nothing
   else.
2. **The controller starts `backend-builder`** with one line per task. Tasks
   that depend on each other go to the same builder, one after another, with no
   stop between them.
3. **Hamoud reviews the built tasks** in the same session that planned them, and
   reports.
4. **The controller reads the report** and amends this program when the result
   calls for it.

The six plan files below are draft material for step 1, not builder
instructions. Where a plan file and a detailed task in `stages.md` disagree,
the task in `stages.md` is right and the plan file is corrected when its slice
is detailed.

## Slices

| Slice | Backend tasks | Runnable outcome | Web tasks | Draft material |
|---|---|---|---|---|
| 1. Real files and the task contract | B-96, B-99, B-103, B-104, B-106 | The Planner and the person see the files the project really holds. A task names file work and function behavior, and the executor reads that through `task_context`. | F-99 (files view), F-101 | [discovery](2026-11-09-repository-discovery.md), [contracts](2026-11-09-detailed-task-contracts.md) |
| 2. A real PHP language server | B-97, B-98, B-119 | `Code` answers definition and reference questions from Intelephense, beside the existing name index. | F-99 (server status) | [discovery](2026-11-09-repository-discovery.md) |
| 3. Framework preparation from planning | B-100 to B-102 | The Planner prepares an agreed empty destination and plans on the files that appear. | F-100 | [preparation](2026-11-09-project-preparation.md) |
| 4. The attempt brief and task questions | B-107 to B-109 | The executor receives one brief per attempt and asks about code beside its task's obligations. | F-102 | [briefs](2026-11-09-execution-briefs.md) |
| 5. Checks and the critic | B-110 to B-114 | A task is accepted on recorded checks and a critic that read the exact commit. | F-103 | [checks](2026-11-09-checks-and-critic.md) |
| 6. Bounded repair and full acceptance | B-115 to B-118 | Failures reach the Planner, retries stay bounded, and the whole journey survives interruption. | F-104, F-105 | [repair](2026-11-09-repair-and-acceptance.md) |

The [frontend plan](../../frontend/plans/2026-11-09-local-delivery-ui.md)
holds F-99 to F-105. A web task waits for its backend API.

Slice 1 changes two tasks from the draft:

- **B-99** carries only the inventory to HTTP and to the Planner
  (`project_files`, `project_refresh`). The language-server status and question
  routes become **B-119** in slice 2.
- **B-106** exposes the contract without decisions, and updates the executor's
  instructions so the contract is part of its brief.

## Rulings on the draft

These hold for every slice. Each is applied to a plan file when its slice is
detailed.

- **Tests are real.** A new test uses a real temporary folder, a real Git
  repository, a real language server and the live adapter (MiniMax, as in
  `crates/shadows-core/tests/aylul_extensions/live.rs`, ignored unless its key
  is present). No new fake worker, fake server or `fake-acp` test is written.
  Existing `fake-acp` tests stay for CI.
- **Language-server code is a library.** The client that starts a server and
  asks it lives in a new crate, `shadows-lsp`, beside `shadows-index`. It knows
  no project, grant, task or table. `Code` owns each project's server
  configuration and the choice between the name index and the server. One
  client and one catalog serve every server; a server gets its own file only
  when it needs code of its own.
- **Rust symbols are found with the LSP,** as `CLAUDE.md` says. The draft's
  "use rg, not Serena" line was a remark to one session and is not a rule.
- **The full gate runs in CI** after the branch is pushed. A task runs its own
  tests locally.

## Corrections to examine when their slice is detailed

- **Slice 3:** whether preparation can be an ordinary executor attempt with the
  destination as its write scope, in place of a new worker, grant kind, thread
  kind and table.
- **Slice 4:** `task_question` refuses a file outside the task's reads while
  the executor's own Read and Bash are not limited. The boundary is the commit
  ([executors design](../specs/2026-11-08-executors-design.md), section 7), so
  the refusal is examined before it is built. `where_is`, `who_uses` and
  `outline` stay with the executor until then.
- **Slices 5 and 6:** the critic and the attempt ladder are also described in
  the [executors design](../specs/2026-11-08-executors-design.md), phases 2
  and 3. One of the two documents keeps the decision and the other links to it.
- **Deferred until a task needs them:** project decision revisions (B-105),
  accepting one output while another item is pending, pending rechecks, and the
  before/after baseline.

## Source and migration reconciliation

Every path the draft plans name was checked against the checkout on 2026-11-09:
each `Modify` entry exists or is created by an earlier task. The last migration
is `0053_attempt_report`; the draft reserves 0054 to 0061. A slice numbers its
own migrations from the last one present when it is detailed. No existing
migration or Frozen task is rewritten.

## What stays from the accepted corrections

- Precise file and symbol work stays central. A function task states its name,
  inputs, returns, behavior and errors; the executor writes its implementation.
- Permission globs coexist with exact named work.
- One producer contract with explicit consumers; no schema fingerprints.
- One stored brief per attempt; no hashed packet compiler or source snapshots.
- The existing write-overlap check and one project check slot; no lease manager.
- The critic reads the exact Git SHA and path.
- Plan state and the Runs watcher stop obsolete attempts; no hold/ack protocol.
- 13 server descriptors as data; a server is ready only after a real trial.
