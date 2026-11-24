# Shadows backend engineering review

**Reviewed revision** `98d14878` · **Date** 2026-11-25 · **Scope** the Core Rust workspace, including the AYLUL backend services.

## Engineering judgment

Keep the modular monolith. Cargo enforces the direction of crate dependencies, HTTP and MCP both depend directly on `shadows-core`, and project removal shows that separate table owners can participate in one atomic operation without exposing their stores. The main weaknesses I found are in the agreements **between** otherwise well-separated components: what identifies a replay, what makes an instant later than another, and who finishes work after the initiating future disappears.

**Fix findings 1–3 first.** An accepted command can replay a different result, a check can compare execution windows incorrectly, and a valid database path can resolve to a different file. Each has a small deterministic regression case. Next address extension imports, event publication and recheck recovery. All three affect other work while the daemon stays up.

I inspected the current source independently of the branch diff, enumerated production dependencies for all 35 workspace packages, and followed selected public operations through their owners, storage, background work and consumers. Two packages are test programs (`fake-acp` and `fake-codex`). Manual review depth is recorded below. I ran 99 repository tests and three isolated experiments. This report concerns the local backend. The React client, native Tauri workspace, desktop hook, real model providers and deployment were outside its scope. [AppCore service map](../../crates/shadows-core/src/app.rs) · [Workspace code map](../codebase/README.md)

## Correctness and persistence

### 1. High — Recheck command replay can return another command’s batch

**Evidence.** A task completion and an explicit recheck classify the caller’s command ID under separate scopes, `run.mark_done` and `run.recheck`. Both store the bare ID in `run_rechecks.cause_command`, which is globally unique. On replay, `ask_recheck` discards the stored outcome returned by `CommandContext::classify` and looks up a batch by the bare ID. [Command identity](../../crates/core-db/src/command.rs) · [Completion-triggered batch](../../crates/core-runs/src/store/marks.rs) · [Explicit recheck and replay](../../crates/core-runs/src/store/rechecks.rs) · [Schema](../../crates/core-db/migrations/0070_run_rechecks.sql)

**Consequence.** Two legitimately distinct commands with the same ID string can collide. When both create batches, the second insert fails and rolls back its parent transaction, potentially including a task’s done mark. A subtler sequence starts with an explicit request that finds nothing to recheck. Its recorded outcome is empty. If a later completion creates a batch with the same bare ID, replaying the empty request can now return the completion’s batch. I exercised the lookup and uniqueness constraint on the actual recheck table definition in memory. The application-level sequence follows from the two callers.

**Change and acceptance.** Decode the original `outcome_ref` on replay, including its empty value. Migrate the uniqueness rule to the command’s full identity or rely on the atomic command record after checking all consumers. Test an originally empty replay, two productive commands in different scopes, a same-scope fingerprint conflict, and replay against migrated rows.

### 2. High — Text comparison reverses some timestamps inside the same second

**Evidence.** `Storage::now` emits RFC3339 with variable fractional precision. `last_recheck` sorts the text directly, `failed_before` compares the text to select a preceding batch, and `Runs::overlapped` compares operation start and finish strings against a check’s interval. Using the workspace’s compiled Rust `time` library, I formatted instants 20 ms apart as `2026-11-25T12:00:00.1Z` and `2026-11-25T12:00:00.12Z`. In SQLite, the production ordering expression selected the **first** as newest. The trailing `Z` sorts after the next digit. [Timestamp producer](../../crates/core-db/src/lib.rs) · [Batch selection](../../crates/core-runs/src/store/rechecks.rs) · [Overlap decision](../../crates/core-runs/src/checks.rs)

**Consequence.** The plan can show an older recheck. Prior-failure detection can suppress or repeat a notice based on the wrong batch. The overlap predicate can misclassify a shared-folder check when another executor starts or finishes near a boundary. I confirmed the formatting and ordering error directly. The effects on check classification follow from the comparisons.

