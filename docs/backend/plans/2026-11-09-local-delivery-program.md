# Local delivery implementation program

- Approved with review corrections by Mohammed on 2026-11-09; reordered into
  slices the same day, after his review of the first plans.
- Planning branch: `codex/local-delivery-plans`, based on `e63002ad`.
- Slice 1 is built (with B-120 to B-122 from its first real runs); slice 1b is detailed
  (2026-11-11).
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
| 1. Real files and the task contract | B-96, B-99, B-103, B-104, B-106 | The Planner and the person see the files the project really holds. A task names file work and function behavior, and the executor reads that through `task_context`. | F-99 (files view), F-101 | discovery (`2026-11-09-repository-discovery.md` at `db760593`), [contracts](2026-11-09-detailed-task-contracts.md) |
| 1b. The Planner runs and judges tasks | B-123 to B-127 | The Planner makes the folder a git repository, starts a task, hears how its attempt ended, judges the commit, and marks the task done or sends an addition (three at most) the executor carries on with in its own session; a task stopped on the cap leaves a lessons record. | per `docs/shared/requests/2026-11-11-planner-review.md` at `617d806f` (Masada numbers it) | `docs/backend/plans/2026-11-11-slice-1b-planner-runs-tasks.md` at `617d806f` (detailed; built) |
| 3. Setup first, through the Planner | B-128, B-100, B-101 (B-102 dropped) | Before planning in detail, the Planner sets the project up (git with the folder's files in its first commit, containers, the framework's scaffold, binaries allowed only in the plan titled Setup) as a setup plan whose tasks land as recorded commits, then plans on the files that appeared. | none (F-100's preparation view is not needed) | [slice 3](2026-11-11-slice-3-setup-first.md) (detailed) |
| 2. Real language servers for the project's languages | B-138 to B-146 (`docs/backend/stages.md` at `db760593`; B-97, B-98, B-119 replaced, 2026-11-13) | The project's language servers run while a conversation in it runs; the Planner and the executor ask them through `code_lsp`; a missing server is installed after the person chooses it on a question card; Settings › Code. The name index is suspended. | F-115, F-116 (`docs/frontend/stages.md` at `db760593`) | [design](../specs/2026-11-13-language-servers-design.md) |
| 4. The attempt brief and task questions | B-107 to B-109 | The executor receives one brief per attempt and asks about code beside its task's obligations. | F-102 | [briefs](2026-11-09-execution-briefs.md) |
| 5. Checks | B-111, B-113 (re-scoped), B-114 | A task's required checks run and are recorded, so the Planner judges on evidence. | F-103 | [checks](2026-11-09-checks-and-critic.md) |
| 6. Stopping obsolete plans and full acceptance | B-117, B-118 | A new Draft stops its plan's attempts, and the whole journey survives interruption. | F-104, F-105 | [repair](2026-11-09-repair-and-acceptance.md) |

The [frontend plan](../../frontend/plans/2026-11-09-local-delivery-ui.md)
holds F-99 to F-105. A web task waits for its backend API.

Slice 1 changes two tasks from the draft:

- **B-99** carries only the inventory to HTTP and to the Planner
  (`project_files`, `project_refresh`). The language-server status and question
  routes become **B-119** in slice 2.
- **B-106** exposes the contract without decisions, and updates the executor's
  instructions so the contract is part of its brief.

**The order is 1, 1b, 3, 2, 4, 5, 6** (Mohammed, 2026-11-11). Slices keep their numbers so
earlier references stay true. Slice 3 moves up because setup belongs to the Planner before
detailed planning ([§5.2](../specs/2026-11-08-executors-design.md#52-setup-before-detailed-planning-mohammed-2026-11-11)).
It no longer builds the draft's preparation worker, grant kind, thread kind, table and routes:
the setup is a plan whose tasks are ordinary attempts, so each lands as a recorded commit.
B-102 is dropped, and the web's preparation view (F-100) is not needed.

**Slice 1b comes before slice 2** (Mohammed, 2026-11-11, after trying slice 1 on a real Go
project; [executors design §5.1](../specs/2026-11-08-executors-design.md#51-the-planner-judges-mohammed-2026-11-11)).
The Planner is the reviewer, and an addition replaces a retry. What it does to the later
slices:

- **Absorbed:** B-115 (retry in the same live session with a fresh attempt identity) is B-126:
  an addition is a new attempt on the same executor thread.
- **Replaced:** B-116 (find the actual Planner, the three-attempt ladder) by B-125: the
  Planner is the plan version's `written_by` thread, the notice is a waiting message through
  `Turns::queue`, and the ladder becomes a cap of three additions per task start (B-126).
  B-112 (the critic session) by the Planner's judgment, and B-110 (source at the reported SHA),
  whose only reader was the critic, with it; both come back only if §5.1's critic OPEN closes
  that way.
- **Re-scoped:** B-113 keeps evidence-backed outputs; its "honest task acceptance" is the
  Planner's done mark from B-124 and B-125. B-114 exposes checks without a critic. Both are
  re-read when slice 5 is detailed.
- **Left as they are:** B-111 (required checks), B-117 (a new Draft stops its plan's
  attempts), B-118 (the full journey).

**Slice 2 changes** (Mohammed, 2026-11-11), applied to B-97 when the slice is detailed:

- **The server catalog is the twelve `lspServers` definitions** of
  [claude-plugins-official](https://github.com/anthropics/claude-plugins-official/tree/main/plugins)
  (`*-lsp/.claude-plugin/plugin.json`: `command`, `args`, `extensionToLanguage`; Apache-2.0):
  clangd, csharp, gopls, jdtls, kotlin, lua, php, pyright, ruby, rust-analyzer, swift and
  typescript, copied as one data file with its licence notice.
- **A project's servers are chosen from its files' extensions** in the B-96 inventory, as
  Serena does; no person picks them.
- **A server that is not installed** shows "not installed" with its install command; nothing
  installs it.
- The slice is no longer PHP-only: Intelephense is the PHP entry of the catalog, not the
  slice's subject.

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

- **Slice 3:** settled 2026-11-11: preparation is an ordinary setup plan run
  through the Planner's tools, in place of a new worker, grant kind, thread kind
  and table.
- **Slice 4:** `task_question` refuses a file outside the task's reads while
  the executor's own Read and Bash are not limited. The boundary is the commit
  ([executors design](../specs/2026-11-08-executors-design.md), section 7), so
  the refusal is examined before it is built. `where_is`, `who_uses` and
  `outline` were suspended at `db760593`; the executor has `code_lsp` instead.
- **Slices 5 and 6:** the critic and the attempt ladder were replaced by the
  Planner's judgment on 2026-11-11; the [executors design
  §5.1](../specs/2026-11-08-executors-design.md#51-the-planner-judges-mohammed-2026-11-11)
  keeps the decision, and the draft plan files' critic and ladder tasks are
  history.
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
- The Planner judges a commit by reading what it changed (the critic is replaced, §5.1).
- Plan state and the Runs watcher stop obsolete attempts; no hold/ack protocol.
- 12 server descriptors as data, from the plugins' `lspServers`; a server is ready only after
  a real trial.
