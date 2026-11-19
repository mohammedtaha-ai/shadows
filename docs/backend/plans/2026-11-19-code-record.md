# The code record, slice 1: a task verified against its contract, and the record

Design: [`../specs/2026-11-19-project-code-record-design.md`](../specs/2026-11-19-project-code-record-design.md)
§3, §4, §8, §9 slice 1 (approved by Mohammed, 2026-11-19).

## What is reused, and what is new

Most of slice 1 already exists in Runs for checks (executors design §5.4). The comparison is stored
as one more check result, so all of this works on it unchanged:

| Existing piece | Where | What it gives the `contract` check |
|---|---|---|
| Confirming checks on a quiet project, one project at a time | `Runs::confirm_attempt`, `wait_for_quiet`, `Inner::checking` (`crates/core-runs/src/checks.rs`) | The code compared is the attempt's commit, with no other task's unfinished files |
| Check results per attempt | table `run_checks`, `RunsStore::finish_check`, `Runs::check_results` | Storage, the `GET /api/attempts/{id}/checks` route, the task page's Report tab |
| Return to the executor, at most two returns | `Runs::after_checks`, `RunsStore::returnable_checks`, `returns_since` | A missing symbol goes back to the same executor with the result |
| The Planner's notice | `Runs::notify` (`notify.rs`), `Runs::check_line` | The Planner reads the result when it judges |
| Done needs a reason when a check did not pass | `RunsStore::put_mark` (`store/marks.rs`) | A done mark over a missing symbol needs a reason |
| Re-running unfinished checks after a restart | `Runs::confirm_attempt` at recovery | The check runs again if it has no result |
| Asking a language server, waking the project and waiting for a starting server | `Codebase::lsp` (`documentSymbol`) | The facts |

New: the `CodeRecord` service with the comparison rule (B-170), Runs running it as the `contract`
check (B-171), and the record written on done (B-172).

## Rulings for every task

- `CodeRecord` (crate `crates/code-record`) owns the rule "does the code hold what the contract
  promised" and, from B-172, the record's tables. Runs owns check results and their flow, and
  calls `CodeRecord`; `CodeRecord` never calls Runs. `CodeRecord` depends on `codebase` and
  `core-db` (and `core-plans` for the contract types only).
- The check's id is `contract`. A task's own check may not be named `contract`: Plans refuses it
  in B-171 (one line in `ContractRules::problems`).
- Matching: a contract symbol is `found` when the language server's `documentSymbol` answer for
  its file has a symbol of the same name (children flattened, so a method inside a class counts).
  Kinds are not compared in this slice: servers name kinds differently and a wrong kind is a
  question for the Planner, not a missing function. Names compare exactly as written.
- `unchecked`: the file has no language server (`LspAnswer::NoServer`) or the question failed.
  `unpromised`: a top-level symbol (not a child) in a file the task creates that no contract
  symbol names.
- The check's outcome: no row when every contract file is unchecked; else `passed` when no
  checked symbol is missing, `failed` when one is. Unpromised symbols are listed, never fail it.
  Its `output` is the readable list, missing first:
  `missing: refresh_token in backend/auth/service.py`, `unpromised: helper in backend/auth/service.py`,
  `unchecked: backend/auth/schema.sql (no language server)`.
- A task with no contract, or a contract with no symbols, gets no `contract` check.

## Task B-170 · `CodeRecord` and its comparison

**Files:**
- Create: `crates/code-record/Cargo.toml`, `src/lib.rs`, `src/model.rs`, `src/verify.rs`,
  `src/store/mod.rs`, `src/error.rs`, `contract.yaml`, `tests/verify.rs`;
  `crates/core-db/migrations/0075_code_record.sql` with its first table,
  `code_record_verdict (attempt_id PRIMARY KEY, project_id, verdict_json, verified_at)`.