**Change and acceptance.** Parse timestamps into instants for interval arithmetic. Give persisted batch order a sortable, exact representation, or use a durable sequence if creation order is the intended rule. Handle previously stored variable-width strings. A millisecond-only conversion needs a separate ordering key for submillisecond ties. Test the two values above, an exact second, two submillisecond writes, and operations touching either endpoint of a check.

### 3. High — Opening a database can reinterpret characters in its filesystem path

**Evidence.** `Storage::open` interpolates the configured path into a SQLite URL, converts it with `to_string_lossy`, then parses the URL. In a standalone probe linked to this checkout’s compiled SQLx, a requested path of `E:/data/team%20A/shadows.db` produced a connection filename of `E:/data/team A/shadows.db`. The probe inspected the parsed filename without opening either database. [Connection setup](../../crates/core-db/src/lib.rs)

**Consequence.** A literal percent-encoded-looking directory name is valid on Windows. Depending on which directories exist, the daemon can fail at startup or open, migrate and recover a different database. Lossy conversion introduces a related ambiguity on platforms supporting non-UTF-8 paths.

**Change and acceptance.** Build `SqliteConnectOptions` with its native `filename(db_path)` setter and the existing journal, foreign-key, creation and timeout options. In a temporary tree create both `team%20A` and `team A`, start storage with the former, and assert that only the requested directory receives the database. Include a platform-gated non-UTF-8 case.

### 4. High impact on concurrent use — Extension import holds the sole writer while copying files

**Evidence.** `ExtensionsStore::add` enters `write_txn_as`, then awaits a blocking task that walks, hashes, copies and renames the extension tree. Import accepts up to 64 MiB and 10,000 files. `spawn_blocking` moves the filesystem work off Tokio’s async executor, while the shared `BEGIN IMMEDIATE` writer remains occupied throughout. Cleanup after an error runs in the initiating future. [Import transaction](../../crates/core-extensions/src/store/mod.rs) · [File publication](../../crates/core-extensions/src/files.rs) · [Serialized writer](../../crates/core-db/src/lib.rs)

**Consequence.** Importing on a slow disk can delay unrelated task, message and event writes across projects. Cancellation while waiting for the blocking task can also leave a published directory without a metadata row: the copy can finish after the outer future has been dropped. I traced the lock lifetime and cleanup path; I did not measure production latency.

**Change and acceptance.** Check for replay early, prepare a uniquely named staging directory outside the write transaction, then reclassify the command and check the name inside a short transaction. Assign responsibility for cleanup across cancellation and for reconciliation after process exit, including the gap between rename and commit. The attachment upload path already demonstrates expensive validation before acquiring the writer. Test with a barrier inside copying: an independent write should finish while the copy is paused. Also cancel at publication and retry the same command. [Existing attachment pattern](../../crates/core-threads/src/attachment.rs)

## Background work and live services

### 5. Medium — A cancelled LSP question can leave its document open

`Server::ask` locks the server, sends `didOpen`, awaits the request, then sends `didClose`. Its internal request timeout returns normally through the close path. Dropping the outer future during the await releases the lock and skips the close. The server can retain stale text or see a second open for the same URI. [LSP document lifecycle](../../crates/shadows-lsp/src/client.rs) · [Codebase caller](../../crates/codebase/src/lsp/ask.rs)

**Change and acceptance.** Arm a local close guard immediately after a successful open. Ensure its drop path sends the close before releasing the question lock, and preserve error reporting on ordinary completion. With a recording language server, cancel after it observes the open, ask again with changed text, and assert the open/close order. This finding is based on control flow.

### 6. Medium — A transient recheck error can leave a batch waiting until daemon restart

If `run_recheck` fails before `end_recheck`, its spawned worker logs and exits. The batch may still lack `finished_at`, and live watchers may never receive the end signal. Replaying the accepted command returns the batch with `fresh=false`, so replay does not relaunch it. Startup recovery already knows how to read unfinished rows. [Worker and recovery](../../crates/core-runs/src/recheck.rs) · [Watcher lifetime](../../crates/core-runs/src/recheck_live.rs) · [Durable recovery query](../../crates/core-runs/src/store/rechecks.rs)

