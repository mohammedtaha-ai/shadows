# The Planner reads what an attempt committed — Implementation Plan

> **For agentic workers:** each task is built by `backend-builder` from its entry in
> [stages.md](../stages.md); the entry's Touch and Done when are binding, and so are this
> file's Interfaces and steps. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The Planner judges an attempt from what its commit changed, read through its own
tools, so additions go to real fixes, not to asking the executor for its files.

**Why (evidence, 2026-11-11):** the live trial of slice 1b on `testing3` (Planner on Codex,
executor on Claude `sonnet`), plan "Go health endpoint in two tasks" (workflow
`4710b69a-8794-4340-bf12-e23bf84e067f`, v1). T2 took four attempts and all three additions.
Addition 1 asked for the full contents of the six committed files; the executor showed them in
its tool output; addition 2 said "Your tool output is invisible to the planner" and asked for
them pasted in the report; only addition 3 fixed a real defect (`HEAD /health` answered 200,
not 405). T1 also spent an addition on an empty commit whose report held its files. The
`[Shadows]` notice names the commit and each file with its counts, but no content, and no
Planner tool reads a file at a commit (`project_files` lists names only; a Codex Planner has no
`Read`).

**Architecture:** Runs already reads an attempt's commit (`Runs::diff`, behind
`GET /api/attempts/{id}/diff`). It gains one method, `Runs::file`, a file of the task as an
attempt's commit holds it, which answers both the Planner and the web's open request
(`docs/shared/requests/2026-11-11-attempt-file-content.md` at `617d806f`, F-109). The Planner reaches both
methods through two tools, `task_diff` and `task_file`, by `diff_for` and `file_for`: the
Planner's grant checked as `start_for` checks it, the task's latest committed attempt found,
then the same `diff` and `file` the routes call. The notice stays as it is but names the two
tools; the patches do not go into the notice, which the person reads in the conversation. The
executor's report no longer has to carry file contents; it carries the commands it ran and
their results, which the commit cannot show.

**Tech Stack:** Rust 1.94+, git through `shadows-process`, rmcp tools in `shadows-mcp`.

