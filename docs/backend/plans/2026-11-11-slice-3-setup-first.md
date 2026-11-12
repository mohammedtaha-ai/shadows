# Slice 3: setup first, through the Planner — Implementation Plan

> **For agentic workers:** each task is built by `backend-builder` from its entry in
> [stages.md](../stages.md); the entry's Touch and Done when are binding, and so are this
> file's Interfaces and steps. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Before it plans in detail, the Planner sets the project up (git, the environment
such as database containers, the initial structure) as recorded commits, then plans on the
files that appeared.

**Architecture:** No new owner, worker, grant kind, thread kind, table or route. The setup is
an ordinary **setup plan**, the project's first plan: its tasks are run by executors that the
Planner starts with `task_start`, judges and marks done (slice 1b). Each one lands as a
reported commit that Runs checked. Git comes first through `project_git_init`. The Planner has
no write tool (`PLANNER_TOOLS` is Read, Grep, Glob, WebSearch, WebFetch, ToolSearch, Agent and
TodoWrite), so every file the setup creates has a commit as its source. What changes is what
the Planner is told, two Runs rules a real setup needs (Mohammed, 2026-11-11), and a live
acceptance that proves the path on an empty folder:

- **`project_git_init` takes the files the folder already holds** into its first commit,
  leaving out only what git ignores; an empty folder still gets an empty commit (B-128, a fix
  of the built B-123).