**Change and acceptance.** Give Runs ownership of worker completion and bounded, delayed retry for transient failures while the daemon is up. Guarantee a single worker per batch. Distinguish failure before an external check runs from failure to save a completed check, since rerunning an arbitrary command can have effects. Surface a terminal or recoverable diagnostic for permanent errors. Test transient failure without restart, failure after execution, and an already-connected watcher. This is a traced failure path.

### 7. Medium — Cancellation after commit can delay a live event indefinitely

After SQLx commits, `write_txn_as` separately awaits a `MAX(seq)` query, then advances the watch channel. A query failure logs a warning. Cancellation in that interval can also leave the channel behind the journal. A caught-up project subscriber waits for the channel to change. The event remains durable and can be replayed, but an already-connected consumer may wait until another write or reconnect. [Commit and publication](../../crates/core-db/src/lib.rs) · [Project subscriber](../../crates/core-events/src/subscription.rs)

**Change and acceptance.** Make completion of commit and notification the responsibility of work whose lifetime extends across both, or reconcile the last durable sequence when publication fails. Preserve commit ordering and never announce a rolled-back row. Add a barrier between commit and publication, cancel the caller, and require an existing subscriber to receive the event **without any subsequent write**. I identified the cancellation window from source.

### 8. Low today — The ended-turn query selects event kinds its decoder rejects

`ended_turns_after` includes `RecheckStarted` and `RecheckFinished` in SQL, but its decoder only recognizes the four operation endings. Today recheck writers set only the project identifier, so the query’s additional thread and operation predicates exclude them. If a writer later adds those identifiers for provenance, the messages worker can keep retrying a batch that fails at the same cursor. [Query and decoder](../../crates/core-events/src/store.rs) · [Current writer](../../crates/core-runs/src/store/rechecks.rs) · [Cursor consumer](../../crates/core-messages/src/replies.rs)

**Change and acceptance.** Remove the two recheck kinds from the ended-turn query and keep them in the project-stream query. Test a mixed journal containing a recheck with provenance followed by a real ending. The ended-turn reader should reach the real ending. This is a latent contract mismatch, with its current reachability checked against the writer.

## Query cost, service boundaries and reuse

### 9. Medium — The latest recheck summary combines two database snapshots

`RunsStore::last_recheck` fetches the batch header and the outcome counts in two independent pool reads. A final write between reads can pair an old header with newer counts. For example, if a newer batch is created after the header read and the older batch then makes progress, those later counts never belonged to that batch while it was the newest. Zero unfinished rows with an open header can also be a legitimate intermediate state, so that combination alone cannot diagnose the race. [Summary query](../../crates/core-runs/src/store/rechecks.rs)

Use a single reader transaction or one statement for both pieces. `PlansStore::live_versions` already shows the appropriate reader-transaction pattern. This preserves concurrency with the WAL writer and avoids API changes. Verify with coordinated reader and writer transactions, comparing the full returned tuple against the two snapshots. [Existing snapshot pattern](../../crates/core-plans/src/store/live_versions.rs)

### 10. Medium — Checking one project reads other projects’ history under the writer lock

`superseded_in` retrieves the selected plans’ version IDs, then fetches **every** completed mark in `run_task_marks`. It builds a map for this project and discards unrelated rows in Rust. Its caller then reads project-relevant marks again. Both operations happen while building a recheck batch in the serialized write. A small project therefore inherits work from the history of unrelated projects. [Recheck construction and supersession](../../crates/core-runs/src/store/rechecks.rs)

Restrict the candidate marks to the selected workflow IDs at the database, preserving the rule that a newer done mark can supersede an older one even if the newer mark has no judged attempt. Keep Plans responsible for version-to-plan mapping. Compare results across multiple plans, historical versions, marks without attempts and two projects. Measure rows read and time holding the writer as unrelated history grows.

### 11. Medium — Waiting for a quiet project repeatedly loads operation history to answer a boolean

