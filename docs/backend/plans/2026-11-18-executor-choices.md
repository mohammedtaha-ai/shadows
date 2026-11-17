# Executor choices: CLI, model and effort per task — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Each task stores its executor's CLI (claude-code or codex), model and effort, chosen from that CLI's own catalog. The Planner proposes the choice and the person edits it. Every Start runs on the last saved choice.

**Architecture:** Harness owns a harness's catalog: models in order, each with its own efforts. It reads the catalog once per daemon run from a short probe session and keeps it in memory. Runs owns the choice, stored per plan and task number outside the plan version, so a frozen version stays immutable while the person and the Planner change the choice. Start reads the saved choice in its admission write and records it on the attempt. Harness opens the executor on the attempt's CLI, model and effort; on Codex it runs with Codex's own tools in Codex's "Approve for me" mode.

**Tech Stack:** Rust 1.94+, SQLx 0.9 on SQLite, axum adapters, the ACP and Codex app-server connections in `shadows-agent`.

**Spec:** [executors design](../specs/2026-11-08-executors-design.md), D6 and rule 6 of the controller's settlements, amended by this plan's B-157. The rulings below are Mohammed's.

## Rulings (Mohammed, 2026-11-18)

1. **The choice lives on the task:**
   - CLI, model and effort are saved per task.
   - The Planner proposes them while planning, and the person changes them on the task page.
   - The last write wins, whoever made it.
   - Mohammed's Start and the Planner's `task_start` both run on the saved choice.
2. **Lists come from the CLI:**
   - Models and efforts are each CLI's own; Shadows keeps no list of either.
   - Claude Code shows the efforts of its selected model only. To get the full list, Shadows opens one probe session, switches through each model, reads that model's efforts, and closes the session.
   - The catalog stays in memory until the daemon restarts.
3. **No saved choice, no Start:** Start is refused with `no_executor_choice` until a CLI and model are saved.
4. **A Codex executor uses Codex's own tools:**
   - It gets the same instructions with a Tools section of its own, and runs in Codex's "Approve for me" mode (`ask`).
   - The check on its reported commit stays its write boundary, as for Claude Code.
5. **The choice follows the task, not its number** (Mohammed, 2026-11-18):
   - A task carried into a new version keeps its choice.
   - A task deleted and replaced by another under the same number starts empty, so the Planner or the person chooses again.
   - Plans' `TaskId` changes per version (`after_approval_start_draft_copies_tasks_and_links_with_their_numbers` asserts it). So Plans records, on each copied task, the task it was copied from, and the choice is found along that chain.
   - A save is accepted only on the plan's newest version (its Draft, or its latest Approved). A Superseded or Archived version refuses it.
6. **Small models for well-planned tasks** (Mohammed, 2026-11-18). The plan's contract already says what to build: the functions, their behavior and the acceptance. So the Planner picks the smallest model and effort that can build the task from its contract (for example Haiku on low effort), and a larger one only where the task needs judgment the contract cannot give, such as unclear existing code or a risky change.
7. **Replays answer their own result.** Every save keeps its command, fingerprint and result, so a resent older save answers what it answered then and never overwrites a later one. A Start command recorded before the upgrade still replays.
8. **A new conversation shows the Claude Code catalog before its first message:**
   - The draft composer reads the same catalog, starting on the remembered model and effort.
   - The send body already carries `model` and `effort`.

## Global Constraints

- **AppCore first.** Every change starts at `crates/shadows-core/src/app.rs` and stays in its owner:
  - Harness: the catalog and opening an executor on a harness.
  - Runs: the choice and Start.
  - Events: the project tail.
  - The adapters translate only.
- **Executor CLIs:** `policy::EXECUTOR_HARNESSES = ["claude-code", "codex"]`, in `shadows-agent/src/policy.rs`. AYLUL is never an executor. Codex counts only while it is configured (`Sessions::harness_available`).
- **No list of models or efforts in Shadows.** Every check of a model or effort reads `Harness::catalog`.
- **The choice spans versions through the task's lineage.** It is keyed by the `TaskId` it was saved on. A task's current choice is the first one found walking from its own `TaskId` back through `copied_from`. A corrected version keeps T9's choice, and a replaced T9 starts empty.
- **Next migration:** `0073`. The last shipped is `0072_check_foreign_leftovers.sql`.
- **Runs refusals** reach HTTP as `409` with Runs' own lowercase `code`, through `RunsError::code()` (`shadows-http/src/failure/runs.rs`). Harness refusals keep their `ErrorCode`s.
- **The Planner's tool list grows from 39 to 41** (`task_executor_set`, `executor_catalog`).
- **Edits:** Edit and Write tools only, never a script. Commit each path by name.
- **What the web sees waits for Masada's accepted request.** B-155 and B-156 change no route and no serialized type. B-157 and B-158 do (`Attempt`, `TaskRun`, `ReadyTask`, the Start body, the new routes), so both stay blocked until the request is accepted.

