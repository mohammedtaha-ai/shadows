# Codex commit recovery on Windows

> Implement the approved tasks through `superpowers:executing-plans` or the repository's
> subagent workflow, as chosen at handoff. The checkboxes are implementation steps.

**Goal.** A Codex executor can recover a scoped Git write through its configured approval
flow, and the Planner can tell whether a failed task may start again with the files it left.

**Owners.** `Runs` owns the executor commit instructions and Start admission. `Harness` owns
the Planner's instructions and the existing Codex session policy. `Grants` permits the new
read tool for Planners. `shadows-mcp` translates one call into `AppCore::runs()`.

**Branch.** `codex/code-record-live-fixes` at evidence commit `627a0e94`.

**Spec.** [Executors design, §6–8](../specs/2026-11-08-executors-design.md), especially
D10 and §7. [Harness controls, native Codex](../../core/specs/2026-09-24-harness-controls-design.md).
Record the approved refinements in the relevant sections of the executors design with the
implementing tasks. [Windows test evidence](../../evidence/2026-11-22-testing7-go-notes-live.md).

## Observed behavior

- During Setup in `testing7`, Codex received `Permission denied` creating `.git/index.lock`.
  Delays of 1, 2 and 4 seconds did not help. The attempt ended `Failed` with `ended without a
  commit` and two files remained. A second executor got the same refusal, requested the
  configured scoped approval, then staged and committed those exact paths as `fa15e24`.
- The Planner reported that the remaining files blocked another Start. The HTTP task read
  returned `start.allowed: true`, and Start accepted them. `Runs::checks` already admits
  leftovers after a qualifying failed, stopped or interrupted attempt releases its scope.
- `task_runs` reports held attempts. `task_ready` selects never-started, mutually compatible
  tasks. An already attempted task's absence from those results gives no Start decision.
  The HTTP task read already obtains that decision from `Runs::start_answer`.

## Rulings

- The executor makes and reports its own commit. Exact-path staging, `commit --only`,
  `writes`, the reported-commit checks and the task's judgment are part of the same path.
- A genuine index or ref lock that already exists keeps the existing 1/2/4-second retry
  rule. A permission refusal when creating a lock goes through the harness's normal
  approval flow for that exact operation, when offered. An unconfirmed cause stays
  unconfirmed. File presence and the command's result are evidence for the distinction.
- A preflight answer reflects the named version and the workspace at the time of the read.
  `task_start` checks again and remains authoritative. An ended attempt's scope must be
  released before any retry can proceed. The Planner retries an attempted task only when
  the person requests it.
- If the executor cannot commit, its final message gives the failed Git step, the concrete
  obstruction and the approval result when known. The existing failed-attempt notice
  carries that message. The terminal state stays `Failed` with no commit. If there is no
  usable diagnosis, the generic `ended without a commit` remains honest.
- This plan uses the current Codex `ask` policy, the existing approval handling and the
  current HTTP schema. The first delivery covers commit recovery. File-change attribution
  and Go symbol contracts have their own follow-ups. The Claude catalog issue is deferred
  until the user has Claude quota again.
- Automated coverage consists of the existing prompt assertion, one MCP behavior test,
  and the existing tool-list assertion. Native Git and approval outcomes are verified in
  one live Windows session in a disposable project. The repository's formatting,
  architecture and pre-PR gates apply at their usual points.

## B-194 · Distinguish Git locks from denied writes in the executor procedure

**Files**

- `crates/core-runs/src/lib.rs` (`COMMIT` and the existing unit test
  `the_first_prompt_says_how_an_attempt_ends`).
- `crates/core-runs/contract.yaml`.
- `docs/backend/specs/2026-11-08-executors-design.md` (§7).

**Interface.** The initial prompt and additions already consume `COMMIT`. The executor
uses existing Git commands and `task_report`. `Runs::notify` already includes the last
executor message for a failed attempt. Reuse these interfaces.

- [x] Extend `the_first_prompt_says_how_an_attempt_ends` to require the distinct
  permission-denial instruction and the final failure summary while retaining its checks
  for exact paths, lock delays and reporting the accepted SHA. Check that it fails against
  the old prompt.
- [x] Update the shared `COMMIT` text. Stage each new file by its exact path before attempting
  the commit. If a Git write was denied, ask for the existing approval for the exact
  failed `git add` or `git commit` command. Continue after the approved command succeeds.
  A refused or unavailable approval ends the attempt with the obstruction in the final
  message. Keep the bounded delay rule for a lock that exists. The executor leaves any
  existing lock and foreign staged entries alone and reports a SHA only after a commit.
- [x] Update the Runs contract and §7. Rerun the focused command and confirm it executes
  one test and passes. This assertion covers the prompt text. B-196 covers runtime behavior.

```powershell
cargo test -p core-runs --lib the_first_prompt_says_how_an_attempt_ends
```

## B-195 · Let the Planner read Start's answer for an attempted task

**Files**

- `crates/core-runs/src/planner.rs`, `crates/core-runs/contract.yaml`.
- `crates/shadows-mcp/src/run_tools.rs`, `crates/shadows-mcp/src/contract.yaml`.
- `crates/core-grants/src/tools.rs`, `crates/core-grants/contract.yaml`.
- `crates/core-harness/src/prompt.txt`, `crates/core-harness/contract.yaml`.
- `crates/shadows/tests/mcp_tools.rs`, `crates/shadows/tests/mcp_server.rs`.
- `docs/backend/specs/2026-11-08-executors-design.md` (§6 or its existing Start description).