`wait_for_quiet` iterates the executor threads, calls `Turns::operations` for each and scans the returned operations for a missing finish time. It repeats after one second if any thread is busy. `Operations::thread_is_busy` already answers the lifecycle question through an existence query with `LIMIT 1`. Loading history is particularly costly for long-lived executor threads. [Polling loop](../../crates/core-runs/src/checks.rs) · [History API](../../crates/core-turns/src/lib.rs) · [Busy API](../../crates/core-operations/src/lib.rs) · [Existence predicate](../../crates/core-operations/src/store/checks.rs)

Reuse the owner's busy check through an explicit dependency, or give Operations a scoped batch check when the number of threads warrants it. Verify the agreement between terminal status and finish time against pending, stopping, terminal and recovered operations. Keep the before-and-after project-lock checks: they handle different scheduling windows. The separate historical overlap query genuinely needs interval facts.

### 12. Medium — Project state pays for full collections before applying response limits

The state view reads stage, layout, elements and live versions. Plans reads Frozen and Draft heads per active plan, then reads tasks and parts. ProjectState reads progress per live version. For each displayed part it scans the plan and element lists again. Its `bounded` helper collects **all** mapped objects before keeping the first page; in part detail, those discarded objects can already contain fully constructed task lists. The elements-only endpoint also invokes the common facts path, including the stage and layout that it does not return. [Composition and limits](../../crates/project-state/src/state.rs) · [Per-plan reads](../../crates/core-plans/src/store/live_versions.rs) · [HTTP reads](../../crates/shadows-http/src/project_state.rs)

Start with the cheapest changes. Count excess matches before constructing their detailed DTOs. Index progress by task number and accumulate element counts by part in one pass. Separate the facts needed to calculate element states from stage/layout reads; preserve the planned pins and progress that element states actually need. Then measure whether batching plan heads or progress reads inside their respective owners is worthwhile. Validate ordering, omitted counts and multi-part plans using a dataset above the display limits. Avoid a cross-owner SQL shortcut.

### 13. Medium — LSP response limits apply after potentially expensive work

In `Codebase::lsp`, the source file is read in full, converted to an owned string and cloned for the request closure. A call-hierarchy request can issue a follow-up for every prepared item, awaiting each separately and collecting all results before the final 100-item/32-KiB output limit. File questions are already serialized by the server’s question lock. A small response on the wire therefore gives no bound on the amount of work done or on total latency. [Question construction, hierarchy and final bound](../../crates/codebase/src/lsp/ask.rs) · [Server serialization](../../crates/shadows-lsp/src/client.rs)

Apply a shared request deadline and a work budget to hierarchy expansion. Preserve scope filtering **before counting visible results** and report truncation when the budget is reached. For a multi-step question, consider one document session that owns its text and closes safely on cancellation. Validate request count, large-file allocation and ordering with a stub server that returns many prepared roots. A complementary issue exists in contract verification: each distinct file can independently spend up to its own startup retry budget while the project check lock is held. Share server readiness for the batch while retaining per-file verdicts. [Contract verification](../../crates/code-record/src/verify.rs) · [Project check lock](../../crates/core-runs/src/recheck.rs)

### 14. Lower priority — Usage reporting materializes every measurement before aggregation

The store uses the existing `julianday(recorded_at)` expression index to narrow the range, fetches all matching SQL rows, then converts them into a second owned collection for `Usage::aggregate`. The report ultimately keeps totals per provider/model plus the most recent historical provider label. A broad reporting period has memory cost proportional to invocations even if there are only a handful of result groups. [Query](../../crates/core-usage/src/store/mod.rs) · [Aggregation](../../crates/core-usage/src/report.rs) · [Existing index](../../crates/core-db/migrations/0034_aylul_usage.sql)

Fold rows incrementally into the existing accumulators. Preserve exact fractional endpoints, checked integer sums, incomplete cost handling and the latest-label tie-breaker. Keep the index; its expression already matches the filter. Compare output to the current implementation across historical provider changes and measure peak memory on a long reporting period.

## Code structure and maintainability

