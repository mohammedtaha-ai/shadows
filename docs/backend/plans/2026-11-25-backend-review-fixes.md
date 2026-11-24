# Backend review fixes implementation plan

**Goal.** Repair the observable failures in the [backend review](../../evidence/2026-11-25-backend-review.md), then reduce unnecessary work in the same production paths. This is a proposed implementation sequence for the code reviewed at `98d14878`. The numbered tasks correspond to the review's 14 findings and then its seven maintainability rows, in order.

**Execution order.** First **2 → 8 → 3 → 6 → 7 → 1 → 9 → 5**: the wrong check verdicts, the stuck batch and the missed live event, which the web now depends on since it follows the stream. Then **13 → 4 → 11 → 10**. Then **12 → 17 → 18 → 19 → 20 → 21**. Tasks **14, 15 and 16** are deferred to [future](../../core/future.md) (see each). Task 13 builds on task 5. Changes to the same store run serially. Each task should leave a usable backend and a separately reviewable commit.

**Simplest checkpoint first.** Each fix below takes the one simplest point that removes the observed failure and names the risk it accepts. Heavier machinery (retry loops, reconciliation workers, schema rebuilds) is added only when evidence shows the simple fix is not enough.

**Into stages.** Builders take work from [`stages.md`](../stages.md). Each task here becomes one `B-<n>` entry there (B-211 onward, in the execution order above) before it is built; this plan keeps the reasoning and the tests.

**Implementation boundaries.** Start at `crates/shadows-core/src/app.rs`, then the owner's contract. Paths below are repository-relative. Amend the owning contract with each changed obligation. Add a code-map row for a new module. Keep HTTP and MCP on the same Core operation. For a change visible over the wire, settle the corresponding shared request and regenerate the schema in that task. Task numbers here are plan-local.

**Test decisions.** “Extend” means change the named existing test or its data table. “Keep” means run the existing behavioral proof as written. “Add” identifies a distinct failure that the current suite does not exercise. Use actual SQLite migrations and commits, temp directories, Git, OS child processes and loopback sockets as appropriate. A controlled peer or barrier may determine *when* a real operation proceeds. The assertion must observe the production result (file, persisted row, response, event or child protocol). Do not assert helper call counts in place of behavior, or add timing thresholds to CI to prove a speedup. For a correctness regression, run its case on the old code and record the failing assertion, implement the change, then run the entire named target. For a behavior-preserving optimization, collect before/after evidence on the same input and keep existing semantics tests green.

## Durable commands, time and storage

### 1. Recheck replay and cause identity

- [ ] **Change** `crates/core-runs/src/store/rechecks.rs` (`ask_recheck`, `begin_recheck_in`) and verify `store/marks.rs::put_mark`. Decode the stored outcome from `CommandContext::classify`: empty string maps to `None`, a stored batch ID to `Some(id)`, both with `fresh=false`. Keep fingerprint mismatch propagation and perform classification before any writes. Both trigger paths already classify and record their command in the same write as batch creation.
- [ ] **No schema change.** `run_rechecks.cause_command` keeps its global `UNIQUE`. A collision needs one command ID used for both a done mark and an ask: the web sends `crypto.randomUUID()`, and a Planner's ID is derived from its turn and the command's kind and fingerprint (`core-grants/src/approver.rs`), so the two kinds never share one. **Accepted risk:** an outside client reusing an ID across the two gets a refused write, not a wrong answer, once replay reads the stored outcome.
- [ ] **Extend** `crates/shadows-core/tests/runs_attempts/rechecks.rs::a_replayed_ask_starts_no_second_recheck` to assert the original ID and a stable batch count. **Add** the case the decoding fixes: an ask that found nothing to recheck, a later batch begun under the same ID string by a done mark, then a replay of the ask returns none.
- [ ] **Run** `cargo test -p shadows-core --test runs_attempts rechecks::`.

### 2. Correct fractional-second ordering and overlap