**Interface.** Add the read-only Planner tool `task_start_status` with
`{ workflow_id, task_number }`, the same version identifier and task number as `task_start`.
It returns the existing `StartAnswer` shape, `{ allowed, code, reason }`.
`code` and `reason` are null when allowed. A refusal to start returns `allowed: false`
with Runs' existing code and reason. Invalid scope, absent task and read failures use the
existing MCP refusal path.

```rust
pub async fn start_status_for(
    &self,
    grant: &Grant,
    workflow: &WorkflowId,
    task: u32,
) -> Result<crate::StartAnswer, RunsError>
```

- [ ] Add one behavioral test `task_start_status_matches_the_task_read` in `mcp_tools`.
  Use the existing listener/plan fixtures and a disposable Git project. Approve the
  plan and save an offered executor choice before testing readiness. With an untracked
  file inside the write scope and no eligible predecessor, assert `allowed: false` and
  `dirty_scope`. Use a task with no unmet needs. Clear that fixture file before Start,
  keep the fixture's `release` file absent and outside the task's writes, and use the
  existing `Runs::set_first_prompt_for_tests("wait-for-release")` hook. After Start,
  recreate the scoped file, then create `release` so the fake turn ends without a commit.
  Use bounded polling for `Failed` and the released scope before asserting `allowed: true`
  for the same path. Compare the MCP answer with `Runs::task(...).start` in both cases.
  Repeat the read while checking the
  attempt count stays the same. Check that this test fails before registration.
- [ ] Implement `Runs::start_status_for` in `planner.rs`. Use the same `planner_plan`
  check as `start_for`, check that the task exists in that plan, and call the existing
  `start_answer(&plan, task)`. Leave the existing `ready_group` and Start checks in place.
- [ ] Translate the call in `run_tools.rs` with the existing Start argument shape and
  refusal mapper. Add `task_start_status` to the Planner list in `Grants`. Amend the
  existing `the_tool_list_depends_on_the_grant_kind` expectation for that list. The
  external Project and Executor lists keep their current permissions.
- [ ] Update `Running tasks` in the Planner prompt. On a failure or a requested retry,
  read `task_start_status` for that exact task. Report its reason when blocked. An allowed
  preflight means a person-requested retry may proceed through `task_start`. If the actual
  Start then refuses, report its current response. Describe `task_ready` as the
  never-started batch selector and `task_runs` as the held-scope list.
- [ ] Update the Runs, Grants, Harness and MCP contracts and the Start section of the
  executors design. Verify the two targeted commands. `api/openapi.json` stays byte-identical.

```powershell
cargo test -p shadows --test mcp_tools task_start_status_matches_the_task_read
cargo test -p shadows --test mcp_server the_tool_list_depends_on_the_grant_kind
git diff --exit-code -- api/openapi.json
```

## B-196 · Verify the recovery with native Codex on Windows

**Needs:** B-194 and B-195. The controller runs the live acceptance with the person in a
disposable Git project. Use the normal Windows debug build and the configured Codex
`ask` session. Its configured reviewer settles the Git approval. Record the build SHA,
model, attempt IDs, tool answers, commit and check verdicts in a dated `docs/evidence/`
record.

- [ ] **Permission refusal.** Start a small task whose contract creates one text file
  and checks its contents. Observe native Codex's Git call and the configured approval
  outcome. If creating `index.lock` is denied, capture whether the executor requests
  approval for the exact path command and what happens next. A refusal leaves an honest
  diagnostic, no commit and the files available for a later retry. A successful approval
  must result in a scoped commit and a verified report. Record the permission branch as
  unobserved if this environment lets the Git operation through on the first call. That
  run verifies the paths it exercised and leaves permission recovery open.
- [ ] **Contention and retry over leftovers.** Before Start, stage a fixture file outside
  the task's `writes`. Add `/.fixture-continue` to the fixture's Git excludes and leave
  that marker absent. In the task steps, have the executor write the expected text file,
  then wait up to 60 seconds for `.fixture-continue` before its first `git add`. Keep the
  saved contract check limited to the target file's contents. If the marker wait times
  out, end without attempting Git and record this fixture run as inconclusive.
- [ ] Once the target file appears, the tester creates `.git/index.lock` with exclusive
  create, closes the handle, and creates `.fixture-continue`. Keep the lock in place until
  the first attempt has exhausted its bounded retries, ended `Failed` and released its
  scope. Remove only the fixture lock the tester created. Leave the ignored marker present
  so the retry proceeds without a second pause.
- [ ] Ask the Planner whether that task can be retried in the same conversation. Capture
  `task_start_status` with `allowed: true` and the Planner's explanation. Ask it to retry.
  The next attempt receives the leftover file, commits it, reports the SHA and passes
  the checks before the Planner marks it done.
- [ ] **Personal edits still block.** In a separate task that has never run, put a
  personal untracked file inside its `writes`. Observe `allowed: false` with `dirty_scope`,
  confirm the Planner tells the person which path blocks it, and confirm the file remains.
  Compare the task's attempt count before and after. A request to Start while the file is
  there must refuse with the same reason.
- [ ] **Scope and judgment.** For the successful commit, inspect `git show --name-only`
  and the Shadows checks. A foreign path staged by the tester outside the task's `writes`
  remains staged and absent from the task commit. Capture the task's transition from
  attempt `Committed` to `Done` after review. Write observed, failed and unobserved cases
  explicitly in the evidence record.