| Location | Observation | Suggested improvement and verification |
|---|---|---|
| [Architecture dependency tests](../../crates/shadows/tests/architecture/main.rs) | Many tests repeat a text parser for `Cargo.lock` and the same assertion loop. The lockfile flattens dependency kinds, which makes it difficult to express different production and test policies. The executed test list has individual dependency tests for Plans and Turns but no correspondingly named tests for Runs or Messages. | Put package policies in a table and run one checker over Cargo metadata, using workspace membership and dependency kind. Keep service-specific failure messages. Verify one prohibited production edge, a permitted dev-only edge, and that every production owner has an explicit policy. This is a worthwhile loop conversion because the operation is identical and only the data varies. |
| [Design map validation](../../crates/core-design/src/map_rules.rs) | For every part, parent validation can walk its ancestors. Each step uses a linear search for the next parent. In a deep chain the parent-validation phase alone can approach cubic work. | Build a key-to-parent index once and use visitation states to validate ancestry. Preserve deterministic refusal precedence for duplicate keys, missing parents and cycles. Test a deep valid chain and a cycle near its end. |
| [Project subscriptions](../../crates/core-events/src/lib.rs) · [Subscription implementation](../../crates/core-events/src/subscription.rs) | A project subscriber acquires receivers for bus, options, commands, UI signals and tasks, although its live loop reads only the committed-sequence watch. This also causes thread-only senders to retain bounded broadcast history when project subscribers are the only listeners. | Let the scope own the receivers it consumes. Keep the thread rule that receivers must be acquired before journal replay. Verify project-only subscription does not register a thread listener and that concurrent commits during replay still arrive. |
| [Approval provenance](../../crates/core-grants/src/approver.rs) | The public `Approver::command` accepts `Person` at the type level and reaches `unreachable!` for that valid enum variant. Current Planner callers can satisfy the precondition, but a new caller can accidentally cause a panic. | Place derived command construction on a planner-specific value or return a typed refusal. Keep existing fingerprint, principal and command ID derivation. Test person and Planner paths separately. |
| [Daemon configuration](../../crates/shadows/src/config.rs) | `harness_path` accepts an existing directory because it tests `exists()`. The Codex executable validator in the same file already checks `is_file()`. `allowed_origin` also accepts authorities such as `localhost:abc`, which a browser cannot use as a port. | Require a regular file for the executable and parse the origin’s authority with a URL/authority parser while retaining the product’s exact-origin policy. Test a directory, an invalid port, bracketed IPv6, default-port spelling and a valid executable. These are startup diagnostics. |
| [Provider listing](../../crates/core-providers/src/probe.rs) | The listing loop stops after five pages. If page five says there is another page, the successful result is indistinguishable from a complete listing. Repeated provider cursors also use the remaining budget. | Keep the bound and expose clipping if a complete catalog is promised to callers. Track seen cursors within one request. Test exactly five complete pages, a sixth page and a repeated cursor. |
| [Agent home initialization](../../crates/core-agents/src/home.rs) · [Harness home file handling](../../crates/core-harness/src/profile_home.rs) | Seeding creates the final settings filename first and accepts any existing file on retry. A failed or interrupted initial write can leave partial JSON that future seed attempts skip and the harness later refuses to parse. | Publish a fully written temporary file using **no-clobber** semantics, preserving manually supplied settings. The harness’s staged file writing is a useful pattern, but its replace-on-rename semantics are inappropriate for seeding. Test interrupted first creation and preservation of a pre-existing valid file. |

## Service independence and areas worth preserving