The cause is one line: `Storage::now` (`crates/core-db/src/lib.rs`) formats RFC3339 with trailing zeros trimmed, so the fraction has 0 to 9 digits and text order is not time order. Besides the three sites the review names, at least 14 other `ORDER BY …_at` reads sort such text (agents, providers, attempts, conflicts, lessons, the recheck notice queue). Fix the writer, not each reader.

- [ ] **Change** `Storage::now` to always write nine fractional digits (`YYYY-MM-DDTHH:MM:SS.fffffffffZ`, fixed width, UTC), through one `Storage` formatter. Route every Core writer that formats its own database time through it (for example `core-grants/src/store/mod.rs`); a query bound that is compared through `julianday` (usage periods) does not need it. Every new row then sorts by text in time order at every existing reader, with no migration and no generated key.
- [ ] **Change** `crates/core-runs/src/checks.rs::overlapped` to parse both boundaries into `OffsetDateTime` and compare instants, keeping the open-interval and absent-start/finish rules. It reads operation times written before this change, so it must not rely on width. A time that does not parse is an error, never a pass.
- [ ] **Accepted risk.** Rows written before the change keep their trimmed spelling (and migrations 0022/0074 wrote three digits, 0068 none). Only two such rows within the same second can still sort wrongly; no backfill.
- [ ] **Add** a `storage_contract` case that `Storage::now` has the fixed width across several calls, including one that lands on a whole second if reachable through the formatter. **Extend** the overlap fixture in `runs_attempts/checks.rs` with adjacent and strictly overlapping windows written in both old widths (`.1Z` against `.12Z`, a whole second against `.5Z`); assert the accepted or retried result. Run each on the old code first and record the failing assertion.
- [ ] **Run** `cargo test -p core-db --test storage_contract` and `cargo test -p shadows-core --test runs_attempts`.

### 3. Open the literal database filename

- [ ] In `crates/core-db/src/lib.rs::Storage::open`, replace URL construction and parsing with `SqliteConnectOptions::new().filename(db_path)`. Preserve WAL, foreign keys, create-if-missing and busy timeout for both connections. Remove the unused parsing import.
- [ ] **Extend** `crates/core-db/tests/storage_contract/schema.rs::fresh_database_migrates_and_applies_the_connection_policy`: make sibling directories named `team%20A` and `team A`; run the existing migration and policy assertions on the former, close and reopen, and assert the latter has no DB file. On Unix exercise an actual non-UTF-8 path using the platform path API and check SQLx's result. If the pinned driver rejects it, require a clear error and no altered-path creation; do not claim support the driver cannot provide.
- [ ] **Run** `cargo test -p core-db --test storage_contract`. This test must open an actual filename; inspecting parsed options alone does not prove the fix.

## Cancellation, worker recovery and subscription consistency

### 4. Move extension copying outside the SQLite writer

- [ ] Split `ExtensionsStore::add` in `crates/core-extensions/src/store/mod.rs` and publication in `src/files.rs` in two. **Before** the writer: check replay with a read (so a removed source still replays), then copy, hash and validate into a staging directory on the library volume named with a fixed prefix (for example `.staging-<uuid>`), using the existing traversal, size and digest rules. **Inside** one short writer transaction: classify again, check the name, rename the staged directory to its id, insert the row and record the result. On an error, remove only this import's directories.
- [ ] At startup, before imports are admitted, delete leftover `.staging-*` directories. **Accepted risk:** a crash between the rename and the commit leaves an id directory with no row; it holds no row's data and is not read. Nothing outlives a dropped caller except a staging directory the next start removes.
- [ ] **Keep** `extension_commands_replay_and_conflict`, `extension_import_copies_and_redacts_source` and the bundled-restart cases in `crates/core-extensions/tests/library.rs`. **Add** a concurrency case with a real temp tree and a barrier during the copy: an independent real DB write commits before the barrier releases. **Add** a leftover `.staging-*` directory before start and assert it is gone and published items are intact.
- [ ] **Run** `cargo test -p core-extensions --test library` and `cargo test -p shadows-core --test aylul_extensions`.

### 5. Close an LSP document when the calling future is dropped