- Modify: `Cargo.toml` (workspace members are `crates/*`, so nothing unless a list names crates),
  `crates/shadows-core/Cargo.toml`, `crates/shadows-core/src/app.rs` (build once in
  `AppCore::start` with `code.clone()`; accessor `code_record()`; its row in the doc table),
  `docs/codebase/README.md` (its row; `cargo test -p shadows --test codemap` checks it).

**Shape:**
- `model.rs`: `SymbolState { Found, Missing, Unchecked }`; `PromisedSymbol { id, path, name,
  state }`; `Unpromised { path, name }`; `Verdict { symbols: Vec<PromisedSymbol>, unpromised:
  Vec<Unpromised>, unchecked_files: Vec<String> }` with `outcome() -> Option<bool>` (None = nothing
  checked, Some(true) = passed) and `lines() -> String` (the output above). Ordered by path, then
  symbol id: never by map iteration.
- `CodeRecord::new(storage, code: Codebase)`; `CodeRecord::verify(&self, attempt: &str, project:
  &ProjectId, contract: &TaskContract) -> Result<Verdict, CodeRecordError>`: one `documentSymbol`
  per distinct contract file with action create or modify that the contract's symbols name, asked
  as `Asker::Person(project)`; a failed question makes that file `unchecked` and is logged
  `code_record.lsp_failed`. It stores the verdict under the attempt (replacing one stored before),
  so the record is later written from it; only a storage failure is an error.
- `CodeRecord::verdict(&self, attempt: &str) -> Result<Option<Verdict>, CodeRecordError>`.
- The answer's parsing is a private function over the `result` JSON (`DocumentSymbol[]` with
  `children`, or `SymbolInformation[]` with `name` and no children), so it is tested without a
  server.

**Tests (`tests/verify.rs`, through the public API where possible; the parser by a unit test in
`src/verify.rs`):**
1. `document_symbols_are_read_with_their_children`: nested `DocumentSymbol` JSON gives the class
   and its methods as names, and only the class as top level; `SymbolInformation` JSON gives its names.
2. `a_promised_symbol_is_found_or_missing`: a verdict built from a contract with `login`,
   `refresh_token` and an answer holding `login`: `login` found, `refresh_token` missing,
   outcome failed, `lines()` starts with the missing line; `verdict(attempt)` reads it back.
3. `a_file_without_a_server_is_unchecked_and_alone_makes_no_check`: every file unchecked →
   `outcome()` is None.
4. `an_unpromised_symbol_is_listed_and_does_not_fail`.
For 2 to 4, `verify` takes the answers through a `#[cfg(feature = "test-support")]` constructor that
replaces the language server with recorded answers per path; the real path is the same code after
the answer arrives.

**Done when:** `cargo test -p code-record --features test-support`,
`cargo test -p shadows --test codemap`, `cargo test -p shadows-core --features test-support --test contracts`
and clippy pass. Test 2 fails if the matcher always answers found.

## Task B-171 · Runs runs the `contract` check

**Files:** `crates/core-runs/Cargo.toml`, `src/lib.rs` (field and constructor), `src/checks.rs`,
`src/store/checks.rs`, `src/store/marks.rs`, `contract.yaml`, its tests; every fixture that calls
`Runs::new`; `crates/shadows-core/src/app.rs` (pass `code_record.clone()`);
`crates/core-plans/src/task_contract.rs` and `contract.yaml` (refuse a check named `contract`).

**Steps:**
1. `Runs::new` takes `CodeRecord`.
2. In `confirm_attempt`, after the task's own checks and still under the project's `checking`
   lock: when the attempt's contract has symbols and no finished `contract` row exists, call
   `CodeRecord::verify` and, when its outcome is Some, store a `run_checks` row: `check_id
   'contract'`, `for_task` the attempt's task, `ordinal` after the task's checks, `run_json` `[]`,
   `outcome` passed or failed, `output` `Verdict::lines()`, `head_before`/`head_after` the HEAD,
   `matches` NULL, `reported` NULL. The contract comes from the attempt's stored context
   (`run_attempt_context`), as `specs_in` reads it.
