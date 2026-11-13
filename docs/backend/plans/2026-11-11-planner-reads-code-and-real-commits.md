# The Planner reads the code; an executor's commit changes something — Implementation Plan

> **For agentic workers:** each task is built by `backend-builder` from its entry in
> [stages.md](../stages.md); the entry's Touch and Done when are binding, and so are this
> file's Interfaces, Rules and steps. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The Planner reads the project's files before it names one in a task, on Claude Code
and on Codex; and an executor's report is accepted only for a commit that changes something.

**Rulings (Mohammed, 2026-11-11):**
1. The Planner must read the code before it plans, on every harness; otherwise a plan is only
   headlines. When it writes a task it knows which files the task touches, which functions
   are there and what each does, what the executor will write, and where each layer's part
   goes in the project's composition (its AppCore or equivalent) and layers.
2. Shadows refuses an empty commit from an executor. An executor needs no empty commit to
   return a report, nor to answer the Planner. `project_git_init`'s first commit is the
   Planner's tool, not a report, and is untouched.

**Evidence:** in the `testing3` trial the Codex Planner ran no shell command and read no file;
it listed names with `project_files` and wrote the plan from them. The cause is the Planner's
instructions, not the sandbox: `prompt.txt` says "you cannot run commands or edit code",
while the Codex session opens `sandbox: read-only`, `approvalPolicy: never`
(`crates/shadows-agent/src/codex/session.rs`, with `windows.sandbox="mxc"` from `codex.rs`),
where harness-controls §12.14 measured that a command starts and a write is refused. A Claude
Planner already has `Read`, `Grep` and `Glob` (`PLANNER_TOOLS`). T1 and T2 of the same trial
each committed empty commits (`--allow-empty`) only to carry a report.

**Architecture:** no new owner, tool, table or route.
- **B-132** changes the Planner's instructions (Harness) so it reads on both harnesses and
  carries the obligation to read before naming; a live Codex test proves a read.
- **B-133** adds one refusal to `Runs::report` (`EmptyCommit`), tells the executor not to
  commit when nothing changes, and gives the Planner the executor's last message when an
  attempt ends without a commit.

**Why the reading stays an instruction (decided here):** Shadows sees a Planner's reads only
as tool-call entries: a Claude `Read` with a path, a Codex shell command line (PowerShell,
cmd or sh). Checking a task's paths against them means parsing shell commands, a guess, not a
check; a read from an earlier turn or before a compaction would be missed, and a task that
creates new files has nothing to read. A refusal built on that guess would block honest
edits. So the rule is the instruction, a live proof that a Codex Planner reads, and the
lessons record (B-127), which shows when a task failed for want of reading. Cheap checks that
do not depend on reads (a `modify` file that must exist in the inventory) are a separate
decision, listed below.

**Why an attempt with no change ends without a commit, not a new state (decided here):** the
commit stays the one checkpoint (Mohammed, 2026-11-08: one checkpoint, no edge-case
machinery). The refusal reaches the executor during its turn, so it learns at once and can
make a real commit; if it has nothing to commit it ends its turn, the attempt becomes
`Failed` "ended without a commit" as today, and the Planner hears it with the executor's last
message, so an answer is not lost. Accepting a report with no commit would need a state that
waits for judgment without a commit of its own, which `task_diff`, `task_file`, the done mark
and the cap would each have to learn. With `task_diff` the Planner no longer needs to ask the
executor anything a commit does not show.

**Tech Stack:** Rust 1.94+; `prompt.txt`, `executor.md`; git through `shadows-process`.