- [ ] In `crates/shadows-lsp/src/client.rs::Server::ask`, acquire the question lock, send `didOpen`, then arm a private document guard holding the socket and URI. Give it a synchronous close operation using the existing notification API. Disarm after the normal close attempt. On drop send a best-effort close while the lock is still owned. Preserve the original operation error and ordinary close failure semantics. Never close a document whose open failed.
- [ ] **Add** a cancellation case in `crates/shadows-lsp/tests/requests.rs`. Reuse the existing real Node stdio process pattern. Have the peer acknowledge that it observed open and is holding the first request. Abort that caller, issue a second question for the same URI with changed text, and assert from the peer's protocol transcript that close preceded the second open and the second answer used the new text. The existing server-initiated-request case covers a different behavior and stays intact.
- [ ] **Run** `cargo test -p shadows-lsp --test requests` and the existing timeout case `cargo test -p shadows --test code_lsp a_slow_answer_times_out`.

### 6. Recover an interrupted recheck worker while Core is running

- [ ] **The batch always ends.** In `crates/core-runs/src/recheck.rs`, when `run_recheck` returns an error (fresh spawn or startup recovery), finish the batch in one write: each row with no `finished_at` gets `not_run` with the error as its output, `finished_at` is set, and `RecheckFinished` is journaled as `finish_batch` does today. Then end the live watchers (`recheck_live`). The plan page leaves "Rechecking…" and the panel ends.
- [ ] If that finishing write itself fails (storage down), still end the live watchers and log; the batch stays unfinished and startup recovery runs it again, as today. **Accepted risk:** a transient error ends the batch as `not_run` rows instead of retrying; the person presses Recheck again. A retry loop is added only if evidence shows transient errors recur.
- [ ] **Keep** `a_restart_runs_an_interrupted_recheck_again` and `a_finished_batch_never_loses_its_notice`. **Add** a same-process failure case in the recheck suite: make one row fail mid-batch with a real cause (for example its attempt's folder removed so the check cannot run, or a SQLite trigger in the fixture database that refuses that row's result write); assert the batch has `finished_at`, the remaining rows read `not_run` with the reason, `RecheckFinished` is in the journal, and a connected watcher received its end.
- [ ] **Run** `cargo test -p shadows-core --test runs_attempts rechecks::`.

### 7. Reconcile event publication after commit cancellation

- [ ] In `crates/core-db/src/lib.rs::write_txn_as`, keep the borrowed transaction API for the caller's closure. Hold the writer through an owned guard (`Arc<Mutex<…>>` with `lock_owned`) so that, once the closure returns `Ok`, the commit **and** `publish_committed` run together in one `tokio::spawn`ed task that the caller only awaits. A dropped caller then cannot stop between commit and publication; the next writer still waits for that task's guard, so commit order is kept. A failed `MAX(seq)` read after a commit is retried once, then logged. No reconciliation worker. **Accepted risk:** if that read fails twice, a caught-up subscriber sees the event at the next write.
- [ ] **Keep** atomic commit/rollback and panic-recovery tests in `crates/core-db/tests/storage_contract/txn.rs`. **Add** a deterministic cancellation case alongside journal tests: subscribe, start a real journal write, pause after SQLite commit but before publication, abort the caller, and observe the exact sequence on the existing receiver without a second write. Exercise rollback and ensure its event cannot be replayed. A narrow test-only barrier is justified here because an ordinary race cannot establish which side of commit was cancelled.
- [ ] **Run** `cargo test -p core-db --test storage_contract` and `cargo test -p shadows --test project_events`.

### 8. Restrict the ended-turn cursor to actual operation endings

- [ ] In `crates/core-events/src/store.rs::ended_turns_after`, remove the two recheck kinds from that query only. Keep project-stream projection intact.
- [ ] **Extend** `ended_turns_come_back_in_seq_order_after_the_cursor` in `crates/core-events/src/store_tests.rs`. Its existing fixture writes real journal entries with thread and operation provenance. Interleave both recheck kinds between two legitimate endings. Assert only the endings return, in order, and resume after the first cursor. **Keep** the existing recheck project-stream test as the opposite-side proof.
- [ ] **Run** `cargo test -p core-events --lib ended_turns` and `cargo test -p shadows-core --test runs_attempts an_asked_recheck_starts_and_finishes_on_the_project_stream`.