| Area inspected | Assessment |
|---|---|
| Composition, binary, HTTP, MCP | `AppCore::start` constructs shared owners and performs recovery before serving new work. The HTTP project-state endpoint and the Planner MCP tool both call `ProjectState::state`. This is a concrete example of one business operation shared by two adapters. The binary additionally depends on the transport composition and process support. [Composition](../../crates/shadows-core/src/app.rs) · [HTTP](../../crates/shadows-http/src/project_state.rs) · [MCP](../../crates/shadows-mcp/src/state_tools.rs) |
| Projects, ProjectLifecycle, Threads, Grants | Removal delegates to Projects, Plans, Codebase, CodeRecord and Grants within one write. Threads owns thread checks and attachment access. Grants verifies the writer, actual harness and originating thread entry inside the approval transaction. Keep these owner methods when adding cross-domain operations. [Removal](../../crates/core-project-lifecycle/src/store.rs) · [Approval](../../crates/core-grants/src/approval.rs) |
| Runtime, Operations, Turns, Harness, Events, Messages | Runtime identity, durable operation lifecycle, live session state and stream delivery have distinct owners. Shutdown checks durable terminal state. Messages deliberately reconciles unlinked starts before consuming terminal events. Preserve that ordering when improving retries. Coupling is concentrated in Turns (15 direct production workspace dependencies), Harness (14) and Runs (12). Those counts identify expensive change points; the call paths in findings 5–11 explain where to act. [Shutdown](../../crates/core-turns/src/shutdown.rs) · [Dispatch and cursor handling](../../crates/core-messages/src/replies.rs) |
| Design, Plans, Standards, Instructions, ProjectState | Plans uses `petgraph` for graph analysis and supplies product-specific task/phase ordering. Standards and Instructions retain their versioned stores. ProjectState has no store and composes the owners’ reads. The opportunities here are query shape and deterministic validation. The existing phase and cross-plan tests check important differences between unrelated cycles, stale revisions and dependencies. [Dependency graph](../../crates/core-plans/src/dependency_graph.rs) · [ProjectState dependencies](../../crates/project-state/src/lib.rs) |
| Agents, Providers, Secrets, Extensions, Person, Usage | The agent/provider update uses owner methods inside one transaction to protect model references. Agents already reuses the provider slug generator. Secrets derives separate encryption and fingerprint keys with existing libraries. Person settings depend only on the database foundation. Usage preserves measured historical labels. File publication and broad report reads deserve more attention than factoring these services into a generic settings store. [Agent/provider operation](../../crates/core-agents/src/lib.rs) · [Slug reuse](../../crates/core-agents/src/store/mod.rs) · [Key derivation](../../crates/core-secrets/src/crypto.rs) |
| Codebase, CodeRecord, index, LSP, agent and process infrastructure | CodeRecord asks Codebase for symbols; the index takes source text without a database dependency; LSP owns the protocol conversation; the process package owns OS execution. The suspended name index is explicitly disabled at startup. Preserve these boundaries when adding limits and cancellation handling. The review followed file questions and contract verification more deeply than individual adapter message variants or process backends. [Codebase contract](../../crates/codebase/contract.yaml) · [Tag extraction](../../crates/shadows-index/src/extract.rs) · [Agent connection types](../../crates/shadows-agent/src/connection.rs) · [Process API](../../crates/shadows-process/src/lib.rs) |

The repository’s architecture guard is valuable precisely because SQLite permissions are shared across services. Its current ownership allowlist includes explicit test fixtures and reviewed SQL cases. A passing guard supports the structural judgment; it cannot establish correctness across cancellation, chronological ordering or every generated SQL path. [Guard coverage and detection limits](../../crates/shadows/tests/architecture/README.md) · [Ownership exceptions](../../crates/shadows/tests/architecture/ownership.allowlist)

## Verification record

| Executed on this revision | Result |
|---|---:|
| Architecture tests, including the repository scan | 51 passed |
| Code map coverage | 1 passed |
| Service contracts | 6 passed |
| AYLUL usage | 6 passed |
| Cross-plan resolution | 4 passed |
| Design elements | 14 passed |
| Operation lifecycle | 12 passed |
| Phase dependencies | 3 passed |
| Assistant project lifecycle | 2 passed |
| **Total** | **99 passed, 0 failed** |

Commands used from the repository root:

```powershell
cargo test --offline -p shadows --test architecture --test codemap -p shadows-core --test contracts
cargo test --offline -p shadows-core --test project_assistant --test aylul_usage --test design_elements --test phase_dependencies --test operation_lifecycle --test cross_plan_resolution
```

The isolated probes covered the recheck schema and replay lookup, Rust timestamp formatting plus SQLite ordering, and SQLx filename parsing. The table above counts repository tests only. I did not run the full workspace gate, benchmark production load, simulate every cancellation window, or perform an end-to-end provider test. The proposed acceptance checks are regression cases for future fixes. The review changed only this report.