**Spec:** [harness controls §12.14](../../core/specs/2026-09-24-harness-controls-design.md#1214-codex-over-app-server)
(the Planner reads through read-only commands); [executors design §5.1](../specs/2026-11-08-executors-design.md#51-the-planner-judges-mohammed-2026-11-11)
(reading before planning; no empty commit). Both amended with this plan.

## Global Constraints

- The same as slice 1b (`docs/backend/plans/2026-11-11-slice-1b-planner-runs-tasks.md` at `617d806f`, its Global Constraints): real
  tests (live or no agent), never a new fake-acp test; only the tests named, with `-p`; no
  `crates/**/*.rs` line over 100 columns; commit by path.
- The Codex sandbox, approval policy and tool gate do not change. If the live test shows a
  read refused under `mxc`, stop and report the measured refusal; widen nothing.
- `project_git_init` (`Runs::init_git`) keeps its empty first commit.
- Order: B-132, B-133, then slice 3 (B-128 onward).

## Review Focus

1. **A Codex Planner told it may run commands runs one that writes** (a build, `npm install`):
   the sandbox refuses it; the prompt forbids it. The live test asks for a write and checks
   no file appeared.
2. **A secret file read by a shell command** (`.env`): the prompt's existing rule ("A secret
   file is listed by name only. Do not read it") stands; the read-only sandbox does not stop
   it. Listed for Mohammed below.
3. **An executor asked a question in an addition** commits nothing and ends: the attempt fails
   and the Planner reads its last message. Held by the extended live failure test.
4. **An empty commit refused, then a real one** in the same attempt is accepted. Held by
   `an_empty_commit_is_refused_until_it_changes_something`.
5. **A merge or root commit with no change** is empty by `Git::changes` too. The same test's
   root case is not built: Start always has a HEAD before an attempt's commit.

---

## Task B-132: The Planner reads the code before it plans, on every harness

**Files:**
- Modify: `crates/core-harness/src/prompt.txt`
- Modify: `crates/core-harness/contract.yaml`
- Test: `crates/shadows/tests/mcp_tools.rs`
- Test: `crates/shadows-core/tests/codex_session.rs` (one live test and the start it needs)

**Rules:**
- `prompt.txt`, "Your job": the line "- Read the code, search, and think before you plan. Your
  available tools are fixed by Shadows; you cannot run commands or edit code." becomes,
  exactly: "- Read the code, search, and think before you plan. You read and search the
  project's files and never change them: on Claude Code with Read, Grep and Glob; on Codex
  with read-only shell commands in the project's folder, such as git ls-files, git grep, git
  show HEAD:<path>, rg, and Get-Content or cat. Never run a command that writes, installs,
  builds, starts a server or reaches the network; Shadows' tools are your only way to write."
- `prompt.txt`, "Look before you write a path": its first line ("- Before a task names a path,
  list the real files with project_files and read the ones that decide the layout: ...")
  becomes, exactly: "- Before a task names a file, a function, a class or a layer, read the
  files that decide it: the manifest first (composer.json, package.json, go.mod, Cargo.toml);
  the composition root, where the application is wired (its AppCore, main, bootstrap or
  service container); the folders of the layers the task works in; and every file the task
  modifies, whole. List them with project_files, then read them." A new line follows: "- A
  task names what the executor will write and where each layer's part of it goes, as the
  project already does it; name a function only after you read where it is or will be and
  what is beside it. A plan written from file names alone is not ready." The other lines of
  that section stay.

- [ ] **Step 1: Pin the texts.** New test in `mcp_tools.rs`:

```rust
#[test]
fn the_planner_reads_the_code_before_it_plans() {
    let prompt = shadows_core::testing::PROMPT;
    assert!(!prompt.contains("you cannot run commands"));
    for text in [
        "on Codex with read-only shell commands in the project's folder",
        "Never run a command that writes, installs, builds, starts a server or reaches the network",
        "the composition root, where the application is wired",
        "and every file the task modifies, whole",
        "A plan written from file names alone is not ready.",
    ] {
        assert!(prompt.contains(text), "{text}");
    }
}
```

  Run `cargo test -p shadows --test mcp_tools the_planner_reads_the_code`: it fails.
- [ ] **Step 2: Change `prompt.txt`** by the Rules; the test passes, and so does every other
  prompt pin in `mcp_tools.rs` (`the_planner_is_told_how_to_run_and_judge_tasks`,
  `the_planner_is_told_what_a_contract_holds`, the indexing one).
- [ ] **Step 3: The live proof** in `codex_session.rs`, beside the fixture's `start_in`, with a
  start that takes the real program:

```rust
/// A Codex Planner reads a file of the project with a read-only shell command,
/// and a command that would write leaves nothing behind.
#[tokio::test]
#[ignore = "live: needs codex (SHADOWS_CODEX_PATH) signed in to a dedicated home \
            (SHADOWS_CODEX_HOME)"]
async fn live_a_codex_planner_reads_a_file_and_writes_none() {
    // The project folder holds notes/word.txt with a fresh random word (a uuid's first
    // 8 hex digits), named in no prompt, tool answer or file name.
    // Turn 1: "What word is written in notes/word.txt? Read the file, then answer with
    //   the word only." The turn completes; its reply holds the word.
    // Turn 2: "Create the file made.txt in the project folder containing x." The turn
    //   completes or fails; made.txt does not exist afterwards.
    // println!("live codex read: {word} {} tool entries", the thread's ToolCall count);
}
```

  The start reads both variables and fails with their names when one is missing; otherwise
  it is `start_in`'s, with `codex_path` and `codex_home` from them and the project folder
  holding the file. Model: the catalog's default.
- [ ] **Step 4: Contract.** `core-harness/contract.yaml`: the Planner's instructions name the
  read rule on both harnesses and the obligation, with `the_planner_reads_the_code_before_it_plans`
  and `live_a_codex_planner_reads_a_file_and_writes_none`.
- [ ] **Step 5:** run what the entry names; commit `feat(backend): B-132 <goal>`.

---

## Task B-133: An executor's commit must change something

**Files:**
- Modify: `crates/core-runs/src/report.rs` (the empty check in `verify`)
- Modify: `crates/core-runs/src/error.rs` (`EmptyCommit`)
- Modify: `crates/core-runs/src/notify.rs` (the failed notice's last message)
- Modify: `crates/core-runs/contract.yaml`
- Modify: `crates/shadows-mcp/src/run_tools.rs` (`task_report`'s description)
- Modify: `crates/shadows-mcp/src/refusal.rs` (only if it lists Runs refusals by variant)
- Modify: `crates/shadows-mcp/src/contract.yaml`
- Modify: `crates/core-harness/src/executor.md`
- Modify: `crates/core-harness/src/executor.rs` (its test module only)
- Modify: `crates/core-harness/src/prompt.txt` ("Running tasks", one line)
- Modify: `crates/core-harness/contract.yaml`
- Test: `crates/shadows-core/tests/runs_attempts/report.rs`
- Test: `crates/shadows/tests/mcp_tools.rs`
- Test: `crates/shadows/tests/planner_runs_live.rs`

**Interfaces:**
- Produces: `RunsError::EmptyCommit { commit: String }`, code `empty_commit`, text exactly:
  "the commit {commit} changes no file: commit only work; if nothing needs to change, do not
  commit, and say why in your last message". HTTP serves it as every refusal (409, `RunRefusal`);
  no route reports a commit, so `api/` does not change.
- Consumes: `Git::changes` (already run by `verify`); `Threads::entries` (Runs holds `Threads`
  since B-127).

**Rules:**
- In `verify`, after `NotNew` and before `OutsideWrites`: `changes` empty is `EmptyCommit`
  with the full sha. A refusal leaves the attempt `Running`, as every report refusal does.
- `notify`, a failed attempt: after the reason, when the executor's thread holds an agent
  message of the attempt's operation, the notice adds "The executor's last message:\n{text}"
  with the last one, cut at 4,000 chars on a char boundary with "[cut]" appended when cut.
  Its absence or a failure to read entries adds nothing and is logged
  `runs.notify_no_message`. The committed notice does not change.
- `executor.md`, the commit bullet gains: "If nothing needs to change, do not commit: end
  with a last message saying why. Shadows refuses an empty commit." The text the B-129 report
  bullet holds stays.
- `prompt.txt`, "Running tasks", after the `task_amend` line: "- An addition asks for a change
  to the task's files. Never send one to ask a question or for evidence the commit shows:
  read that with task_diff and task_file. An executor that has nothing to change commits
  nothing, and its attempt ends without a commit."
- `task_report`'s description: "... changes only paths inside your task's writes, changes at
  least one file and adds no binary file; ..." (the rest unchanged).

- [ ] **Step 1: Failing test** in `report.rs`:

```rust
#[tokio::test]
async fn an_empty_commit_is_refused_until_it_changes_something() {
    // begin(3); git commit --allow-empty -m "Nothing"; report it:
    //   Err(RunsError::EmptyCommit { commit }) with commit == the full sha;
    //   f.still_running(&attempt).
    // Then put backend/app/a.php, commit it, report: Ok, state Committed, its sha.
}
```

  Run `cargo test -p shadows-core --test runs_attempts an_empty_commit`: it fails to compile.
- [ ] **Step 2:** add `EmptyCommit` and the check; the test passes, and every earlier report
  test passes unchanged.
- [ ] **Step 3: The texts** by the Rules; pins: `the_planner_is_told_how_to_run_and_judge_tasks`
  adds "An addition asks for a change to the task's files"; the executor test module's
  `the_executor_reports_commands_and_leaves_its_writes_clean` adds "Shadows refuses an empty
  commit"; `the_planner_is_told_how_to_run_and_judge_tasks` or a test beside it checks
  `task_report`'s description holds "changes at least one file" (through an executor
  client's tool list, as `an_executor_grant_lists_its_tools_and_reads_only_its_task` reads it).
- [ ] **Step 4: The notice.** `live_an_attempt_without_a_commit_reaches_its_planner` in
  `planner_runs_live.rs` also asserts the notice holds "The executor's last message:" and
  prints its first line after it.
- [ ] **Step 5: Contract.** `core-runs/contract.yaml`: `EmptyCommit` in `RunsError` with its
  code; `report`'s order names it; `notify`'s failed notice names the last message; an
  obligation "A reported commit that changes no file is refused; the attempt stays Running,
  and a real commit is accepted" tested by the new test. `shadows-mcp` and `core-harness`
  contracts name the texts.
- [ ] **Step 6:** run what the entry names; commit `feat(backend): B-133 <goal>`.

---

## Left for Mohammed to decide

- **A cheap check that needs no reads:** Plans could refuse a task whose contract file has
  action `modify` or `delete` and is not in the project's inventory (Code, B-96). It catches a
  task written from a guessed layout, not one written without reading. It is a Plans rule
  reading Code; not built without a ruling.
- **What a Planner may read outside the project.** Claude's `Read` and Codex's read-only
  sandbox can both read any file the person's account can, outside the project too (`~/.ssh`,
  another project's `.env`). The instructions keep the Planner in the project's folder and away
  from secret files; nothing enforces it.
- **The executor's leftover in `writes` after its commit** (the line-ending change): still only
  told (B-129). A `report` refusal when `writes` is dirty after the commit stays listed.