### 9. Read the newest recheck and its counts from one snapshot

- [ ] In `crates/core-runs/src/store/rechecks.rs::last_recheck`, begin one WAL reader transaction, select the batch with today's ordering (`created_at DESC, id DESC`, correct for new rows after task 2), read grouped counts through that same transaction, and release it before returning. Preserve empty-project, nullable outcome and finished-row counting behavior. Follow the existing `PlansStore::live_versions` reader pattern. The doc comment there already says "in one read"; this makes it true.
- [ ] **Extend** `the_plan_page_reads_the_projects_last_recheck`. Use a test-only coordination point between the two reads if necessary. Start reading batch A, commit a newer B and later progress on A from the writer, then finish reading. The first answer must contain A with its original counts and the next call must see B. A fixture that only checks that zero pending rows may coexist with an open batch cannot prove snapshot consistency.
- [ ] **Run** `cargo test -p shadows-core --test runs_attempts the_plan_page_reads_the_projects_last_recheck`.


## Query cost and bounded work

### 10. Restrict supersession reads to the selected project

- [ ] In `crates/core-runs/src/store/rechecks.rs::superseded_in`, retain the workflow-to-plan map from `Plans::version_ids_in`. Bind its workflow IDs to a Runs-owned query, using one JSON array and SQLite's existing `json_each` pattern or bounded groups of parameters. Return immediately for an empty set. Select only done marks for those workflows. Keep the newest mark per (plan, task) before filtering to eligible attempts. A newer mark with no attempt can still supersede an older mark. Keep the work in the batch-building transaction.
- [ ] **Keep** `an_older_versions_done_task_is_rechecked` and `a_done_task_a_newer_version_marks_done_is_not_rechecked`. Extend the latter with an unrelated project's same-numbered task if its fixture lacks that distinction. Use a populated scratch DB and `EXPLAIN QUERY PLAN` to verify a workflow-key lookup; filtering after a full mark-table scan misses the performance objective. Check existing indexes before adding one.
- [ ] **Run** `cargo test -p shadows-core --test runs_attempts rechecks::`. Record the selected plan and rows returned as unrelated history grows; keep the experiment outside the permanent suite.

### 11. Reuse Operations’ busy predicate in quiet-project checks

- [ ] In `crates/core-runs/src/checks.rs::wait_for_quiet`, replace history materialization with `Operations::thread_is_busy(&ThreadId)`. `core-runs/Cargo.toml` already depends on `core-operations`, but `Inner` does not hold it: pass the shared Operations instance from `AppCore` into Runs' constructor (no new crate edge). Confirm every state with no finish timestamp, including pending and stopping, is covered by the owner's open-state rule. Keep the second quiet check after acquiring the project lock. Historical overlap lookup has different semantics and remains separate.
- [ ] **Keep** `checks_wait_for_a_quiet_project`, `quiet_is_checked_again_under_the_lock` and `checks_of_one_project_run_one_at_a_time` in `runs_attempts/checks.rs`. Extend the first only if its state matrix misses a supported open state. Measure the SQL issued for a thread with many finished turns and one open turn; a pass should return at most one match per checked thread.
- [ ] **Run** `cargo test -p shadows-core --test runs_attempts checks::` and `cargo test -p shadows-core --test operation_lifecycle`.

### 12. Build only the project-state data required by the caller

- [ ] In `crates/project-state/src/state.rs`, separate element-state inputs (elements, live pins, progress) from the full view’s stage/layout. Preserve errors for unknown or removed projects. Index progress by task once per version. Accumulate state counts and plan membership by part in one pass, preserving source order for output. Change the limiting operation to count every eligible match but construct DTOs only for the first N; preserve distinct membership and omitted counts. Keep an explicitly requested part available even if it falls outside the summary page.
- [ ] Measure the public state read with multiple plans and more items than a page. If per-plan SQLite reads dominate after these changes, batch the draft/frozen heads inside `crates/core-plans/src/store/live_versions.rs` under its existing snapshot. Group progress inside Runs if measurement warrants it. Retain a draft-only plan and the newest frozen head correctly. Keep all SQL inside the respective owners.
- [ ] **Extend** `the_state_is_bounded` in `runs_attempts/project_state.rs` to verify omitted counts and selected detail beyond the summary limit. **Keep** `a_part_asked_in_full_lists_its_plans_tasks_and_elements`, `a_draft_adds_nothing_to_the_state` and the existing route suite. Compare the actual response and measured query/allocation profile on the same populated project before and after.
- [ ] **Run** `cargo test -p shadows-core --test runs_attempts project_state::` and `cargo test -p shadows --test project_state_routes`.

