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
the Planner is told, and a live acceptance proves the path on an empty folder.

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
- Needs B-123 to B-127 built: the setup uses every Planner tool of slice 1b.

## Review Focus

1. **A file in the folder that no commit made** (a scaffold's leftover, a stray download): the
   setup task's acceptance says `git status` shows nothing uncommitted, and the Planner judges
   it. Held by the live test's assertion on `git status --porcelain`.
2. **The Planner plans in detail before the setup is done.** The prompt orders it; nothing
   refuses it (§5.2's OPEN). The live test's second turn checks that the detailed plan names
   files that exist.
3. **A folder that already holds the person's files** is made a repository by
   `project_git_init` with an empty commit; their files stay untracked and Start refuses a
   setup task whose `writes` cover them (`dirty_scope`). Not handled here; listed as an open
   question.

---

## Task B-100: The Planner sets the project up first, through a setup plan

**Files:**
- Modify: `crates/core-harness/src/prompt.txt`, `crates/core-harness/contract.yaml`
- Test: `crates/shadows/tests/mcp_tools.rs`

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
- `core-harness`' contract: the prompt's section list gains `Setup`, and the Agent Mode
  phrase it repeats goes (line 65's "explicitly requested action in Agent Mode").

- [ ] **Step 1: Write the failing test:** in `mcp_tools.rs`,
  `the_planner_sets_the_project_up_first` asserts `shadows_core::testing::PROMPT` contains
  `"set the project up before you plan anything in detail"`,
  `"git status shows nothing left uncommitted"` and `"call project_refresh"`, and does not
  contain `"Agent Mode"`.
- [ ] **Step 2: Run it and see it fail:** `cargo test -p shadows --test mcp_tools the_planner_sets`.
- [ ] **Step 3: Change** the prompt and the contract.
- [ ] **Step 4: Run** `cargo test -p shadows --test mcp_tools --test planner_mcp`;
  `cargo test -p core-harness`; `cargo test -p shadows --test architecture --test codemap --test contract_tests`.
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
     turn `Completed` and the project holds one plan in Draft whose title contains `Setup`.
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
