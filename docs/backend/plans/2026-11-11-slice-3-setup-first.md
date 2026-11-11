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

## Global Constraints

- The same as [slice 1b](2026-11-11-slice-1b-planner-runs-tasks.md#global-constraints).
- Needs B-123 to B-127 built: the setup uses every Planner tool of slice 1b. Order inside
  the slice: B-128, B-100, B-101.

## Review Focus

1. **A file in the folder that no commit made** (a scaffold's leftover, a stray download): the
   setup task's acceptance says `git status` shows nothing uncommitted, and the Planner judges
   it. Held by the live test's assertion on `git status --porcelain`.
2. **The Planner plans in detail before the setup is done.** The prompt orders it; nothing
   refuses it (§5.2's OPEN). The live test's second turn checks that the detailed plan names
   files that exist.
3. **A folder that already holds the person's files**: B-128's first commit takes them, so
   Start finds nothing uncommitted inside a setup task's `writes`. Held by
   `git_init_takes_the_files_the_folder_holds`.
4. **A binary file in a task that is not a setup task** is still refused. Held by the existing
   `a_commit_with_a_binary_file_is_refused`, whose plan is titled `Web`.

---

## Task B-128: `project_git_init`'s first commit takes the files the folder holds

**Files:**
- Modify: `crates/core-runs/src/git.rs`, `crates/core-runs/src/init.rs`,
  `crates/core-runs/contract.yaml`
- Modify: `crates/shadows-mcp/src/run_tools.rs`, `crates/shadows-mcp/src/contract.yaml`,
  `crates/core-harness/src/prompt.txt`, `crates/core-harness/contract.yaml`
- Test: `crates/shadows-core/tests/runs_attempts/init.rs`, `crates/shadows/tests/mcp_tools.rs`

**Interfaces:**
- `git.rs`: `commit_empty` becomes
  `pub(crate) async fn commit_all(&self, message: &str) -> Result<String, String>`: runs
  `git add -A`, then the same Shadows-identity commit with `--allow-empty` (so an empty folder
  still gets its first commit), and answers HEAD. What `.gitignore` excludes stays out; nothing
  else is left out.
- `init.rs`: `init_git` calls `commit_all("Initial commit")`. Its checks and refusals do not
  change. Its doc and the module doc say the first commit takes the folder's files.
- `project_git_init`'s description: "Make this project's folder a git repository when Start
  says it is not one: git init and a first commit, made in Shadows' name, of the files the
  folder already holds (none in an empty folder), leaving out what git ignores. Refused when
  the folder is already inside a repository. It writes no file; starter files are a task's
  work."
- `prompt.txt`, the `project_git_init` line becomes:
  `- project_git_init: when Start is refused because the project folder is not a git repository, make it one with project_git_init: git init and a first commit of the files the folder already holds (none in an empty folder). It is refused inside an existing repository and writes no file; starter files are a task's work.`

- [ ] **Step 1: Change and write the tests** in `runs_attempts/init.rs`:
  - `the_planner_initialises_git_in_a_folder_without_one` now expects `README.md` in the first
    commit (`git ls-tree -r --name-only HEAD` is `README.md`) and `git status --porcelain`
    empty, instead of an empty tree and `?? README.md`; the rest stays.
  - `git_init_takes_the_files_the_folder_holds`: remove `.git`; write `.gitignore` with
    `secret.txt`, `secret.txt`, `src/main.go` and a binary `assets/icon.ico` (bytes with NULs);
    `init_git` answers a sha whose tree (`git ls-tree -r --name-only HEAD`) is exactly
    `.gitignore`, `README.md`, `assets/icon.ico`, `src/main.go`; `secret.txt` is not in it and
    still on disk; the author is `Shadows`; `task(&f.workflow, 1, 3).start.allowed` is true.
  - `git_init_in_an_empty_folder_makes_an_empty_commit`: remove `.git` and `README.md`;
    `init_git` answers a sha; `git ls-tree -r HEAD` is empty.
  - `the_planner_is_told_how_to_run_and_judge_tasks` asserts
    `"a first commit of the files the folder already holds"` in place of B-123's sentence if
    that one changed.
- [ ] **Step 2: Run and see them fail:** `cargo test -p shadows-core --test runs_attempts git_init`.
- [ ] **Step 3: Implement** the Interfaces.
- [ ] **Step 4: Run** `cargo test -p shadows-core --test runs_attempts`;
  `cargo test -p shadows --test mcp_tools --test mcp_server --test planner_mcp`;
  `cargo test -p shadows --test architecture --test codemap --test contract_tests`.
- [ ] **Step 5: Contracts.** `core-runs`: `functions.init_git` and its obligation name the
  first commit of the folder's files and the new tests; `shadows-mcp` and `core-harness`: the
  texts.
- [ ] **Step 6: Commit** `fix(backend): B-128 project_git_init's first commit takes the files the folder holds`.

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
  `- Each setup task's acceptance says what must work afterwards (the containers run and answer, the scaffold's own test or build passes) and that git status shows nothing left uncommitted.`
  `- When the person approves it, run its tasks with task_start, judge each as any task, and mark it done.`
  `- When the setup is done, call project_refresh, read the files that appeared with project_files, and plan the detailed work on them. Name only paths that exist or that a task says it creates.`
  `- Only a task of the plan titled exactly "Setup" may commit binary files, such as a scaffold's icon; every other task's commit with a binary file is refused.`
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
    detail"`, `"git status shows nothing left uncommitted"`, `"call project_refresh"` and
    `"Only a task of the plan titled exactly \"Setup\" may commit binary files"`, and does not
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