### 13. Bound hierarchy expansion and share language-server readiness waits

- [ ] After task 5, update `crates/codebase/src/lsp/ask.rs` and `crates/shadows-lsp/src/client.rs`. Keep the document open for a multi-step hierarchy question under one cancellation-safe session. Move file text into that session once. Use a single deadline across queue acquisition, prepare and follow-ups; pass the remaining duration to each call. Proposed work cap: at most 100 prepared-root follow-ups, with the current 100-visible-item and 32-KiB result bounds. Track whether any work was skipped. Apply scope filtering before counting visible answers. Keep timeout/error reporting distinguishable from successful partial output.
- [ ] This part comes first within the task: in the 2026-11-24 live rechecks each `contract` row waited on a server that pyright's absence never started, while the batch held the project check lock. In `crates/code-record/src/verify.rs`, share the **startup** wait budget among paths using the same starting server in one verification. Retain a separate per-file existence query and verdict; if the server becomes ready, still ask it for each eligible file. Use a deadline rather than restarting the two-minute allowance at each path. Preserve multi-language independence.
- [ ] **Keep** real-language cases in `crates/shadows/tests/code_lsp.rs` for ordinary results and scope. **Add** one process-backed hierarchy case producing more roots than the cap, with early out-of-scope locations and a later visible one. Observe actual protocol requests, bounded output, `truncated`, and balanced open/close. The existing slow-answer case covers one RPC, so extend deadline coverage to lock wait and follow-ups. For readiness, extend verification coverage with multiple files on one late-starting peer and a second healthy server; assert actual per-file verdicts and a single startup window. Use a controlled clock for waits where possible, retaining actual requests.
- [ ] **Run** `cargo test -p shadows --test code_lsp --test code_scope`, `cargo test -p shadows-lsp --test requests` and `cargo test -p code-record --test verify`. Run installed-language acceptance for the languages affected.

### 14. Fold usage rows as SQLite returns them

**Deferred** to [future](../../core/future.md): a usage report's rows are few today, and the change has no observed failure. The steps below stay as the design for when a long period is slow.

- [ ] In `crates/core-usage/src/{lib,report}.rs` and `src/store/mod.rs`, replace `fetch_all → Vec<UsageRow> → aggregate` with a SQLx row stream and a private accumulator. The store decodes one measurement at a time; the report accumulator owns `push(row)` and final sorting. Keep the existing indexable candidate predicate, exact half-open period check, historical metadata tie-breaker, incomplete cost rule and checked arithmetic. Propagate errors encountered partway and release the reader on cancellation.
- [ ] **Keep** `reports_keep_exact_endpoints_cache_counts_and_historical_metadata` and `a_partial_cost_group_is_unknown_and_a_refused_write_rolls_back` in `crates/core-usage/tests/report.rs`. Existing public-API and integration tests are sufficient for unchanged behavior. Run a one-off broad-period comparison with many operations but few groups, comparing output and peak resident memory against the baseline.
- [ ] **Run** `cargo test -p core-usage --test report` and `cargo test -p shadows-core --test aylul_usage`.

## Maintainability findings from the review table

### 15. Make architecture dependency checks data-driven

**Now, small:** add `extracted_runs_depends_only_on_declared_lower_owners` and `extracted_messages_depends_only_on_declared_lower_owners` in the existing style, with their allowed owners reviewed against AppCore and the contracts. **Deferred** to [future](../../core/future.md): the table-driven rewrite over `cargo metadata` below.