- **A setup task may commit binary files** (a scaffold's icon); every other task's report
  still refuses them. A task is a setup task when its plan version's title is `Setup`
  (trimmed, any case): the title the Planner is told to give the plan, read from the `Plan`
  that `Plans::get` already answers. No new field, flag or column (B-100).

**Why not the draft:** [the preparation draft](2026-11-09-project-preparation.md) added a
preparation worker, a `Preparation` grant and thread kind, a table and four routes (B-100 to
B-102). The program asked whether preparation could be an ordinary attempt instead; with
`project_git_init`, `task_start`, `task_done` and `task_amend` it can, with the same write
boundary and the same review. Mohammed's condition, "the Planner's setup lands as a recorded
commit, like an executor's", is what an attempt already does.

**Tech Stack:** Rust 1.94+; the Planner's prompt (`crates/core-harness/src/prompt.txt`).

**Spec:** [executors design §5.2](../specs/2026-11-08-executors-design.md#52-setup-before-detailed-planning-mohammed-2026-11-11).

## Re-read against the code, 2026-11-12

Since this plan was written, B-129 to B-135 landed. Checked against each, and amended in place:

- **B-133 (an executor's commit must change something).** No conflict for a task that writes
  files. But a setup task that only *runs* something (start the containers, run a migration)
  changes no file, so its attempt would end without a commit and fail. B-100's Setup section
  now says every setup task writes files, and running them belongs to the acceptance of the
  task that writes them.
- **B-132 (the Planner reads before it plans).** "Look before you write a path" already holds
  the reading rule and "Name a path that exists, or say plainly that the task creates it". B-100's
  last Setup lines repeated the second and only *listed* the new files; they now point at that
  section instead of restating it (one rule, one place).
- **B-134 and B-135 (executor notices; only the person approves).** No change: setup attempts
  reach the Planner like any attempt, in the executor's name, and the Planner never approves
  the Setup plan itself (B-101's test approves it as the person; a notice turn could not).
- **B-128 still fits `project_git_init`:** `Git::commit_empty` (`git.rs`) is called only by
  `init_git` (`init.rs`), and the three `init.rs` tests are as the task expects. The prompt's
  line still says "an empty first commit". Mohammed ruled on 2026-11-12 that the first commit
  leaves out secret and dependency files, through Code's inventory (task below).
- **B-136's cost is accepted** (Mohammed, 2026-11-12): an edit the person makes inside a running
  task's `writes` blocks its report until it is committed or removed.
- **Two items left from the trials now belong here** as B-136, before B-100: a report is refused
  while the task's `writes` hold uncommitted changes, and `FIRST_PROMPT` says to commit nothing
  when nothing changes. A setup's scaffold is exactly where leftovers appear, and a Claude
  Planner has no shell to run `git status` itself.

## Global Constraints

- The same as slice 1b (`docs/backend/plans/2026-11-11-slice-1b-planner-runs-tasks.md` at `617d806f`, its Global Constraints).
- Needs B-123 to B-135 built. Order inside the slice: B-128, B-136, B-100, B-101.

## Review Focus

1. **A file in the folder that no commit made** (a scaffold's leftover, a stray download):
   inside the task's `writes`, B-136 refuses the report until it is committed or removed;
   outside them, the setup task's acceptance says `git status` shows nothing uncommitted and
   the Planner judges it. Held by `a_report_is_refused_while_writes_hold_uncommitted_changes`
   and the live test's assertion on `git status --porcelain`.
2. **The Planner plans in detail before the setup is done.** The prompt orders it; nothing
   refuses it (§5.2's OPEN). The live test's second turn checks that the detailed plan names
   files that exist.
3. **A folder that already holds the person's files**: B-128's first commit takes them, so
   Start finds nothing uncommitted inside a setup task's `writes`. Held by
   `git_init_takes_the_files_the_folder_holds`. A `.env` or a `node_modules` in it, with no
   `.gitignore`, is left out and stays untracked and locally ignored. Held by
   `git_init_leaves_out_secrets_and_dependencies`.
4. **A binary file in a task that is not a setup task** is still refused. Held by the existing
   `a_commit_with_a_binary_file_is_refused`, whose plan is titled `Web`.
5. **A setup task that only runs something** (start the containers) changes no file and fails
   on B-133's rule. The Setup section forbids such a task; the live test shows whether the
   Planner follows it.

---

## Task B-128: `project_git_init`'s first commit takes the folder's files, never its secrets or dependencies

**Ruling (Mohammed, 2026-11-12):** the first commit leaves out what Code's inventory classes as
secret or dependency. Runs reads the inventory through Code, which AppCore now passes to it. The
files left out stay in the folder, untracked.

**Files:**
- Modify: `crates/core-runs/Cargo.toml` (`core-code`), `crates/core-runs/src/lib.rs`
  (`Runs::new` takes `code: Code`; the field), `crates/core-runs/src/git.rs`,
  `crates/core-runs/src/init.rs`, `crates/core-runs/src/error.rs` (`NotListed`),
  `crates/core-runs/contract.yaml`
- Modify: `crates/shadows-core/src/app.rs` (the `Runs::new` call only)
- Modify: `crates/shadows-mcp/src/run_tools.rs`, `crates/shadows-mcp/src/refusal.rs` (only if
  it lists Runs refusals by variant), `crates/shadows-mcp/src/contract.yaml`,
  `crates/core-harness/src/prompt.txt`, `crates/core-harness/contract.yaml`
- Modify: `docs/codebase/README.md` (only if `codemap` asks)
- Test: `crates/shadows-core/tests/runs_attempts/init.rs`, `crates/shadows/tests/mcp_tools.rs`

**Interfaces:**
- `Runs::new(storage, plans, harness, turns, projects, threads, code)`; `core-runs`' contract
  gains the dependency "code: Code lists the project's files, to leave secret and dependency
  files out of the first commit". Runs calls only `Code::refresh_inventory` and
  `Code::inventory`, with `Asker::Grant(&grant.project_id)`.
- `init.rs`, `init_git`, in order:
  1. The grant, the folder and `AlreadyGit` checks as today.
  2. `Code::refresh_inventory`, then `Code::inventory` page by page (`prefix ""`, `limit 200`,
     `after` the page's `next`). It collects every entry whose `excluded_reason` is `secret` or
     `dependencies`. A page whose `state` is `Indexing` or `Missing`, or an `InventoryStale`
     page after one fresh start from the first page, is
     `RunsError::NotListed` (code `not_listed`, text "the folder's file listing is not complete
     yet; call project_git_init again"). Nothing is written before this step ends.
  3. `git.init()`.
  4. `git.exclude(&paths)`: it appends one line per path to `.git/info/exclude`: `/` + the path
     with `\`, `*`, `?`, `[`, `]`, `!` and `#` escaped by `\`, and a trailing `/` for a
     folder (`InventoryKind::Directory`). This is git's own local ignore list inside `.git`,
     not a project file: the folder's files do not change, and the files left out stay
     untracked and out of `git status`, so neither Start's dirty check nor B-136's sees them.
  5. `git.commit_all("Initial commit")`.
  6. Logs `runs.git_initialised` with the commit and the number of paths left out.
- `git.rs`: `commit_empty` becomes
  `pub(crate) async fn commit_all(&self, message: &str) -> Result<String, String>`. It runs
  `git add -A`, then the same Shadows-identity commit with `--allow-empty` (so an empty folder
  still gets its first commit), and answers HEAD. New:
  `pub(crate) async fn exclude(&self, paths: &[(String, bool)]) -> Result<(), String>`, where
  each path is paired with `true` for a folder. It appends to `<root>/.git/info/exclude`,
  creating `info/` when git did not.
- `project_git_init`'s description: "Make this project's folder a git repository when Start
  says it is not one: git init and a first commit, made in Shadows' name, of the files the
  folder already holds (none in an empty folder). It leaves out what git ignores and the
  secret and dependency files project_files lists as left out; those stay in the folder,
  untracked. Refused when the folder is already inside a repository. It writes no project
  file; starter files are a task's work."
- `prompt.txt`, the `project_git_init` line becomes:
  `- project_git_init: when Start is refused because the project folder is not a git repository, make it one with project_git_init: git init and a first commit of the files the folder already holds (none in an empty folder), leaving out secret and dependency files, which stay in the folder untracked. It is refused inside an existing repository and writes no project file; starter files are a task's work.`

- [ ] **Step 1: Change and write the tests** in `runs_attempts/init.rs`:
  - `the_planner_initialises_git_in_a_folder_without_one` now expects `README.md` in the first
    commit (`git ls-tree -r --name-only HEAD` is `README.md`) and `git status --porcelain`
    empty, instead of an empty tree and `?? README.md`. The rest stays.
  - `git_init_takes_the_files_the_folder_holds`: remove `.git`. Write a `.gitignore` naming
    `notes.txt`, then `notes.txt`, `src/main.go` and a binary `assets/icon.ico` (bytes with
    NULs). `init_git` answers a sha whose tree (`git ls-tree -r --name-only HEAD`) is exactly
    `.gitignore`, `README.md`, `assets/icon.ico`, `src/main.go`. `notes.txt` is not in it and is
    still on disk. The author is `Shadows`. `task(&f.workflow, 1, 3).start.allowed` is true.
  - `git_init_leaves_out_secrets_and_dependencies`: remove `.git`. With no `.gitignore`, write
    `.env` (`KEY=x`), `node_modules/pkg/index.js` and `src/main.go`. `init_git` answers a sha
    whose tree is exactly `README.md`, `src/main.go`. `.env` and `node_modules/pkg/index.js` are
    still on disk with their content. `git status --porcelain --ignored` lists them as ignored
    (`!! .env`, `!! node_modules/`), and `git status --porcelain` is empty. `.git/info/exclude`
    holds the lines `/.env` and `/node_modules/`.
  - `git_init_in_an_empty_folder_makes_an_empty_commit`: remove `.git` and `README.md`.
    `init_git` answers a sha; `git ls-tree -r HEAD` is empty.
  - `the_planner_is_told_how_to_run_and_judge_tasks` asserts
    `"leaving out secret and dependency files, which stay in the folder untracked"`; its
    `"project_git_init: when Start is refused"` text stays.
- [ ] **Step 2: Run and see them fail:** `cargo test -p shadows-core --test runs_attempts git_init`.
- [ ] **Step 3: Implement** the Interfaces, wiring `code.clone()` into `Runs::new` in `app.rs`.
- [ ] **Step 4: Run** `cargo test -p shadows-core --test runs_attempts`;
  `cargo test -p shadows --test mcp_tools --test mcp_server --test planner_mcp`;
  `cargo test -p shadows --test architecture --test codemap --test contract_tests`.
- [ ] **Step 5: Contracts.** `core-runs`: the `code` dependency; `NotListed` in `RunsError`;
  `functions.new` and `functions.init_git` in the order above; the obligation names the first
  commit of the folder's files without its secret and dependency files, which stay untracked,
  with the new tests. `shadows-mcp` and `core-harness`: the texts.
- [ ] **Step 6: Commit** `fix(backend): B-128 project_git_init's first commit takes the folder's files, never its secrets or dependencies`.

`NotListed` has no test of its own. Making the listing stay `Indexing` past `refresh_inventory`'s
30-second wait cannot be done deterministically without a fake. The review reads it.

## Task B-136: A report leaves nothing uncommitted inside the task's writes

**Why:** the `testing3` trial. An executor left a line-ending-only change in
`backend/internal/domain/health.go` after its commit, and the next Start was refused with
`dirty_scope`. B-129 only tells the executor. A setup's scaffold leaves such files too, and a
Claude Planner cannot run `git status`. Separately, `FIRST_PROMPT` (`crates/core-runs/src/lib.rs`)
still tells every executor to commit, though B-133 refuses an empty commit.

**Files:**
- Modify: `crates/core-runs/src/start.rs` (the dirty read becomes one private method)
- Modify: `crates/core-runs/src/report.rs` (`verify` calls it last)
- Modify: `crates/core-runs/src/lib.rs` (`FIRST_PROMPT`; its unit test)
- Modify: `crates/core-runs/contract.yaml`
- Test: `crates/shadows-core/tests/runs_attempts/report.rs`

**Interfaces:**
- `start.rs`: `checks`' inline read becomes
  `async fn dirty(&self, git: &Git, prefix: &str, matcher: &Scope) -> Result<Vec<String>, RunsError>`
  (`pub(crate)`): `git.changed(prefix)` filtered by `matcher.admits`, the read Start already
  makes. `checks` calls it and answers `DirtyScope` as today.
- `report.rs`, `verify`: after the `BinaryFiles` check, `dirty` with the attempt's `writes`; a
  non-empty answer is `RunsError::DirtyScope { paths }` (existing variant, code `dirty_scope`,
  text "uncommitted changes inside the task's writes: ..."). A refusal leaves the attempt
  Running, as every report refusal does. A change outside `writes` (the person's file) does not
  refuse it.
- `FIRST_PROMPT` becomes, exactly: "Do task T{n}. Read it first: call task_context with
  task_number {n} and no workflow_id. When the work is done, stage only your own files inside
  the task's writes (never a binary or an installer), run git commit yourself, and leave
  nothing uncommitted inside the task's writes; then call task_report with the commit's sha
  and an honest report of what you did, ran and found. If nothing needs to change, commit
  nothing and end with a last message saying why."

- [ ] **Step 1: The failing tests.** In `runs_attempts/report.rs`:

```rust
#[tokio::test]
async fn a_report_is_refused_while_writes_hold_uncommitted_changes() {
    // begin(3); put backend/app/a.php and backend/app/b.php; put notes.txt (outside writes);
    // commit only a.php; report -> Err(RunsError::DirtyScope { paths }) with
    //   paths == ["backend/app/b.php"]; f.still_running(&attempt).
    // Remove b.php from disk; report the same sha -> Ok, Committed; notes.txt still untracked.
}
```

  In `crates/core-runs/src/lib.rs`'s test module (new if it has none),
  `the_first_prompt_says_how_an_attempt_ends`: `FIRST_PROMPT` contains "leave nothing
  uncommitted inside the task's writes" and "If nothing needs to change, commit nothing".
  Run `cargo test -p shadows-core --test runs_attempts uncommitted` and
  `cargo test -p core-runs first_prompt`: both fail.
- [ ] **Step 2: Implement** the Interfaces. Every earlier report test must pass unchanged; if
  one left an uncommitted file inside `writes` before its report, it now commits or removes
  it, and the builder names each such test in the report.
- [ ] **Step 3: Contract.** `core-runs`: `functions.report` names the dirty check last in its
  order; the `check_free, checks, report` agreement names `dirty` as Start's and report's one
  read; an obligation "A report is refused while the task's writes hold uncommitted changes;
  the attempt stays Running" tested by the new test; the first prompt's obligation names the
  two sentences.
- [ ] **Step 4:** run what the entry names; commit `fix(backend): B-136 <goal>`.

## Task B-100: The Planner sets the project up first, through a setup plan

**Files:**
- Modify: `crates/core-harness/src/prompt.txt`, `crates/core-harness/contract.yaml`,
  `crates/core-runs/src/report.rs`, `crates/core-runs/contract.yaml`
- Test: `crates/shadows/tests/mcp_tools.rs`, `crates/shadows-core/tests/runs_attempts.rs`
  (the `fixture_titled` helper), `crates/shadows-core/tests/runs_attempts/report.rs`

**Interfaces:**
- In "The project map", the sentence "You may do that in Agent Mode when the person asks." is
  removed: the Planner has no tool that writes files, so it was never true.
- A new section after "The roadmap", headed `Setup`, with these lines exactly:
  `- Once the roadmap is approved, set the project up before you plan anything in detail. The setup is the project's first plan; call it "Setup".`
  `- If Start says the folder is not a git repository, run project_git_init first.`
  `- Its tasks bring what the work needs before any feature: the environment (database and other service containers, an example environment file) and the initial structure, built with the framework's own scaffold command in each runnable part's folder, as the approved map places it.`
  `- Every setup task writes files. Running something (starting the containers, running a migration) is part of the acceptance of the task that writes what it runs, never a task of its own: a task that changes no file ends without a commit.`
  `- Each setup task's acceptance says what must work afterwards (the containers run and answer, the scaffold's own test or build passes) and that git status shows nothing left uncommitted.`
  `- When the person approves it, run its tasks with task_start, judge each as any task, and mark it done.`
  `- When the setup is done, call project_refresh, then read the files that appeared as "Look before you write a path" says, and plan the detailed work on them.`
  `- Only a task of the plan titled "Setup" (in any case) may commit binary files, such as a scaffold's icon; every other task's commit with a binary file is refused.`
- `core-harness`' contract: the prompt's section list gains `Setup`, and the Agent Mode
  phrase it repeats goes (line 65's "explicitly requested action in Agent Mode").
- **The binary rule in Runs** (`crates/core-runs/src/report.rs`): `verify` skips the
  `BinaryFiles` check when the attempt's plan version is a setup plan. A private
  `fn is_setup(plan: &Plan) -> bool` answers `plan.title.trim().eq_ignore_ascii_case("Setup")`
  (the version's title field as `Plans::get` answers it; check its name with the LSP);
  `verify` reads the plan with `read_plan(&attempt.workflow_id)`. Every other check, the
  `OutsideWrites` one included, is unchanged for a setup task. `core-runs`' contract:
  `functions.report` names the exception, with an obligation tested by the test below.

- [ ] **Step 1: Write the failing tests:**
  - In `mcp_tools.rs`, `the_planner_sets_the_project_up_first` asserts
    `shadows_core::testing::PROMPT` contains `"set the project up before you plan anything in
    detail"`, `"Every setup task writes files."`, `"git status shows nothing left
    uncommitted"`, `"read the files that appeared as \"Look before you write a path\" says"` and
    `"Only a task of the plan titled \"Setup\" (in any case) may commit binary files"`, and does not
    contain `"Agent Mode"`.
  - In `crates/shadows-core/tests/runs_attempts.rs`, `fixture_with` takes the source plan's
    title through a new `fixture_titled(title: &str)` that it and the other helpers call with
    `Web`; in `runs_attempts/report.rs`, `a_setup_task_may_commit_a_binary_file` (no agent):
    `fixture_titled("Setup")`, `begin(3)`, a binary `backend/app/favicon.ico` (bytes with
    NULs) committed with `commit`, `report` answers `Committed`. The existing
    `a_commit_with_a_binary_file_is_refused` (plan `Web`) still passes unchanged.
- [ ] **Step 2: Run them and see them fail:**
  `cargo test -p shadows --test mcp_tools the_planner_sets` and
  `cargo test -p shadows-core --test runs_attempts binary`.
- [ ] **Step 3: Change** the prompt, `report.rs` and the contracts.
- [ ] **Step 4: Run** `cargo test -p shadows --test mcp_tools --test planner_mcp`;
  `cargo test -p core-harness`; `cargo test -p shadows-core --test runs_attempts`;
  `cargo test -p shadows --test architecture --test codemap --test contract_tests`.
- [ ] **Step 5: Commit** `feat(backend): B-100 the Planner sets the project up first, through a setup plan`.

## Task B-101: A setup runs end to end on an empty folder

**Files:**
- Test: `crates/shadows/tests/planner_runs_live.rs`
- Modify: `crates/core-runs/contract.yaml` (one obligation naming the live test)

**Interfaces:** none new. The test uses B-124's listening app on `subscription_adapter()`,
`planner_turn_on`, `plan::draft_on` and the plan fixtures.

- [ ] **Step 1: Write the live test** `live_the_planner_sets_up_an_empty_folder`,
  `#[ignore = "live: needs the signed-in Claude Code subscription and Docker"]`:
  1. A listening app whose project folder is empty and is **not** a git repository; its
     roadmap seeded as `plan::draft_on` seeds it.
  2. Planner turn 1, on `SUBSCRIPTION_MODEL`: "The roadmap is approved. Set this project up: a
     Go module named example.com/shop in the folder's root, and a docker-compose.yml that
     runs PostgreSQL 16 on port 5432. Write the Setup plan now; do not run it yet." Assert the
     turn `Completed` and the project holds one plan in Draft titled `Setup`.
  3. The test approves it as the person does (`approve_plan_for_test`, after `tie_to_part`).
  4. Planner turn 2: "The Setup plan is approved. Run it." Poll up to 20 minutes until every
     task of that plan version is marked done (`runs().task(..).done`), printing each
     attempt's state as it changes.
  5. Assert: `git rev-parse HEAD` succeeds and `git log --format=%an` lists `Shadows` (the
     first commit) and other authors; every non-ignored file in the folder is tracked
     (`git status --porcelain` is empty); `go.mod` and `docker-compose.yml` exist and are in
     the commit of some task's latest attempt; the Planner's entries name `project_git_init`,
     `task_start` and `task_done`.
  6. Planner turn 3: "Now plan the first feature: an HTTP health endpoint. Write the plan."
     Assert the new plan's tasks name `go.mod` or a path under a folder the setup created,
     and no path that neither exists nor is marked as created by its task (`contract.files`
     with action `create`). Print `live setup: <commits in order>`.
- [ ] **Step 2: Run it**, signed in, with Docker running:
  `cargo test -p shadows --test planner_runs_live live_the_planner_sets_up -- --ignored --test-threads=1`.
  If it fails on Shadows' side (a refusal the Planner cannot act on, a commit refused for a
  scaffold's file), stop and report the refusal; do not widen any rule.
- [ ] **Step 3:** `core-runs` contract: an obligation "The Planner sets an empty folder up
  through a setup plan whose tasks land as reported commits, then plans on the files that
  appeared", tested by the live test. Run `cargo test -p shadows --test contract_tests`.
- [ ] **Step 4: Commit** `test(backend): B-101 a setup runs end to end on an empty folder`, with the
  printed line in the stages entry.

## B-102: dropped

The preparation routes, MCP tools and status of the draft (B-102) have nothing left to carry:
the setup plan is read through the plan and task pages and the task routes that exist.
