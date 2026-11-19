# The code record, slice 1: a task verified against its contract, and the record

Design: [`../specs/2026-11-19-project-code-record-design.md`](../specs/2026-11-19-project-code-record-design.md)
§3, §4, §8, §9 slice 1. Rulings of Mohammed, 2026-11-19: a new service owns it; a task is verified
before done; a missing symbol is returned as a failed check; an unchecked contract is said, never
silent; a later task that removes a done task's promised symbol is caught now, by the project
recheck; the executor is told up front its symbols are checked by name; the tests run a real
language server, not only recorded answers. Reviewed 2026-11-19 (the review's ten findings are
answered below, each where it applies).

## What is reused, and what is new

The comparison is stored as one more check result, so the checks pipeline (executors design §5.4)
carries it. Each row below was confirmed against the code by the review.

| Existing piece | Where | What it gives the `contract` check |
|---|---|---|
| Confirming checks on a quiet project, one project at a time | `Runs::confirm_attempt`, `wait_for_quiet`, `Inner::checking` (`crates/core-runs/src/checks.rs`) | No other task's unfinished files are in the folder |
| Check rows per attempt, started then finished | table `run_checks`, `RunsStore::start_check` / `finish_check`, `clear_unfinished` | Storage, restart signal, `GET /api/attempts/{id}/checks`, the Report tab |
| Return to the executor, at most two returns | `Runs::after_checks`, `RunsStore::returnable_checks`, `returns_since` | A missing symbol goes back to the same executor |
| The Planner's notice and the return prompt | `notify.rs` (`ended_intent`), `Runs::check_line` | The Planner and the executor read `contract failed` with its output |
| Done needs a reason when a stored row did not pass | `RunsStore::put_mark` (`store/marks.rs`), which loops over every stored row | A done over a missing symbol needs a reason |
| The project recheck after each done | `begin_recheck_in` (`store/rechecks.rs`), `Runs::run_recheck`, `end_recheck`, `failed_before` | A later task that removed a done task's promised symbol is told once |
| A language server question that wakes the project and waits for a starting server | `Codebase::lsp` (`crates/codebase/src/lsp/ask.rs`, `wait_running`, `START_WAIT` 20 s) | The facts |

New: `CodeRecord` with the comparison (B-170); Runs running it as the `contract` check (B-171); the
recheck running it for done tasks (B-172); the record written with the done mark (B-173); the Report
tab's row (F-129).

## Rulings for every task

**Ownership.** `CodeRecord` (crate `crates/code-record`) owns the rule "does the code hold what the
contract promised", its verdicts and, from B-173, the record. Runs owns check rows and their flow
and calls `CodeRecord`; `CodeRecord` never calls Runs. `CodeRecord` depends on `core-db`,
`core-projects` (`ProjectId`), `codebase` and `core-plans` (contract types only). It is built once
in `AppCore`'s `assemble_services` with the `Codebase` it shares.

**The check.** Its id is `contract`; Plans refuses a task check with that id. It is due for an
attempt when the attempt's contract (from its stored context, as `specs_in` reads it) has at
least one symbol whose file has action create or modify.

**What is compared.** The folder as it stands when the check runs: the project is quiet, so HEAD
is the attempt's commit or a later one (the design's "at the attempt's commit" is amended to this).