- [ ] In `crates/shadows/tests/architecture/main.rs`, replace the repeated lockfile parsers with one policy table and checker. Obtain workspace metadata once with `cargo metadata --offline --no-deps --format-version 1`; identify internal packages by workspace manifest/path, handle renamed dependencies, and distinguish normal, build and dev edges. Keep explicit allowed production owners for each service, including Runs and Messages. Review their allowed dependencies against AppCore and contracts. A newly discovered edge is reviewed rather than automatically allowed. Keep existing ownership, SQL and constructor scanners.
- [ ] **Modify** the existing dependency tests into one table-driven test over the **real workspace**. Extend architecture fixtures with a tiny Cargo workspace only where necessary to prove that a forbidden production edge fails, a permitted dev-only edge passes, and a missing owner policy fails with both package names. Use actual Cargo manifests and the same metadata reader.
- [ ] **Run** `cargo test -p shadows --test architecture`. Compare the set of dated exceptions explicitly; migrating the checker must not expand unrelated allowlists.

### 16. Index design-map parents during validation

**Deferred** to [future](../../core/future.md): a project map has tens of parts, so the walk costs nothing measurable today.

- [ ] In `crates/core-design/src/map_rules.rs`, build an index of keys/parents and compute missing parents and cycle membership with an iterative visiting/visited traversal. Then emit refusals in the same input order and at the same stage as the present checks. For duplicate keys, preserve the original lookup/diagnostic precedence. Reuse the parent index in later ancestry checks where equivalent. Keep path, waiver and folder rules separate.
- [ ] **Extend** `a_map_change_into_its_own_descendant_is_refused` in `crates/shadows-core/tests/design_parts/map_proposals.rs` with a deeper valid chain followed by a cyclic move; verify refusal and unchanged approved map. Keep the existing move-to-top/current-parent cases. Use a local deep hierarchy to compare runtime without committing a timing assertion.
- [ ] **Run** `cargo test -p shadows-core --test design_parts`.

### 17. Subscribe only to channels used by the stream scope

- [ ] In `crates/core-events/src/{lib,subscription}.rs`, keep the committed sequence receiver for both scopes, move the thread-only receiver bundle (bus, options, commands, tasks and UI signals) into thread subscription state. Construct the appropriate variant at subscription time. Preserve the ordering “acquire receivers, replay journal, announce caught-up, read live”. Drop the dummy thread receiver construction and scope-dependent unwraps.
- [ ] **Keep** the actual HTTP replay/resume/isolation coverage in `crates/shadows/tests/project_events.rs`. At the owner boundary add an observation only if existing tests cannot show the resource regression: opening just a project subscription must leave a real thread broadcast sender with zero receivers; opening/dropping a thread subscription must increase/decrease it. Exercise a commit during replay and confirm delivery once. A private module test can inspect the channels without adding a public diagnostic.
- [ ] **Run** `cargo test -p core-events` and `cargo test -p shadows --test project_events`.

### 18. Make Planner-only command construction type-safe

- [ ] In `crates/core-grants/src/approver.rs`, move the writer, turn and harness fields into a `PlannerApprover` value; give that value the command constructor. Represent the Planner branch of `Approver` with that value, retain the person branch. Update `approval.rs` and use LSP references to update the actual construction and match sites in owner crates. Preserve principal, audit agent, fingerprint, derived ID and authorization checks. Person commands continue through their existing command-ID path.
- [ ] **Keep** the real MCP approval and replay cases in `crates/shadows/tests/workspace_mcp/approval.rs`, including `the_planner_refuses_waiver_maps_but_the_person_approves`. The compiler now excludes the invalid call; a new panic or source-text test would add no behavioral proof. Extend the replay case only if it does not assert stable stored provenance for the same approval.
- [ ] **Run** `cargo test -p shadows --test workspace_mcp` and the workspace compile checks to catch downstream constructors.

### 19. Validate executable paths and browser origins at startup