**Spec:** [executors design §5.1](../specs/2026-11-08-executors-design.md#51-the-planner-judges-mohammed-2026-11-11),
"The Planner judges" (amended with this plan).

## Global Constraints

- The same as slice 1b (`docs/backend/plans/2026-11-11-slice-1b-planner-runs-tasks.md` at `617d806f`, its Global Constraints): new
  tests are live (the real adapter, real git) or run no agent, never a new fake-acp test;
  symbols through the LSP; only the tests named, with `-p` and the target; one build cache in
  `target/`; no `crates/**/*.rs` line over 100 columns; commit by path.
- Runs reads git only through `Git` (`crates/core-runs/src/git.rs`), one child per call, never
  through a shell. Only a hex SHA or a path that passed the writes check reaches a git
  argument.
- One matcher: a path is inside a task's `writes` only by `Scope` (`scope.rs`). The report
  check and `file` call the same method.
- One bound: `MAX_LINES` (2,000) in `diff.rs` cuts a patch, a file and the Planner's whole diff.
- Order: B-129, then B-130, then slice 3 (B-128, B-100, B-101). B-131 waits for the request to
  be `accepted`.

## Review Focus

1. **A scaffold commit of hundreds of files** (slice 3's setup tasks): `task_diff` must not
   answer megabytes. `diff_for` keeps every file's counts and stops showing patches once 2,000
   patch lines are in the answer; the rest say to read them with `task_file`. Held by
   `the_planners_diff_shows_patches_up_to_its_bound`.
2. **A judgment after an addition**: the latest attempt's diff is only the addition's change.
   `task_file` reads any file inside the task's `writes` at that commit, changed by it or not.
   Held by `a_file_is_read_whole_at_the_attempts_commit` (the file the attempt did not change).
3. **A path that climbs out or leaves `writes`** (`backend/app/../../.env`, `notes.txt`): refused
   as `OutsideWrites` before git runs. Held by
   `a_file_outside_writes_or_not_at_the_commit_is_refused` and the scope unit test.
4. **The person's uncommitted edit to the same file** after the commit: `task_file` answers the
   commit's text, never the working tree's. Held by `a_file_is_read_whole_at_the_attempts_commit`.
5. **A long or binary file**: cut at 2,000 lines or 200,000 bytes with `cut` true; a binary one
   answers `text` null. Held by `a_long_or_binary_file_is_cut_or_has_no_text`.

---

## Task B-129: The Planner reads what an attempt committed

**Files:**
- Modify: `crates/core-runs/src/model.rs` (new `AttemptFile`)
- Modify: `crates/core-runs/src/lib.rs` (re-export `AttemptFile`)
- Modify: `crates/core-runs/src/scope.rs` (`admits_under`, its unit test)
- Modify: `crates/core-runs/src/report.rs` (`verify` calls `admits_under`; no rule changes)
- Modify: `crates/core-runs/src/git.rs` (`Git::blob`)
- Modify: `crates/core-runs/src/diff.rs` (`Runs::file`, `MAX_BYTES`)
- Modify: `crates/core-runs/src/planner.rs` (`diff_for`, `file_for`, `planner_commit`)
- Modify: `crates/core-runs/src/notify.rs` (the committed notice's last sentence)
- Modify: `crates/core-runs/contract.yaml`
- Modify: `crates/shadows-mcp/src/run_tools.rs` (tools `task_diff`, `task_file`)
- Modify: `crates/shadows-mcp/src/contract.yaml`
- Modify: `crates/core-grants/src/tools.rs` (`THREAD_TOOLS` 27 to 29)
- Modify: `crates/core-grants/contract.yaml`
- Modify: `crates/core-harness/src/prompt.txt` ("Running tasks")
- Modify: `crates/core-harness/src/executor.md` (the commit and report bullets)
- Modify: `crates/core-harness/src/executor.rs` (its test module only)
- Modify: `crates/core-harness/contract.yaml`
- Test: `crates/shadows-core/tests/runs_attempts/report.rs`
- Test: `crates/shadows/tests/mcp_server.rs` (the Planner's tool count, 27 to 29)
- Test: `crates/shadows/tests/mcp_tools.rs`
- Test: `crates/shadows/tests/planner_runs_live.rs`

**Interfaces:**
- Consumes: `Runs::diff(&AttemptId) -> Result<Vec<FileDiff>, RunsError>` (unchanged);
  `Runs::planner_plan` (`pub(crate)`, planner.rs); `Runs::attempts` (newest first);
  `RunsStore::writes(&AttemptId)`; `Git::prefix`, `Git::changes`.
- Produces:
  - `pub struct AttemptFile { pub path: String, pub text: Option<String>, pub cut: bool }` in
    `model.rs`, deriving `Debug, Clone, Serialize, utoipa::ToSchema` as `FileDiff` does.
  - `impl Scope { pub(crate) fn admits_under(&self, prefix: &str, path: &str) -> bool }`: true
    when `path` (a repository path, `/`-separated) starts with `prefix` and the rest is admitted
    by `admits`; a path with an empty, `.` or `..` component is admitted by nothing.
  - `impl Git { pub(crate) async fn blob(&self, commit: &str, path: &str) -> Result<Option<Vec<u8>>, String> }`:
    the bytes of `<commit>:<path>` when `git cat-file -t` names a `blob` there, else `None`.
  - `impl Runs { pub async fn file(&self, attempt: &AttemptId, path: &str) -> Result<AttemptFile, RunsError> }`
    in `diff.rs`.
  - `impl Runs { pub async fn diff_for(&self, grant: &Grant, workflow: &WorkflowId, task: u32) -> Result<Vec<FileDiff>, RunsError> }`
    and `pub async fn file_for(&self, grant: &Grant, workflow: &WorkflowId, task: u32, path: &str) -> Result<AttemptFile, RunsError>`
    in `planner.rs`, with the private
    `async fn planner_commit(&self, grant: &Grant, workflow: &WorkflowId, task: u32) -> Result<AttemptId, RunsError>`.
  - B-131 consumes `Runs::file` and `AttemptFile` unchanged.

**Rules the code must hold:**
- `file`: an unknown attempt, or one with no commit, is `NotFound`. The path is trimmed and its
  `\` become `/`. A path `admits_under` refuses (against `Git::prefix` and the attempt's
  `writes`) is `OutsideWrites { paths: [path] }`, before git reads anything. A blob at the
  commit answers its text, or `text: None` when its first 8,000 bytes hold a NUL or it is not
  UTF-8. No blob there: `text: None` when `Git::changes` of the commit lists the path (it was
  deleted), else `NotFound`. Text longer than `MAX_LINES` lines or `MAX_BYTES` (200,000) bytes
  is cut at whichever comes first, on a line and char boundary, with `cut: true`; no marker is
  added to the text.
- `planner_commit`: `planner_plan(grant, workflow)` (a Thread grant of the plan's project,
  else `NotFound`), then the newest attempt of `(workflow, plan.version, task)` that has a
  commit, whatever its state; none is `NotFound`.
- `diff_for`: `diff` of that attempt; walking its files in order, a patch is kept while the
  patch lines kept so far plus its own stay within `MAX_LINES`; every later patch becomes
  `[not shown: read it with task_file]`. Counts and paths always stay. The HTTP route's answer
  does not change.
- `file_for`: `file` of that attempt with `path`.
- `report`'s `verify` replaces its inline `strip_prefix(...).is_some_and(admits)` with
  `admits_under`; every report test passes unchanged.

- [ ] **Step 1: Write the failing tests** in `crates/shadows-core/tests/runs_attempts/report.rs`
  (the test plays the executor with `begin`, `put`, `commit`, `report`; no agent runs):

```rust
#[tokio::test]
async fn a_file_is_read_whole_at_the_attempts_commit() {
    // seed backend/app/a.php "one\n" and backend/app/c.php "c\n" in one commit; begin(3);
    // put a.php "one\ntwo\n" and b.php "b\n"; commit both; report.
    // Then put a.php "the person's edit\n" in the working tree, uncommitted.
    let a = f.core.runs().file(&attempt, "backend/app/a.php").await.unwrap();
    assert_eq!((a.text.as_deref(), a.cut), (Some("one\ntwo\n"), false));
    let b = f.core.runs().file(&attempt, "backend\\app\\b.php").await.unwrap();
    assert_eq!(b.text.as_deref(), Some("b\n"));
    let c = f.core.runs().file(&attempt, "backend/app/c.php").await.unwrap();
    assert_eq!(c.text.as_deref(), Some("c\n")); // in writes, not changed by the attempt
}

#[tokio::test]
async fn a_file_outside_writes_or_not_at_the_commit_is_refused() {
    // seed notes.txt and backend/app/c.php; begin(3); delete c.php, add a.php; commit; report.
    // "notes.txt" and "backend/app/../../notes.txt": Err(RunsError::OutsideWrites { paths })
    //   with paths == [the path given];
    // "backend/app/none.php": Err(RunsError::NotFound);
    // "backend/app/c.php" (deleted by the commit): Ok, text None, cut false;
    // begin(5), still Running with no commit, file(.., "web/x.js"): Err(RunsError::NotFound).
}

#[tokio::test]
async fn a_long_or_binary_file_is_cut_or_has_no_text() {
    // seed backend/app/logo.bin = [0u8, 1, 2] before begin(3); the attempt commits
    // backend/app/long.txt of 2,500 lines "line N\n"; report.
    // long.txt: cut true, text.lines().count() == 2_000, text ends with "line 2000\n";
    // logo.bin: text None, cut false.
}

#[tokio::test]
async fn the_planner_reads_the_same_change_the_diff_route_serves() {
    // begin(3); commit backend/app/a.php; report. grant = planner_grant(&f).await.
    // serde_json::to_value(diff_for(&grant, &f.workflow, 3)) == to_value(diff(&attempt));
    // to_value(file_for(&grant, &f.workflow, 3, "backend/app/a.php"))
    //   == to_value(file(&attempt, "backend/app/a.php"));
    // with super::planner::executor_grant(&f): both Err(RunsError::NotFound);
    // diff_for(&grant, &f.workflow, 5) with no attempt at T5: Err(RunsError::NotFound).
}

#[tokio::test]
async fn the_planners_diff_shows_patches_up_to_its_bound() {
    // begin(3); commit backend/app/f1.txt, f2.txt, f3.txt, 900 lines each; report.
    // diff_for: three files; f1 and f2 patches contain "+line 900";
    // f3.patch == "[not shown: read it with task_file]" and (f3.added, f3.removed) == (900, 0).
    // diff(&attempt) still shows f3's whole patch.
}
```

  In `crates/core-runs/src/scope.rs`'s test module:

```rust
#[test]
fn a_path_under_the_folder_is_admitted_and_one_that_climbs_is_not() {
    let s = scope(&["backend/app/**"]);
    assert!(s.admits_under("", "backend/app/a.php"));
    assert!(s.admits_under("site/", "site/backend/app/a.php"));
    assert!(!s.admits_under("site/", "backend/app/a.php"));
    assert!(!s.admits_under("", "backend/app/../../.env"));
    assert!(!s.admits_under("", "backend/app/./a.php"));
    assert!(!s.admits_under("", "backend//app/a.php"));
}
```

- [ ] **Step 2: Run them to see them fail**

  Run: `cargo test -p shadows-core --test runs_attempts file` and `cargo test -p core-runs scope`
  Expected: compile errors naming `file`, `diff_for`, `file_for`, `admits_under`.

- [ ] **Step 3: Implement** `AttemptFile`, `Scope::admits_under`, `Git::blob`, `Runs::file`,
  `planner_commit`, `diff_for`, `file_for` by the Interfaces and Rules above; switch `verify` to
  `admits_under`. `MAX_BYTES: usize = 200_000` sits beside `MAX_LINES` in `diff.rs`.

- [ ] **Step 4: Run them to see them pass, and the report tests unchanged**

  Run: `cargo test -p core-runs` and `cargo test -p shadows-core --test runs_attempts`
  Expected: the six new tests pass; every earlier test passes.

- [ ] **Step 5: The tools.** In `run_tools.rs`, `TaskDiffArgs { workflow_id, task_number }` and
  `TaskFileArgs { workflow_id, task_number, path }` (doc comments as `TaskDoneArgs`'; `path`:
  "A path inside the task's writes, as task_diff names it."). `task_diff` answers
  `diff_for(...)` and `task_file` answers `file_for(...)`, both through `answer` and
  `Refusal::from`. Descriptions, exact:
  - `task_diff`: "Read what a task's latest committed attempt changed: each file with its added
    and removed line counts and its patch. Read it before you judge an attempt; never ask the
    executor for file contents. After an addition it shows only that attempt's change. Patches
    past 2,000 lines in all are not shown: read those files with task_file."
  - `task_file`: "Read one file of a task as its latest committed attempt left it, whole: any
    path inside the task's writes, changed by that attempt or not. text is null for a deleted
    or binary file; cut is true when Shadows shortened a long file."

  `THREAD_TOOLS` gains `"task_diff", "task_file"` after `"task_lessons"` (29). No other grant
  kind gains them.

- [ ] **Step 6: The texts.**
  - `prompt.txt`, "Running tasks": the line that starts "When an executor's attempt ends"
    becomes, exactly: "- When an executor's attempt ends, Shadows tells you in a message that
    starts with [Shadows]: the commit, the files it changed and the executor's report, or why
    it failed. Judge a commit yourself against the task's acceptance items: first read what it
    changed with task_diff, and any file of the task at that commit with task_file; never ask
    the executor for file contents, and count anything the report says was skipped or could
    not run as not done. Mark the task done with task_done only when every item holds;
    otherwise send task_amend with what is missing or must be extended." A new line follows
    it: "- task_diff and task_file read the task's latest committed attempt. After an addition,
    task_diff shows only that attempt's change: read the task's other files with task_file."
  - `notify.rs`, the committed notice's last sentence becomes: "Read its change with task_diff
    and task_file, then judge it against T{n}'s acceptance items before you mark it done."
  - `executor.md`, the commit bullet gains at its end: "After the commit, `git status --short`
    lists nothing inside your task's writes, not even a change of line endings; restore such
    a leftover with `git checkout -- <path>`." The `task_report` bullet's report becomes:
    "an honest report: what you did; each command you ran (tests, build, lint) with its exit
    code and the lines of its output that show the result; and anything you skipped or could
    not run. Do not paste file contents: the Planner reads your commit itself."

- [ ] **Step 7: Pin the texts.**
  - `mcp_tools.rs`: `the_planner_is_told_how_to_run_and_judge_tasks` adds
    `"first read what it changed with task_diff"`, `"never ask the executor for file contents"`
    and `"After an addition, task_diff shows only that attempt's change"`; every existing text
    stays. New `the_planner_reads_a_commit_with_its_own_tools` (async, as
    `the_planner_is_told_an_indexing_listing_is_not_complete`): the Planner's tool list holds
    `task_diff` with "never ask the executor for file contents" and `task_file` with "inside
    the task's writes"; the executor grant's list in
    `an_executor_grant_lists_its_tools_and_reads_only_its_task` is unchanged.
  - `mcp_server.rs`: the Planner's tool count 27 becomes 29.
  - `executor.rs` test module: `the_executor_reports_commands_and_leaves_its_writes_clean`
    asserts `FILE` contains "with its exit code", "Do not paste file contents" and "not even a
    change of line endings".

- [ ] **Step 8: The live test** in `planner_runs_live.rs`, on the subscription as the others:

```rust
/// The Planner hears T1 committed, reads the change with its own tools and
/// judges it: a done mark or an addition, never a request for file contents.
#[tokio::test]
#[ignore = "live: needs the signed-in Claude Code subscription"]
async fn live_the_planner_reads_the_commit_before_it_judges() {
    // As live_the_planner_runs_judges_and_amends_a_task, with the prompt:
    // "Run T1 of plan version {v1} with task_start, model sonnet. When Shadows tells you T1's
    //  attempt committed, judge it as your instructions say: task_done, or task_amend."
    // Wait up to 15 minutes for T1 done or a second attempt.
    // let notice = planner_entry(app, &["attempt 1 committed", "task_diff"]).await;
    // assert: a ToolCall entry after the notice's ordinal whose Debug text contains
    //   "task_diff" or "task_file";
    // println!("live review: done={} attempts={}", t1.done, attempts.len());
}
```

- [ ] **Step 9: Contracts.** `core-runs/contract.yaml`: `shapes.AttemptFile`; `functions.file`,
  `diff_for`, `file_for`, `planner_commit` (private) by the Rules above; the `check_free, checks,
  report` agreement gains `file` and the scope test; a new agreement
  `between: [diff, diff_for]` and `[file, file_for]`, "the Planner's tools and the routes run
  one method", `tested_by: [the_planner_reads_the_same_change_the_diff_route_serves]`; one
  obligation per new test; `notify`'s line names the two tools. `shadows-mcp`, `core-grants`
  and `core-harness` contracts name the two tools, the count and the texts.

- [ ] **Step 10: Run what the entry names, then commit.** `feat(backend): B-129 <goal>`.

---

## Task B-130: An executor's Windows environment keeps `LOCALAPPDATA`

**Why:** the same trial. The executor's shell had no `LOCALAPPDATA`, so Go could not find its
build cache and the executor set `GOCACHE` by hand. `Launch::command` clears a child's
environment and keeps only `WINDOWS_ESSENTIAL_ENV`, which lists `APPDATA` but not
`LOCALAPPDATA`. Every Windows tool that keeps a per-user cache (Go, npm, pip, Composer) reads
it, and it holds no secret.

**Files:**
- Modify: `crates/shadows-process/src/launch.rs` (`WINDOWS_ESSENTIAL_ENV` gains `"LOCALAPPDATA"`)
- Create: `crates/shadows-process/tests/environment.rs`

- [ ] **Step 1: Write the failing test** (Windows only):

```rust
#[cfg(windows)]
#[tokio::test]
async fn a_windows_child_keeps_local_app_data() {
    // spawn ProcessSpec { executable: std::env::var("ComSpec"), args: ["/d", "/c",
    //   "echo %LOCALAPPDATA%"], cwd: temp_dir(), env: Vec::new(), capture_stdout: true,
    //   pipe_stdin: false }; read stdout to its end; wait.
    // assert_eq!(stdout.trim(), std::env::var("LOCALAPPDATA").unwrap());
}
```

- [ ] **Step 2:** `cargo test -p shadows-process --test environment` fails (`%LOCALAPPDATA%`
  is echoed as written).
- [ ] **Step 3:** add `"LOCALAPPDATA"` after `"APPDATA"`.
- [ ] **Step 4:** `cargo test -p shadows-process` passes (the new test and every earlier one).
- [ ] **Step 5:** commit `fix(backend): B-130 <goal>`.

---

## Task B-131: The web reads a file at an attempt's commit, and each file's hunks

Blocked until `docs/shared/requests/2026-11-11-attempt-file-content.md` at `617d806f` is `accepted`.

**Files:**
- Modify: `crates/core-runs/src/model.rs` (`FileDiff.hunks: Vec<DiffHunk>`)
- Modify: `crates/core-runs/src/diff.rs` (the private hunk reader)
- Modify: `crates/core-runs/contract.yaml`
- Modify: `crates/shadows-core/src/lib.rs` (re-export `AttemptFile`)
- Modify: `crates/shadows-http/src/runs.rs` (route `file`)
- Modify: `crates/shadows-http/src/lib.rs` (`.routes(routes!(runs::file))`)
- Modify: `crates/shadows-http/src/openapi.rs` (only if `AttemptFile` must be listed)
- Modify: `crates/shadows-http/src/contract.yaml`
- Test: `crates/shadows-core/tests/runs_attempts/report.rs`
- Test: `crates/shadows/tests/runs_routes.rs`
- Test: `crates/shadows/tests/openapi.rs` (the route list gains `GET /api/attempts/{id}/files`)
- Regenerate: `api/openapi.json`

**Interfaces:**
- Consumes: `Runs::file`, `AttemptFile` (B-129); `shadows_agent::events::{DiffHunk, DiffLine, DiffOp}`
  (Runs already depends on `shadows-agent`).
- Produces: `GET /api/attempts/{id}/files?path=<path>` answering `AttemptFile`; 404 for an
  unknown attempt, one with no commit or a path not at its commit; 409 `outside_writes` for a
  path outside the task's writes (the existing `RunFailure` mapping, unchanged).
  `FileDiff { path, added, removed, patch, hunks }`.
- Hunks: read from the same git patch `diff` already runs, before it is cut. Each `@@ -a[,b]
  +c[,d] @@` starts a `DiffHunk { path, old_start: Some(a), new_start: Some(c), lines }`; a line
  led by ` `, `+`, `-` is `Context`, `Add`, `Remove` with that first character removed; `\ No
  newline at end of file` is skipped. Lines stop at `MAX_LINES` in all, as the patch is cut. A
  binary file has none.

- [ ] **Step 1: Failing tests.**
  - `report.rs`, `diff_answers_each_files_hunks`: a.php "one\ntwo\n" committed before; the
    attempt makes it "one\nTWO\nthree\n" and adds b.php "b\n". a.php: one hunk,
    `old_start == Some(1)`, `new_start == Some(1)`, ops and texts
    `[(Context, "one"), (Remove, "two"), (Add, "TWO"), (Add, "three")]`. b.php: one hunk,
    `old_start == Some(0)`, `new_start == Some(1)`, `[(Add, "b")]`.
  - `runs_routes.rs`, `a_file_at_an_attempts_commit_is_served`: over the route, after the
    existing diff test's commit, `GET /api/attempts/{id}/files?path=<a diff path>` is 200 with
    `text` the committed text and `cut` false; `path=nope.txt` inside writes is 404; an unknown
    attempt is 404.
- [ ] **Step 2:** run `cargo test -p shadows-core --test runs_attempts hunks` and
  `cargo test -p shadows --test runs_routes file`; both fail.
- [ ] **Step 3:** implement; `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi`.
- [ ] **Step 4:** both pass; `diff_returns_each_files_counts_and_patch` and B-129's tests
  unchanged.
- [ ] **Step 5:** contracts; commit `feat(backend): B-131 <goal>`; the request's backend answer
  gains one line: built.

---

## Found in the trial, left for later

- **Runs could refuse a report that leaves `writes` dirty** (the line-ending leftover in
  `backend/internal/domain/health.go` that made the next Start answer `dirty_scope`). B-129
  only tells the executor; a check in `report`, Start's own dirty check run on the reported
  commit, is a rule and a test, not a line.
- **A Codex Planner reads no file of the project**, only names (`project_files`). It plans
  without reading code. Reading the working tree is slice 2's (code intelligence) or a ruling.
- **Empty commits are accepted as an attempt's work.** Attempts 2 and 3 of T2 committed
  nothing, only to carry a report. With `task_diff` they are no longer needed; whether `report`
  should refuse an empty commit waits for a ruling.