**Matching** (amends the design's "by name and kind"):
- Only these `documentSymbol` kinds count, for promised and unpromised alike: class (5), method
  (6), constructor (9), function (12). A variable, parameter or field never matches.
- A contract symbol's name is matched exactly, as written, against the counted symbols at any depth
  (a method inside a class is found by its bare name). A dotted name `Class.method` is matched
  against a method `method` whose parent is `Class`. The contract's schema text says names are
  bare or `Class.method`.
- Unpromised: a counted top-level symbol in a file the task creates that no contract symbol names.

**States of a promised symbol:**
- `found`: matched.
- `missing`: not matched in a complete answer; or its file does not exist in the folder (an
  absent create or modify file makes every symbol in it missing, never unchecked).
- `unchecked`: the answer was `truncated` and the name was not in what came back; or the file has
  no language server; or the question failed after the wait below.

**Waiting for a server.** `Codebase::lsp` waits 20 s for a starting server. A `NoServer` answer
whose `server` is `Some` and whose `state` is `Idle`, `Starting` or `None` (a server would read the
file but is not running yet, as right after a restart, when `runs.recover()` runs before
`code.start()`) is asked again every 5 s for up to 2 minutes. Any other `NoServer` (`server: None`,
or state `NotInstalled`, `Off` or `Failed`) makes the file unchecked at once, so a missing server
never holds the project's `checking` lock.

**The row's outcome and output:**
- `passed`: no symbol missing and at least one found.
- `failed`: a symbol missing.
- `not_run`: nothing could be checked (every promised symbol unchecked). It is shown, never
  silent: the Report tab and the Planner's notice read `contract not_run` with its output
  ("no language server reads backend/auth/schema.sql"). It is not returned to the executor and
  does not make a done mark need a reason (two explicit exclusions, B-171).
- The output speaks plainly, missing first, one line each:
  `T3 promises refresh_token in backend/auth/service.py; the code does not have it.`
  `backend/auth/service.py is promised and does not exist.`
  `unchecked: backend/auth/schema.sql (no language server reads it).`
  `unpromised: helper in backend/auth/service.py.`
  Unpromised symbols never fail it.

**Verdict keys.** `CodeRecord` stores a verdict under a key: the attempt id for an attempt's
check; `recheck:{attempt_id}` for a recheck, each recheck replacing the last (the result per batch is in `run_recheck_results`; amended in the branch review so the verdict table does not grow per batch).

**Tests run a real server.** Each task has fast tests over recorded answers (they run in CI) and
tests against a real `pyright-langserver`, marked `#[ignore = "needs pyright-langserver on PATH"]`
as `crates/shadows/tests/code_lsp.rs` marks its gopls tests. The controller runs the ignored ones with
`--include-ignored` before the task is done and records the result in the task's status line.

## Gate files every task here touches

Found building B-170. A new migration also updates `crates/core-db/tests/fixtures/base_migrations.sha384`
(its `sha384sum` line), the count in `crates/core-db/tests/migration_checksums.rs`, and a new table's name in the list in `crates/core-db/tests/storage_contract/schema.rs` (found in the full gate). A new table
gets its line in `crates/shadows/tests/architecture/table-owners.txt` (owner `code_record`). A
crate that comes to depend on `code-record` is added to its allowed owners in
`crates/shadows/tests/architecture/main.rs`. Test modules name their imports (the ratchet refuses
`use super::*`), and a `QueryBuilder` is given only whole statements. `cargo test -p shadows --test architecture`
and `cargo test -p core-db --test migration_checksums` are in every task's Done when.

## Task B-170 · `CodeRecord` and its comparison

**Files:** create `crates/code-record/{Cargo.toml, contract.yaml, src/lib.rs, src/model.rs,
src/verify.rs, src/store/mod.rs, src/error.rs, tests/verify.rs}`;
`crates/core-db/migrations/0075_code_record_verdict.sql`; modify
`crates/shadows-core/Cargo.toml`, `crates/shadows-core/src/app.rs` (`assemble_services`, the doc
table row), `crates/core-plans/src/task_contract.rs` (the schema text of `SymbolWork::name`),
`crates/core-plans/contract.yaml`, `docs/codebase/README.md`.

**Steps:**
1. Migration `0075`: `code_record_verdict (attempt_id TEXT PRIMARY KEY, project_id TEXT NOT NULL,
   verdict_json TEXT NOT NULL, verified_at TEXT NOT NULL)`.
2. `model.rs`: `SymbolState { Found, Missing, Unchecked }`; `PromisedSymbol { id, path, name,
   state, why: Option<String> }`; `Unpromised { path, name }`; `Verdict { symbols, unpromised,
   unchecked_files }` ordered by path then symbol id; `Verdict::outcome() -> VerdictOutcome
   { Passed, Failed, NotRun }`; `Verdict::lines() -> String` (the output above).
3. `CodeRecord::new(storage, code)`; `verify(&self, key: &str, project: &ProjectId, task: u32,
   contract: &TaskContract) -> Result<Verdict, CodeRecordError>` asks per distinct file:
   existence first (an absent file: its symbols missing), then `documentSymbol` as
   `Asker::Person(project)` (a system call; the contract says so), with the wait ruled above.
   It stores the verdict under `key` (an attempt id, or a recheck key from B-172), replacing one
   stored before. Only storage fails it.
4. `verdict(&self, key: &str) -> Result<Option<Verdict>, CodeRecordError>`.
5. Removing a project removes its verdict rows: `CodeRecord::delete_project_in(&mut Tx, project)`,
   called from `core-project-lifecycle/src/store.rs` beside `delete_code_index_in`
   (modify that file, its contract, and `ProjectLifecycle::new`'s caller in `app.rs`).
6. The parser of the `documentSymbol` result is private and tested by a unit test.

**Tests:** unit (in `src/verify.rs`): `document_symbols_count_classes_methods_and_functions_only`,
`a_truncated_answer_never_makes_a_symbol_missing`, `a_dotted_name_matches_a_method_in_its_class`.
`tests/verify.rs`, real pyright (ignored in CI), on a temporary Python project:
`live_a_promised_function_is_found_and_a_forgotten_one_missing`,
`live_an_absent_promised_file_makes_its_symbols_missing`,
`live_an_unpromised_function_in_a_created_file_is_listed`,
`live_a_file_no_server_reads_is_unchecked` (a `.sql` file).

**Done when:** `cargo test -p code-record`, the same with `-- --include-ignored` here (pyright),
`cargo test -p shadows --test codemap`, `cargo test -p shadows-core --features test-support --test contracts`
and clippy pass.

## Task B-171 · Runs runs the `contract` check

**Files:** `crates/core-runs/{Cargo.toml, src/lib.rs, src/checks.rs, src/brief.rs,
src/store/checks.rs, src/store/marks.rs, contract.yaml}`; `crates/shadows-core/src/app.rs`
(pass `code_record` to `Runs::new`) and `crates/shadows-core/Cargo.toml` (its `test-support`
gains `code-record/test-support`); `crates/code-record/{Cargo.toml, src/lib.rs, src/verify.rs,
contract.yaml}` (the seam below); `crates/core-plans/src/task_contract.rs` and its contract
(refuse a check named `contract`); tests in `crates/shadows-core/tests/runs_attempts/`.

**Steps:**
1. `Runs::new` takes `CodeRecord`.
2. In `confirm_attempt`, compute `contract_due` first. Both early returns ("no checks" and "every
   check recorded") return only when the `contract` row is not due or is finished. Under the
   project's `checking` lock, after the task's own checks: insert the `contract` row unfinished
   through the existing `start_check` with `TaskCheck { id: "contract", run: [] , ..}` (its other
   fields empty) and the next ordinal after the task's checks, then `CodeRecord::verify(attempt
   id, …)`, then `finish_check` with the verdict's outcome, `output = lines()`, `head_after`.
   `clear_unfinished` already resets an unfinished row of the attempt's own task at a restart.
3. `returnable_checks` excludes a `contract` row whose outcome is `not_run`.
4. `put_mark`'s reason rule: a `contract` row unfinished counts as "no finished result" (needs a
   reason, as an unfinished check does); one finished `not_run` does not need a reason.
5. The executor's first prompt (`FIRST_PROMPT` in `src/lib.rs`, every executor reads it) gains:
   "When its contract names symbols, Shadows checks by name that each one exists in its file after
   your commit: keep the names as written." (Amended 2026-11-19: the brief is an API answer, and a
   new field there would change `api/openapi.json` for one sentence.)
6. Plans refuses a task check whose id is `contract`.

**The test seam lives in `CodeRecord`** (amended 2026-11-19 on the builder's report: a field on
`CoreParts` would have touched 36 test files). Under a new `test-support` feature of
`code-record`, never a default: a public `RecordedAnswer { Symbols(serde_json::Value) (a
`documentSymbol` result, not truncated), NoServer, Absent }` and
`CodeRecord::record_answer_for_tests(&self, path: &str, answer: RecordedAnswer)`, kept in a
`std::sync::Mutex<HashMap<String, RecordedAnswer>>` field that exists only under the feature;
`ask()` answers from it first when the path is recorded, and asks the language server otherwise.
Tests reach it through the existing `app.code_record()`. `shadows-core`'s `test-support` enables
`code-record/test-support`; CI's step 5 (no `test-support` in the shipped tree) still holds.

**Tests** (`crates/shadows-core/tests/runs_attempts/contract_check.rs`, with recorded answers):
`a_task_with_no_checks_still_gets_its_contract_checked`,
`a_missing_symbol_goes_back_to_the_executor_with_its_name`,
`a_found_contract_passes_and_the_notice_says_so`,
`done_while_the_contract_check_runs_needs_a_reason`,
`no_language_server_is_said_and_not_returned`,
`a_check_named_contract_is_refused`; and one real-pyright test, ignored in CI:
`live_an_attempt_missing_a_promised_function_is_returned`.

**Done when:** those tests, with `--include-ignored` here, `cargo test -p core-runs --features test-support`,
the contract test and clippy pass.

## Task B-172 · The project recheck runs the `contract` check of done tasks

**Files:** `crates/core-runs/src/recheck.rs`, `src/store/rechecks.rs`, `contract.yaml`; tests in
`crates/shadows-core/tests/runs_attempts/`.

**Steps:**
1. `begin_recheck_in` adds, for each done task whose judged attempt had a `contract` row, a
   recheck row with `check_id 'contract'`, `run_json` the valid `TaskCheck` JSON
   `{"id":"contract","run":[]}` (so `run_recheck` and `recheck_result` parse it as today), and
   ordinal the done task's check count (after its own checks; `recheck_rows` orders by it).
2. `run_recheck` branches on `check_id == "contract"` before running: such a row goes through
   `CodeRecord::verify` (key `recheck:{attempt_id}`, the contract from the judged
   attempt's stored context) instead of `run_check`, and is finished with the verdict's outcome.
3. `end_recheck` leaves out a `contract` row whose outcome is `not_run` explicitly (recorded,
   never told). `failed_before` skips earlier `contract` rows that are `not_run`, so a server that
   was down once never hides the next real failure. A `failed` row is told to the marking plan's
   Planner once, and again only after it passed in between, as any row.

**Tests:** `a_later_task_removing_a_done_tasks_function_is_told_once` and
`a_not_run_recheck_never_hides_a_later_failure` (recorded answers), and
`live_a_later_task_removing_a_done_tasks_function_is_told` with real pyright, ignored in CI.

**Done when:** those tests (with `--include-ignored` here), the core-runs tests and clippy pass.

## Task B-173 · The record, written with the done mark

**Files:** `crates/core-db/migrations/0076_code_record.sql` (a new file: 0075 may already have
run on a development database); `crates/code-record/src/store/`, `src/record.rs`, `src/model.rs`,
`contract.yaml`, `tests/record.rs`; `crates/core-runs/src/lib.rs` (`mark_done`),
`src/store/marks.rs`, `contract.yaml`. Built after B-172 (both edit `core-runs/contract.yaml`).

**Steps:**
1. Tables `code_record_file (project_id, workflow_id, version, task, path, action, purpose,
   commit_sha, recorded_at)` keyed (workflow_id, version, task, path); `code_record_symbol
   (project_id, workflow_id, version, task, symbol_id, path, name, kind, contract_json, state)`
   keyed (workflow_id, version, task, symbol_id); `code_record_unpromised (project_id,
   workflow_id, version, task, path, name)`. `delete_project_in` removes them too.
2. Two associated functions, called inside another service's write as
   `Codebase::delete_code_index_in` is (so `RunsStore` holds no `CodeRecord`):
   `CodeRecord::record_done_in(tx: &mut Tx, done: &DoneRecord)` replaces the task's rows, where
   `DoneRecord { project, workflow, version, task, attempt: Option<String>, commit:
   Option<String>, contract: TaskContract, verdict: Option<Verdict> }`; and
   `CodeRecord::forget_done_in(tx, workflow, version, task)` deletes them.
3. Before the write, `Runs::mark_done` (done = true) reads the task's latest committed attempt (if
   any), its stored context's contract (the contract the check verified; the frozen version's when
   no attempt exists), its commit, and its verdict (`CodeRecord::verdict(attempt id)`), and passes
   them to `put_mark` as an `Option<DoneRecord>`. `commit` and `attempt` are `None` for a task
   done with no attempt, and then every symbol is `unchecked`.
4. Inside its write, `put_mark` checks that the latest committed attempt is still the one in the
   `DoneRecord`; if another committed meanwhile, it records with `verdict: None`. Then done →
   `record_done_in`, not done → `forget_done_in`. A replayed mark writes nothing (it returns before,
   as today). So the record and the mark never disagree, also after a crash.
5. No read method and no route in this slice: their first user is `code_where` (slice 2). The
   tests read the tables directly in the owning crate.

**Tests** (`crates/code-record/tests/record.rs` and one in `runs_attempts`):
`done_writes_the_record_and_not_done_removes_it`, `a_second_done_replaces_the_rows`,
`a_task_without_a_contract_records_nothing`, `removing_the_project_removes_its_record`.

**Done when:** those tests, the core-runs tests, the codemap and contract tests and clippy pass.

## Task F-129 · The Report tab shows the `contract` row as Shadows' own

In `web/src/app/check-row.tsx`, a row whose id is `contract` shows no command line and no
"Reported" field (`reported={false}`), its title "Contract", and its output as written. Test in the
Report tab's test. Written into `docs/frontend/stages.md` by the controller for Masada's builder.

## Task B-174 · Acceptance on a real provider

On a Python project with pyright, Planner and executors on Sonnet, recorded in `docs/evidence/`:
1. T1 promises `add`, `delete` and `list_all` in `notes/store.py`. The person asks the Planner to
   add, in T1's steps, "do not write list_all yet". Observe `contract failed` with
   `T1 promises list_all…`, the return, the executor's second commit with `list_all`,
   `contract passed`, the done mark and the record's rows.
2. T2 (needs T1) deletes `list_all`. Observe its done mark's recheck telling the Planner once
   that T1's `contract` failed.
3. A task promising a symbol in a `.sql` file: `contract not_run` with its reason on the page and
   in the notice; no return.