- [ ] In `crates/shadows/src/config.rs`, require `harness_path` to name an existing absolute file. Keep platform executable conventions already specified for each harness. For allowed origins, preserve the lexical checks for exact scheme/authority spelling, parse host and port through an existing URL/authority parser, and reject invalid ports, credentials, path, query and fragment. Preserve the established matching policy for valid spelling, including IPv6. Return the existing configuration errors with a useful path-specific diagnostic for directories.
- [ ] **Extend** the harness-path acceptance cases in `crates/shadows/tests/harness_config.rs` with an actual temp directory and file. Add cases to `an_allowed_origin_is_exactly_scheme_host_and_port` in `crates/shadows/tests/cors.rs` for alphabetic/out-of-range port, IPv6 and explicit default port. Keep the actual CORS request/preflight tests. Validate emitted allowed-origin text against the existing contract before changing normalization.
- [ ] **Run** `cargo test -p shadows --test harness_config --test cors`.

### 20. Distinguish a clipped catalog from a complete provider response

- [ ] In `crates/core-providers/src/probe.rs`, retain the five-page ceiling, sorting, deduplication, timeout and response byte budget. Track returned cursors for the current probe. Stop before reissuing a request for an already seen cursor. The fifth page with no continuation is complete; the fifth with a continuation or a repeated cursor is partial. Proposed wire decision: keep the models already found and add an explicit `truncated` boolean to the probe outcome, false on complete/refused/unlisted outcomes. Document it in the provider contract and agree the shared API change with Masada (the web reads this outcome) before shipping; keep existing `ok` and `listed` semantics. If the owner explicitly defines the catalog as a sample instead, record that decision before changing the response type.
- [ ] **Extend** `anthropic_pagination_stops_at_five_pages` in `crates/shadows-core/tests/aylul_profiles/probe.rs`. Its real loopback HTTP peer already serves five pages. Parameterize three responses: exactly complete at five, continuation after five, and repeated cursor. Assert model IDs, clipping signal, and actual requests received by the peer. Keep the existing 404, oversized response, refused key and redirect cases. No provider credentials are required for these protocol cases.
- [ ] **Run** `cargo test -p shadows-core --test aylul_profiles probe::`, regenerate/review OpenAPI and generated consumer types under the shared request, and verify the consumers distinguish partial results where they present completeness.

### 21. Publish complete agent home settings without replacing a user's file

- [ ] In `crates/core-agents/src/home.rs::seed_home`, write bundled settings to a randomly named temporary file in the destination directory, flush/sync, then publish with no-clobber semantics using `std` only: `std::fs::hard_link(temp, settings.json)` fails when the destination exists (NTFS and Unix), then remove the temporary file. `tempfile` is only a dev-dependency of `core-agents`; adding it to production would need its own justification. Only a destination that already exists is a successful concurrent seed; propagate all other errors. Clean the temporary file on failure. Verify filesystem behavior on Windows. Existing user-supplied files retain their exact bytes, including invalid JSON; repair of previously corrupted operator files is a separate decision.
- [ ] **Add** a filesystem case in `crates/core-agents/tests/agents.rs`: synchronize a writer partway through private temp preparation; the final path must still be absent. Exercise two seeders and a pre-existing custom file, then read the final bytes and parse successfully for newly seeded homes. Exercise a failed publication and retry. Keep existing home-instruction import tests; those cover a different file.
- [ ] **Run** `cargo test -p core-agents --test agents` on Windows and a Unix platform.

## Acceptance and delivery

For each commit record the owner, exact tests run, result, and any one-off measurement. Keep a new regression in the closest existing suite unless a genuinely distinct integration boundary requires another target. Use disposable data for crash, migration and concurrency checks. The full gate never runs locally: CI runs it on the PR. Review the whole branch once before the PR opens. For live acceptance use an isolated project to exercise task completion and recheck replay, interrupt/reconnect a live watcher, import an extension during an unrelated write, and repeat an LSP question after cancellation. Record the observed rows/files/events. Report native and real-provider acceptance separately from deterministic local tests.

The [review](../../evidence/2026-11-25-backend-review.md) remains the evidence for the original findings. This plan proposes implementation choices; in particular the hierarchy work cap, permanent-worker failure reporting and the provider clipping field need confirmation against their owner contracts during implementation.