## Review Focus

1. **A saved model the CLI later stops offering.** Start still admits it. The opening then fails as today (`model: X is not offered`) and the attempt is `Failed` with that reason, never stuck `Running`. Test `a_saved_model_no_longer_offered_fails_the_attempt` (B-157).
2. **The probe fails** (CLI not signed in, Codex not configured):
   - The catalog answers `HARNESS_UNAVAILABLE` or `HARNESS_START_FAILED` and caches nothing, so the next call probes again.
   - A save that needs the catalog is refused, never stored unchecked.
   - Test `a_failed_probe_is_tried_again` (B-155).
3. **Two pages ask for the catalog at once.** One probe runs; the second caller waits for its result. Test `concurrent_catalog_reads_open_one_probe` (B-155).
4. **The Planner and the person save in the same second.** The later commit wins, the event fires once per save, and the page refetches. Test `the_last_save_wins_whoever_made_it` (B-157).
5. **An amend or a message to the executor reopens a Codex attempt's session.** It reopens on the attempt's own CLI, model and effort, never on the task's current choice. Test `an_addition_keeps_the_attempts_own_choice` (B-157).

---

## Task B-155: Harness reads each CLI's catalog

**Files:**
- Create: `crates/core-harness/src/catalog.rs` (one job: reading a harness's model and effort catalog through a probe session)
- Modify: `crates/core-harness/src/lib.rs` (the cache field on `Harness`; module list; re-exports)
- Modify: `crates/core-harness/src/model.rs` (`HarnessCatalog`, `CatalogModel`)
- Modify: `crates/core-harness/src/sessions.rs` (only if the probe needs the adapter spawn exposed `pub(crate)`; nothing else)
- Modify: `crates/shadows-agent/src/codex.rs` (`CodexProgram::catalog_spec(cwd)`: the app-server with no MCP server, for the catalog only), `crates/shadows-agent/src/codex/session.rs` (the existing `model/list` read, exposed to a catalog-only connection), `crates/shadows-agent/src/codex/connection.rs` (only if that connection needs it)
- Modify: `crates/shadows-agent/src/policy.rs` (`EXECUTOR_HARNESSES`)
- Modify: `crates/shadows-core/src/lib.rs` (re-exports)
- Modify: `crates/core-harness/contract.yaml`, `docs/codebase/README.md` (row for `catalog.rs`)
- Modify: `docs/backend/stages.md`
- Test: `crates/shadows-core/tests/harness_catalog.rs` (new)

No route and no serialized type change here. The route and `HarnessInfo.executor` are B-158's.

**Interfaces:**
- Produces:
  - `pub struct CatalogModel { id: String, label: String, description: Option<String>, efforts: Vec<Choice> }`. `efforts` is `[]` for a model that offers none. `Choice` is `shadows_agent::choices::Choice`.
  - `pub struct HarnessCatalog { harness: String, models: Vec<CatalogModel> }`, with models in the CLI's order.
  - `Harness::catalog(&self, kind: &str) -> Result<HarnessCatalog, HarnessError>`:
    - Answers the cached catalog when one exists.
    - Otherwise it probes, in an empty `catalog-probe` directory under Shadows' data directory, with no MCP server and no instructions. It then terminates the process.
    - **Claude Code:** the probe opens an ACP session, reads the `Offered`, then for each model sets the model option and reads that model's efforts.
    - **Codex:** the probe spawns the app-server through `catalog_spec` and reads `model/list`, which already names each model's `supported_reasoning_efforts`. It opens no thread, so it needs no grant. Today's opening path requires one MCP server (`codex.rs` `planner_server`), so the probe does not use it.
    - One probe per kind at a time (a per-kind `tokio::sync::Mutex<Option<HarnessCatalog>>`). Only a full read is cached.
    - Refusals: an unknown kind or `aylul` is `InvalidCommand`; Codex not configured is `HarnessUnavailable`; a spawn or handshake failure is `HarnessStartFailed`; a model switch the CLI refuses is `HarnessStartFailed` naming the model.
  - test-support:
    - `Harness::catalog_probes_for_test(&self, kind) -> usize`: how many probes ran.
    - `Harness::fail_next_probe_for_test(&self, kind)`: the next probe of `kind` fails as a spawn failure would.

- [ ] **Step 1: Write the failing tests** in `crates/shadows-core/tests/harness_catalog.rs`. The `live_` ones run on the signed-in subscription, as `checks_live.rs` sets up:
  - `live_the_claude_code_catalog_lists_each_models_efforts`:
    - `models` is non-empty.
    - At least one model has a non-empty `efforts`.
    - Every model id is one a fresh Claude Code thread's `open_session` offers.
  - `live_the_codex_catalog_lists_each_models_efforts`: the same, run only when Codex is configured.
  - `the_catalog_is_read_once`: two calls give equal catalogs and `catalog_probes_for_test("claude-code") == 1`.
  - `concurrent_catalog_reads_open_one_probe`: two joined calls give `catalog_probes_for_test == 1`.
  - `a_failed_probe_is_tried_again`: after `fail_next_probe_for_test("claude-code")`, the first call is `HarnessStartFailed`, the second answers the catalog, and `catalog_probes_for_test == 2`.
  - `codex_not_configured_is_unavailable`: `catalog("codex")` is `HarnessUnavailable` twice, with `catalog_probes_for_test("codex") == 0`.
  - `aylul_has_no_executor_catalog`: `catalog("aylul")` is `InvalidCommand`.
- [ ] **Step 2:** Run `cargo test -p shadows-core --test harness_catalog`. Expected: FAIL to compile.
- [ ] **Step 3:** Add `EXECUTOR_HARNESSES`, the types, Codex's catalog-only spawn, `catalog.rs` and the cache.
- [ ] **Step 4:** Run Step 2's command with `--include-ignored` on the signed-in machine, `cargo test -p shadows --test harness_choices`, and the rule 5 checks of `docs/backend/AGENTS.md`. Expected: PASS.
- [ ] **Step 5:** Update the contracts and the code-map row, set B-155 to done, and commit `feat(core): B-155 Harness reads each CLI's catalog`.

## Task B-156: Harness opens an executor on its attempt's CLI, model and effort

**Files:**
- Modify: `crates/core-harness/src/executor.rs` (`open_executor` takes the effort; mode and instructions per harness)
- Modify: `crates/core-harness/src/executor.md` (its `## Tools` section moves out; the body names a slot for it)
- Create: `crates/core-harness/src/executor_tools_claude.md` (the current Tools section, unchanged)
- Create: `crates/core-harness/src/executor_tools_codex.md` (Codex's Tools section)
- Modify: `crates/core-harness/src/setup.rs` (`for_executor` composes the instructions for the thread's harness; built-in tools only for claude-code)
- Modify: `crates/core-harness/src/live.rs` (the MCP tool list a session starts with follows its thread's kind: `Grants::tools_for(GrantKind::Executor)` for an executor thread, never the Thread grant's)
- Modify: `crates/core-turns/src/record.rs` (an executor thread's one allowed mode is `policy::executor_mode(harness)`, not `EXECUTOR_MODE`), `crates/core-turns/contract.yaml`
- Modify: `crates/shadows-agent/src/choices.rs` (the mode check accepts `policy::executor_mode(harness)` where it accepted `EXECUTOR_MODE`)
- Modify: `crates/core-runs/src/start.rs` (its `MODE` goes: `run_turn` sends the turn in `policy::executor_mode(harness)`) and `crates/core-runs/src/message.rs` (a message to the executor is sent in the same mode). Until B-157 records `attempt.harness`, both pass `claude-code`; B-157 passes the attempt's own.
- Modify: `crates/shadows-agent/src/policy.rs` (`executor_mode(kind) -> Option<&'static str>`)
- Modify: `crates/core-runs/src/start.rs`, `crates/core-runs/src/message.rs` (also pass `None` for the effort until B-157)
- Modify: `crates/shadows-core/tests/executor_session.rs`, `crates/shadows-core/tests/runs_attempts/live.rs` (the new argument)
- Modify: `crates/core-harness/contract.yaml`, `docs/codebase/README.md`
- Modify: `docs/backend/stages.md`
- Test: `crates/shadows-core/tests/executor_session.rs`, `executor::tests` in `executor.rs`

**Interfaces:**
- Produces:
  - `Harness::open_executor(&self, thread: &ThreadId, grant: TaskScope, model: String, effort: Option<String>) -> Result<OpenSession, OpenError>`:
    - The harness is the thread's (Threads' `create_executor_in` already records it).
    - It sets the model, then the effort when given. An effort the selected model does not offer is `OpenError::Start("effort: X is not offered")`.
    - Then it sets the mode `policy::executor_mode(harness)`.
  - `policy::executor_mode(kind)`: `Some("auto")` for claude-code, `Some(CODEX_ASK)` for codex, `None` otherwise. `EXECUTOR_MODE` goes.
  - `ExecutorSession::instructions(harness: &str) -> String`: the body of `executor.md` with that harness's Tools section in place.
  - `ExecutorSession::tools(harness) -> Option<Vec<String>>`: the front matter's list for claude-code, `None` for codex.
  - The Codex Tools section says:
    - "Edit files with apply_patch."
    - "Run the task's commands (tests, build, lint) in the shell."
    - "Never write a file through the shell or Python."
    - The same commit line and rules as Claude Code's.

- [ ] **Step 1: Write the failing tests:**
  - `executor::tests::each_harness_gets_its_tools_section`:
    - claude-code's instructions contain `Never use Python` and `MultiEdit`.
    - codex's contain `apply_patch` and not `MultiEdit`.
    - Both start with `[Shadows] You are an executor.`
  - The existing `the_executor_file_parses_and_names_its_eight_tools` reads the tools through `tools("claude-code")`.
  - In `executor_session.rs`:
    - `an_executor_opens_on_its_model_and_effort`: a Claude Code executor is opened with a model and one of that model's efforts from `Harness::catalog`, and the session's `current` holds both, in mode `auto`.
    - `an_effort_the_model_does_not_offer_refuses_the_opening`.
    - `a_codex_executor_session_lists_only_executor_tools` (Codex configured): the session's MCP tool list is `Grants::tools_for(GrantKind::Executor)`, and none of the Planner's.
    - `a_codex_executor_turn_runs_in_ask`: Turns accepts a turn on a Codex executor thread in `ask` and refuses `auto` there; a Claude Code executor thread is the reverse.
  - `live_a_codex_executor_commits_only_its_writes` in `runs_attempts/live.rs` is `#[ignore]` until B-157 can start on codex. It runs only when Codex is configured. B-157 un-ignores it and runs it from Start through `task_report` to the accepted commit.
- [ ] **Step 2:** Run `cargo test -p core-harness executor::` and `cargo test -p shadows-core --test executor_session`. Expected: FAIL.
- [ ] **Step 3:** Make the change above.
- [ ] **Step 4:** Run Step 2's commands, `cargo test -p shadows-core --test runs_attempts`, `cargo test -p core-turns`, and the rule 5 checks. Expected: PASS.
- [ ] **Step 5:** Update the Harness and Turns contracts (the executor comment block at lines 73–143 and `the_executor_file_parses...`). Set B-156 to done and commit `feat(core): B-156 an executor opens on its CLI, model and effort`.

## Task B-157: Runs stores each task's choice and starts on it

**Files:**
Changes what the web sees (`Attempt`, `TaskRun`, `ReadyTask`, what Start accepts). It is blocked until Masada's shared request is accepted.

- Create: `crates/core-db/migrations/0073_run_task_executors.sql` (also adds `task.copied_from TEXT NULL` for Plans)
- Modify: `crates/core-plans/src/store/draft.rs` (`insert_version` sets `copied_from` on each copied task), `crates/core-plans/src/lib.rs` and `crates/core-plans/src/store/plan.rs` (`Plans::task_lineage_in`), `crates/core-plans/contract.yaml`
- Modify: `crates/shadows/tests/plan_storage.rs` (the copy test also asserts `copied_from`)
- Create: `crates/core-runs/src/executor_choice.rs` (one job: saving and reading a task's executor choice)
- Create: `crates/core-runs/src/store/executor_choice.rs`
- Modify: `crates/core-runs/src/lib.rs` (module list; `task` answers `executor`; `suggested_model`, `EXECUTOR_HARNESS` go; re-exports)
- Modify: `crates/core-runs/src/model.rs`:
  - `ExecutorChoice`, `ExecutorInput`, `Chooser`.
  - `Attempt.harness` and `Attempt.effort`.
  - `TaskRun.executor` and `ReadyTask.executor` replace `suggested_model`.
- Modify: `crates/core-runs/src/error.rs` (`NoExecutorChoice` replaces `NoModel`; `ExecutorNotOffered(String)`)
- Modify: `crates/core-runs/src/start.rs` (no model argument; the choice read in `begin_attempt`; `open_executor` with the attempt's effort)
- Modify: `crates/core-runs/src/store/mod.rs` (`begin_attempt` reads the choice, creates the thread on its harness, records harness, model and effort; `continue_attempt` copies the previous attempt's `harness` and `effort` with its `model`; `by_command` also accepts a pre-upgrade Start fingerprint)
- Modify: `crates/core-runs/src/ready.rs` (`start_answer` answers `no_executor_choice`, so the task page and `task_ready` agree with Start)
- Modify: `crates/core-runs/src/checks.rs` (the automatic return after failed checks continues on the attempt's own harness, model and effort)
- Modify: `crates/core-runs/src/planner.rs` (`start_for` without a model; `set_executor_for`)
- Modify: `crates/core-runs/src/message.rs`, `crates/core-runs/src/amend.rs` (reopen on the attempt's own harness, model and effort)
- Modify: `crates/core-runs/src/ready_group.rs` (`executor` per ready task)
- Modify: `crates/core-events/src/store.rs` (`TaskExecutorChosen` in the project tail)
- Modify: `crates/shadows/tests/architecture/table-owners.txt` (`run_task_executors` and `run_executor_saves` → `runs`)
- Modify: `crates/shadows-core/src/lib.rs` (re-exports)
- Modify: `crates/shadows-http/src/openapi.rs` (register `ExecutorChoice` and `Chooser`, which `TaskRun` and `ReadyTask` now name) and `api/openapi.json`, regenerated in this commit with `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi`, since `Attempt`, `TaskRun` and `ReadyTask` change here
- Modify: every caller of `Runs::start` and `Runs::start_for`. Each drops the model and, where it ran on one, saves a choice first with the test helper below:
  - `crates/shadows-http/src/runs.rs` (B-158 changes the body; here only the call)
  - `crates/shadows-mcp/src/run_tools.rs` (only the call; B-158 does the tool)
  - `crates/shadows-core/tests/runs_attempts.rs`, `runs_attempts/live.rs`, `runs_attempts/planner.rs`
  - `crates/shadows-core/tests/run_admission.rs`, `task_conflicts.rs`, `ready_groups.rs`, `task_questions.rs`, `attempt_briefs.rs`
  - `crates/shadows/tests/task_questions_live.rs`, `planner_runs_live.rs`, `checks_live.rs`
  - any other caller the compiler names, with the same one change
- Modify: `crates/core-runs/src/testing.rs` (`choose_executor_for_test(workflow, task, harness, model, effort)`), or create it when absent with the crate's `test-support` gate
- Modify: `crates/core-runs/contract.yaml`, `crates/core-events/contract.yaml`, `docs/codebase/README.md`
- Modify: `docs/backend/specs/2026-11-08-executors-design.md`:
  - D6 now reads: "Each task stores its executor's CLI, model and effort. The Planner proposes them and Mohammed edits them on the task page; the last save wins, and every Start runs on it (Mohammed, 2026-11-18)".
  - Rule 6 is amended to match.
- Modify: `docs/backend/stages.md`
- Test: `crates/shadows-core/tests/runs_attempts/executor_choice.rs` (new `#[path]` module of `runs_attempts.rs`)

**Interfaces:**
- Produces:
  - **Tables:**
    - `run_task_executors (task_id TEXT PRIMARY KEY, plan_id TEXT, harness TEXT, model TEXT, effort TEXT NULL, chosen_by TEXT CHECK IN ('person','planner'), chosen_at TEXT)`: the choice saved on that version's task.
  - **Plans:** `Plans::task_lineage_in(conn, workflow: &WorkflowId, task: u32) -> Result<Vec<TaskId>, PlansError>`: the task's own id, then each `copied_from` back to the first version, newest first. A task added in a version has a lineage of one. It goes in Plans' `shared_in_transaction`.
    - `run_executor_saves (id TEXT PRIMARY KEY, plan_id TEXT, task INTEGER, result_json TEXT, saved_at TEXT)`: every save and what it answered, so a replay answers its own result. The command's identity lives in `command_record`, which points here.
  - **Columns:** `run_attempts.harness TEXT NOT NULL DEFAULT 'claude-code'` and `run_attempts.effort TEXT NULL`.
  - **Types:**
    - `pub enum Chooser { Person, Planner }`
    - `pub struct ExecutorInput { harness: String, model: String, effort: Option<String> }`
    - `pub struct ExecutorChoice { harness: String, model: String, effort: Option<String>, chosen_by: Chooser, chosen_at: String }`
  - `Runs::set_executor(&self, command_id: String, workflow: &WorkflowId, task: u32, input: ExecutorInput, by: Chooser) -> Result<ExecutorChoice, RunsError>`:
    - **The command** is a `CommandContext` of kind `run.executor_set`, schema version 1, with params `{workflow, task, harness, model, effort}`. A person's save uses `CommandContext::user`, and the Planner's names its agent the way other Planner commands do. It is recorded in `command_record`, whose `outcome_ref` is the id of the save's row in `run_executor_saves`.
    - **Replay first**, before the catalog is read, through core-db's classification: an equal fingerprint answers that save's stored `result_json`. It writes nothing and appends no event, even when the choice has since changed or the catalog is unavailable. Another fingerprint is `CommandConflict`.
    - **Pre-checks** before the catalog read, so a bad save fails fast: the task must exist in that version (`NotFound`). The version must be the plan's newest: its Draft, or its latest Approved when no Draft exists. A Superseded version is `Superseded` and an Archived one `Archived`.
    - `harness` must be in `policy::EXECUTOR_HARNESSES`; otherwise `ExecutorNotOffered("harness: X")`.
    - The input is checked against `Harness::catalog(harness)`, read before the write:
      - a model not listed is `ExecutorNotOffered("model: X")`;
      - an effort not among that model's efforts is `ExecutorNotOffered("effort: X")`.
    - **The effort is explicit.** A model that offers efforts needs one of them, and its absence is `ExecutorNotOffered("effort: required")`. `effort` is NULL only for a model that offers none. An attempt therefore never runs on an effort an earlier conversation left in the session.
    - Catalog errors pass through as `Unavailable` with their text.
    - **The write** is one transaction:
      1. It checks the replay again.
      2. It re-checks, in the same snapshot, everything the pre-checks read: the version is still the plan's newest, the plan is not archived, the task still exists, and, for the Planner, its grant is still live (`Grants`' `*_in` check). A v2 created while the catalog was read therefore refuses the v1 save with `Superseded`.
      3. It upserts the row keyed by the task's `TaskId` in that version, inserts the save with its result, records the command, and appends `TaskExecutorChosen { project_id, plan_id, task }`.
  - **Reading a choice** (Start's admission write, `task`, `ready_group`): `task_lineage_in`, then the first `run_task_executors` row along it.
  - test-support `Runs::pause_next_save_for_test(&self) -> (oneshot::Receiver<()>, oneshot::Sender<()>)`: the next save stops after its catalog read and before its write, says so on the first channel, and goes on when the second fires.
  - `Runs::set_executor_for(&self, grant: &Grant, workflow: &WorkflowId, task: u32, input: ExecutorInput) -> Result<ExecutorChoice, RunsError>`: the grant is checked as `planner_plan` checks it, but on any version state. It calls `set_executor` with `Chooser::Planner` and a fresh command id.
  - `Runs::start(&self, command: String, workflow: &WorkflowId, version: i64, task: u32) -> Result<Attempt, RunsError>`:
    - The fingerprint drops `model`.
    - **A pre-upgrade replay:** a recorded command whose stored fingerprint is not the new one is also an equal replay when it equals the old form, `{workflow, version, task, model}` with the recorded attempt's own model. It answers that attempt.
    - `begin_attempt` reads the choice in its write: none is `NoExecutorChoice`, and nothing is written.
    - It creates the executor thread on the choice's harness and records harness, model and effort on the attempt.
  - `Runs::start_for(&self, grant: &Grant, workflow: &WorkflowId, task: u32) -> Result<Attempt, RunsError>`.
  - `TaskRun.executor: Option<ExecutorChoice>` and `ReadyTask.executor: Option<ExecutorChoice>`.
  - `start_answer` refuses with `no_executor_choice` when the task has no choice, after the checks it makes today. A ready group therefore leaves such a task out, as it leaves out any task `start_answer` refuses.
  - `continue_attempt` (used by `amend` and by the return after failed checks) records the previous attempt's harness, model and effort, never the migration's defaults and never the task's current choice.
  - `Runs::task` no longer reads `Harness::remembered_model`.
  - **Codes:** `no_executor_choice` and `executor_not_offered`, both 409 like every Runs refusal.

- [ ] **Step 1: Write the failing tests** in `runs_attempts/executor_choice.rs`:
  - `a_start_without_a_choice_is_refused`: `NoExecutorChoice`, no attempt row, no thread.
  - `a_start_runs_on_the_saved_choice`: the person saves claude-code, a catalog model and one of its efforts. Start's attempt has those three, and the executor's session `current` model and effort match them.
  - `the_last_save_wins_whoever_made_it`: the Planner saves model A, then the person saves B. `task().executor` is B with `chosen_by: Person`, and the project tail holds two `TaskExecutorChosen`.
  - `a_choice_outlives_its_version`: T2's choice is saved on v1. A v2 is drafted and approved, and Start of v2's T2 runs on that choice.
  - `a_replaced_task_starts_empty`: T2 has a choice on v1. In v2's Draft, T2 is removed and a new T2 is added. v2's T2 has `executor: null`, and its Start is `NoExecutorChoice`.
  - `a_newer_save_wins_over_an_older_versions`: T2 is saved A on v1, then B on v2's Draft. v2's T2 reads B.
  - `a_choice_is_checked_against_the_catalog`: a model not in the catalog, and an effort of another model, are both `ExecutorNotOffered`, and nothing is stored.
  - `a_replayed_choice_answers_the_first`: the same command and body give the same choice; another body gives `CommandConflict`.
  - `a_replayed_older_save_keeps_the_latest`:
    - Save A (command a), then B (command b), then resend command a.
    - The answer is A's stored result, and `task().executor` is still B.
    - The tail holds two `TaskExecutorChosen`, not three.
    - The same holds after `fail_next_probe_for_test`, since the replay never reads the catalog.
  - `a_superseded_version_refuses_a_save`: v1 is superseded by v2, and a save through v1 is `Superseded`.
  - `a_save_racing_a_new_version_is_refused`:
    - `pause_next_save_for_test` stops a v1 save after its catalog read.
    - A v2 Draft is created, and the save goes on.
    - The save is `Superseded`, and nothing is stored or appended.
  - `a_model_with_efforts_needs_one`: a save of such a model with `effort: None` is `ExecutorNotOffered("effort: required")`.
  - `a_start_command_from_before_the_upgrade_replays`:
    - A database migrated from 0072 holds a Start recorded with the old fingerprint, written by SQL in the fixture as the old code wrote it.
    - The same command now answers that attempt, with no new attempt and no `CommandConflict`.
  - `the_task_page_says_no_choice`: a task with no choice has `start` refused with `no_executor_choice`, and `task_ready` leaves it out.
  - `an_addition_keeps_the_attempts_own_choice`: a Codex attempt (or, without Codex configured, a Claude Code one with an effort). The choice changes after Start, and `amend` records and reopens on the attempt's original harness, model and effort.
  - `a_checks_return_keeps_the_attempts_own_choice`: the automatic return after failed checks records the same three as the attempt it continues.
  - `a_saved_model_no_longer_offered_fails_the_attempt`: a row is written with a model the catalog lacks, through the test helper, which writes without the catalog check. The attempt ends `Failed` with reason containing `is not offered`.
  - Change `the_planner_start_needs_a_model` into `the_planners_start_needs_a_choice`. Delete `the_suggested_model_is_one_the_executors_harness_offers`, together with its contract rule.
  - Un-ignore B-156's `live_a_codex_executor_commits_only_its_writes`, starting it on a saved codex choice.
- [ ] **Step 2:** Run `cargo test -p shadows-core --test runs_attempts executor_choice::`. Expected: FAIL to compile.
- [ ] **Step 3:** Migration, store and `executor_choice.rs`.
- [ ] **Step 4:** Start, `start_for`, amend and message on the recorded choice; then update every caller listed in Files.
- [ ] **Step 5:** Regenerate `api/openapi.json`. Then run Step 2's command, all of `runs_attempts`, `run_admission`, `task_conflicts`, `ready_groups`, `task_questions`, `attempt_briefs` and `recovery`, `cargo test -p shadows --test openapi`, and the rule 5 checks. Expected: PASS.
- [ ] **Step 6:** Update the contracts. Runs gets the `choice:` section, the new `start` and `start_for` lines, the rules and the tests. Events gets the tail kind. Then amend the spec, set B-157 to done, and commit `feat(core): B-157 a task's executor choice, and Start runs on it`.

## Task B-158: the person's and the Planner's way to the choice

Changes what the web sees. It is blocked until Masada's shared request for the API below is accepted.

**Files:**
- Modify: `crates/shadows-http/src/runs.rs`:
  - `PUT /api/workflows/{workflow}/versions/{version}/tasks/{task}/executor`.
  - `StartAttempt` keeps only `command_id`.
- Modify: `crates/shadows-http/src/harness.rs` (`GET /api/harnesses/{kind}/catalog` → `Harness::catalog`)
- Modify: `crates/core-harness/src/model.rs`, `crates/core-harness/src/lib.rs` (`HarnessInfo.executor: bool`, true exactly for `policy::EXECUTOR_HARNESSES`, set in `list`)
- Modify: `crates/shadows-http/src/lib.rs` (route table), `crates/shadows-http/src/openapi.rs` (paths; `ExecutorChoice`, `ExecutorInput`, `Chooser`, `HarnessCatalog`, `CatalogModel`)
- Modify: `crates/shadows-mcp/src/run_tools.rs`:
  - `task_start { workflow_id, task_number }` has no `model`.
  - New `task_executor_set { workflow_id, task_number, harness, model, effort? }` and `executor_catalog {}`.
- Modify: `crates/shadows-mcp/src/server.rs` (router), `crates/core-grants/src/tools.rs` (Planner list 39 → 41)
- Modify: `crates/core-harness/src/prompt.txt`, which gets this paragraph: "While you plan, choose each task's executor with `task_executor_set`: a CLI, a model and an effort read from `executor_catalog`. A task you add or replace has no choice until you set one. Your contract tells the executor what to build, so choose the smallest model and lowest effort that can build it from the contract alone; choose larger only where the task needs judgment the contract cannot give, such as unclear existing code or a risky change. The person may change the choice; Start always runs on the latest."
- Modify: `crates/shadows/tests/openapi.rs`, `crates/shadows/tests/mcp_server.rs` (41 tools; the two names), `api/openapi.json`
- Modify: `crates/shadows-http/src/contract.yaml`, `crates/shadows-mcp/src/contract.yaml`, `crates/core-grants/contract.yaml`, `crates/core-harness/contract.yaml`
- Modify: `docs/backend/stages.md`
- Test: `crates/shadows/tests/runs_routes.rs`, `crates/shadows/tests/mcp_server.rs`, `crates/shadows/tests/planner_instructions.rs`, `crates/shadows/tests/harness_choices.rs`

**Interfaces:**
- HTTP:
  - `PUT …/tasks/{task}/executor` takes `{ command_id, harness, model, effort: string | null }`. It calls `Runs::set_executor(…, Chooser::Person)` and answers `ExecutorChoice`.
  - `POST …/tasks/{task}/attempts` takes `{ command_id }`.
- MCP:
  - `executor_catalog` calls `Harness::catalog` for each available kind in `EXECUTOR_HARNESSES`. It answers `[HarnessCatalog]`, with an unavailable CLI left out.
  - `task_executor_set` calls `Runs::set_executor_for`.
- [ ] **Step 1: Failing tests:**
  - `runs_routes.rs`:
    - `the_person_saves_a_tasks_executor_over_http`: PUT, then GET the task, and `executor` matches with `chosen_by: "person"`.
    - `start_without_a_choice_is_409_no_executor_choice`.
  - `mcp_server.rs`: the Planner's list has 41 tools, including both new names; the executor's list has neither.
  - `planner_instructions.rs`: `the_prompt_names_task_executor_set`, which asserts the prompt contains `task_executor_set`, `executor_catalog` and `smallest model`.
  - `harness_choices.rs`:
    - `the_harness_list_names_the_executor_clis`: `executor` is true for claude-code and codex and false for aylul.
    - `the_catalog_route_answers_claude_codes_models`: GET `/api/harnesses/claude-code/catalog` answers 200 with models, and `/api/harnesses/aylul/catalog` answers `INVALID_COMMAND` with that code's existing status.
- [ ] **Step 2:** Run `cargo test -p shadows --test runs_routes --test mcp_server --test planner_instructions --test harness_choices`. Expected: FAIL.
- [ ] **Step 3:** Adapters and the prompt, then `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi`.
- [ ] **Step 4:** Run Step 2's command, `--test openapi`, and the rule 5 checks. Expected: PASS.
- [ ] **Step 5:** Update the contracts, set B-158 to done, and commit `feat(core): B-158 the person and the Planner choose a task's executor`.

## The web API (for Masada's request)

| Call | Sends | Gets back |
|---|---|---|
| `GET /api/harnesses` | — | `HarnessInfo[]`. It gains `executor: bool`; `remembered: { model, effort }` is already there. |
| `GET /api/harnesses/{kind}/catalog` | — | `HarnessCatalog { harness, models: CatalogModel[] }`, where `CatalogModel { id, label, description, efforts: Choice[] }`. Errors: `HARNESS_UNAVAILABLE` (Codex not configured), `HARNESS_START_FAILED` (the probe failed; the next call tries again), `INVALID_COMMAND` (aylul or unknown). The first call per CLI takes a few seconds. |
| `GET …/versions/{version}/tasks/{task}` | — | `TaskRun.executor: ExecutorChoice \| null` **replaces** `suggested_model`. |
| `PUT …/versions/{version}/tasks/{task}/executor` | `{ command_id, harness, model, effort: string \| null }` | `ExecutorChoice { harness, model, effort, chosen_by: "person" \| "planner", chosen_at }`. `effort` is required when the model offers efforts and `null` only when it offers none. 409 `executor_not_offered` (its message names harness, model or effort, or says `effort: required`). |
| `POST …/tasks/{task}/attempts` | `{ command_id }` (**no `model`**) | `Attempt`, which gains `harness` and `effort`. 409 `no_executor_choice`. |
| Project stream | — | `TaskExecutorChosen { project_id, plan_id, task }`: refetch the task. |

For the draft composer:
- Before the first message it fills the model and effort lists from `GET /api/harnesses/{kind}/catalog`, starting on that harness's `remembered` model and that model's remembered effort.
- The first send already carries `model` and `effort`.
- No new Core route is needed for it.

## Task B-159: acceptance in the browser

Needs B-155 to B-158 and Masada's web tasks.

**Files:**
- Modify: `docs/backend/stages.md`
- Create: `docs/evidence/2026-11-18-executor-choices.md`

**Steps:**
- [ ] On a disposable project:
  1. A new conversation shows Claude Code's models and efforts before the first message, starting on the remembered ones.
  2. The Planner plans two tasks and saves a choice on each.
  3. Mohammed changes T2's model and effort on its page.
  4. Mohammed starts T1, and the Planner's `task_start` starts T2. Each attempt's Run tab and the logs show the saved CLI, model and effort.
  5. With Codex configured, one task runs on codex and commits only its writes.
- [ ] Run the signed-in live tests by name, with `--include-ignored`: both `live_..._catalog_lists_each_models_efforts` and `live_a_codex_executor_commits_only_its_writes`. Record each result, or "not run" with its reason.
- [ ] Record the dates, commits and log lines in the evidence file, set B-159 to done, and commit.