3. The early return in `confirm_attempt` ("every check recorded") also requires the `contract` row
   when one is due, so a restart runs it.
4. `put_mark`'s "done needs a reason" also counts a `contract` row that did not pass.
5. Nothing else: `after_checks`, `returnable_checks`, `notify` and the route read every row.
   The return prompt's text says "checks" and lists the row with `check_line`, so the executor
   reads `contract failed` with its output.
6. Plans refuses a task check whose id is `contract`.

**Tests (with B-170's test-support verdicts, so no language server runs):**
1. `a_missing_symbol_goes_back_to_the_executor`: an attempt whose contract promises
   `refresh_token` and a verdict missing it: a `contract` failed row, and a continued attempt with
   origin Checks whose prompt holds `refresh_token`.
2. `a_found_contract_passes_and_the_notice_names_it`: all found: `contract passed` in the
   Planner's notice; no return.
3. `done_over_a_missing_symbol_needs_a_reason`.
4. `no_language_server_means_no_contract_check`.
5. `a_check_named_contract_is_refused` (Plans).

**Done when:** those tests, `cargo test -p core-runs --features test-support`, the
`shadows-core` contract test and clippy pass.

## Task B-172 · The record, written on done

**Files:** `crates/core-db/migrations/0075_code_record.sql`; `crates/code-record/src/store/`,
`src/record.rs`, `src/model.rs`, `contract.yaml`, `tests/record.rs`; `crates/core-runs/src/lib.rs`
(`mark_done`), `contract.yaml`.

**Steps:**
1. Tables, added to `0075_code_record.sql` (this branch has not shipped it; owned by
   `CodeRecord`, keyed explicitly, no rowid ordering):
   `code_record_file (project_id, workflow_id, version, task, path, action, purpose, commit_sha,
   recorded_at)`, key (workflow_id, version, task, path);
   `code_record_symbol (project_id, workflow_id, version, task, symbol_id, path, name, kind,
   contract_json, state)`, key (workflow_id, version, task, symbol_id);
   `code_record_unpromised (project_id, workflow_id, version, task, path, name)`.
   Removing a project removes its rows (as `delete_code_index_in` does for Codebase's).
2. `CodeRecord::record_done(project, workflow, version, task, contract, commit, attempt)`:
   replaces that task's rows in one write, from the verdict stored under the judged attempt
   (`verdict(attempt)`); with none (no server, or a task done without a committed attempt), the
   symbols are written `unchecked`. Runs passes the judged attempt's id and commit and the
   contract from its stored context.
3. `CodeRecord::forget_done(workflow, version, task)` deletes them.
4. `Runs::mark_done`, after the mark is stored and only when it is not a replay: done → spawn
   `record_done`; not done → spawn `forget_done`, as `recheck_done` is spawned. A failure is logged
   `code_record.failed`.
5. `CodeRecord::task_record(workflow, version, task) -> TaskRecord`: the read slice 2 builds on.

**Tests:** `done_writes_the_record_and_not_done_removes_it`, `a_second_done_replaces_the_rows`,
`a_task_without_a_contract_records_nothing`, `removing_the_project_removes_its_record`.

**Done when:** those tests, `cargo test -p code-record --features test-support`, the core-runs
tests, the codemap and contract tests and clippy pass.

## Task B-173 · Acceptance on a real provider

On a project with a Python language server (pyright), Planner and executors on Sonnet:
1. A plan whose T1 promises `add`, `delete` and `list_all` in `notes/store.py`; the person tells
   the executor's task text to skip `list_all`. Observe: `contract failed` with `missing: list_all`,
   the return to the executor, its second commit, `contract passed`, the done mark, and the
   record's rows.
2. A task touching only a file with no server: no `contract` check row.
Recorded in `docs/evidence/` with dates, commits and log lines.

## Frontend

The Report tab draws every check row it receives; whether a row with no command reads well is
for Masada to check after B-171 (a `F-` task only if it does not).
