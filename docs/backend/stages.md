# Core backend stages

This file tracks the existing Shadows Core backend only. See
[the documentation map](../README.md) for AYLUL and independent service work.

## Now

Branch `context-meter`: the architecture restructuring
([plan](plans/2026-10-21-architecture.md)). The rules are the root
`AGENTS.md` "Code structure", and its strict rule binds every task: start at
`AppCore`, work only in the owning service. Task IDs are stable labels, not an
extraction order. Follow the plan's delivery batches and each task's `Needs:`.
Each task preserves behavior and removes resolved violations from the A-0
allowlists. Live keys: only `OPENROUTER` and `API_KEY_MINIMAX`, read with a
loop, never `. ./.env`, never printed or committed.

The settings dialog shipped in PR #40 (`1a2b601`). The context-meter probe
(B-1) is in Git at `e0478f7`; its corrections are A-7.

Before dispatch the controller expands each task's named scope into exact caller,
test, contract and map paths and records the affected test commands. A blocked
task is not a builder assignment. Per-task checks are focused; the full gate and
whole-branch review run once before each delivery PR. Source-contract examples
are reference material; they do not replace the project's template or code.

## Tasks

### A-0 · The architecture ratchet test
- Status: done
- Review: focused independent review repaired construction coverage, extracted-store ownership and SQL classification; regression fixtures use temporary trees.
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-0--the-architecture-ratchet-test
- Read: root `AGENTS.md` "Code structure"; AppCore; the plan's A-0 and audit table; owner spec §14.12; docs/codebase/README.md; crates/shadows/Cargo.toml; service contracts reached from AppCore
- Touch: crates/shadows/tests/architecture/ (new test, fixtures, table owners and baseline files), crates/shadows/Cargo.toml (existing workspace parser dev-dependencies only), Cargo.lock (generated), docs/codebase/README.md (test ownership entry), docs/backend/stages.md (this task), docs/backend/plans/2026-10-21-architecture.md (A-0 clarification only)
- Needs: none; controller scoped this task at 41738fe. A-1's missing service map is an explicit baseline violation, not a reason to skip map checks. Do not implement A-1 here.
- Done when: `cargo test -p shadows --test architecture` and `cargo test -p shadows --test codemap` pass; fixture mutations prove each rule rejects a new violation and stale allowlist entries fail; fmt and focused architecture clippy pass; report detection limits explicitly

### A-1 · `AppCore` is the map
- Status: done
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-1--appcore-is-the-map
- Read: root `AGENTS.md` "Code structure"; crates/shadows-core/src/app.rs
- Touch: crates/shadows-core/src/app.rs, docs/codebase/README.md, crates/shadows/tests/architecture/, crates/shadows/tests/codemap/
- Done when: `cargo test -p shadows --test architecture` and `--test codemap` pass

### A-2 · `Turns` owns what a turn sends; `/` commands go alone
- Status: done
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-2--turns-owns-what-a-turn-sends
- Read: turns/contract.yaml, harness/contract.yaml
- Touch: crates/shadows-core/src/turns/ (mod.rs, spawn.rs, steer.rs, new prompt.rs), crates/shadows-core/src/harness/setup.rs, both contracts, their tests, crates/shadows/tests/planner_mcp.rs
- Done when: `a_slash_command_is_sent_as_one_block`, `context_skipped_by_a_command_reaches_the_next_turn` and the planner context tests pass

### A-3 · The AYLUL window follows the current model
- Status: todo
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-3--the-aylul-window-follows-the-current-model
- Read: harness/contract.yaml
- Touch: crates/shadows-core/src/harness/ (assistant.rs, live.rs, settings.rs), harness/contract.yaml, its tests
- Done when: `switching_model_uses_the_new_models_window` and the agreement test pass

### A-4 · One error mapping for HTTP and MCP
- Status: todo
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-4--one-error-mapping-for-http-and-mcp
- Read: crates/shadows-core/src/error.rs
- Touch: crates/shadows-core/src/error.rs, crates/shadows-http/src/failure/, crates/shadows-mcp/src/refusal.rs, their tests
- Done when: the table test over every `CoreError` variant passes through both adapters

### A-5 · ACP connection safety
- Status: todo
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-5--acp-connection-safety
- Read: crates/shadows-agent/src/acp.rs
- Touch: crates/shadows-agent/src/acp.rs, crates/shadows-agent/src/usage.rs, their tests
- Done when: `a_dropped_prompt_frees_the_session` passes; stderr is logged by length only

### A-6 · The Planner approve tools match the spec
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-6--the-planner-approve-tools-match-the-spec
- Read: §13.2 of its owner spec; crates/shadows-mcp/src/server.rs
- Touch: the wrong side only (spec or code), plus `Grants::tools_for`
- Needs: Mohammed's ruling
- Done when: the MCP tool-list tests pass against the ruling

### A-7 · Correct the context-meter evidence
- Status: todo
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-7--correct-the-context-meter-evidence
- Read: docs/evidence/2026-10-21-context-meter-probe.md; §12.8
- Touch: docs/evidence/2026-10-21-context-meter-probe.md, docs/superpowers/specs/2026-09-24-harness-controls-design.md §12.8
- Done when: corrections cite inspected evidence, preserve the original run and distinguish inference from measurement; §12.8's OPEN closes only against its evidence trigger or an explicitly amended trigger; no API changes

### A-7b · Remove the breakdown route after agreement
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-7b--remove-the-breakdown-route-after-agreement
- Read: A-2, A-7, §12.8, harness/contract.yaml, docs/shared/README.md
- Touch: controller must enumerate the route/service/test/contract paths and api/openapi.json from the accepted request before dispatch; no web files in the backend task
- Needs: A-2; A-7's verified evidence decision; accepted shared request recorded here and in F-22
- Done when: backend removal tests and slash-command regressions pass; OpenAPI is regenerated by the canonical test; F-22 is complete on the delivery branch before release

### A-8 · Every service is built in `AppCore::start`
- Status: done
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-8--every-service-is-built-in-appcorestart
- Read: crates/shadows-core/src/app.rs, harness/sessions.rs, testing/
- Touch: crates/shadows-core/src/{app.rs,harness/sessions.rs,profiles/,testing/}, the profiles and harness contracts, the HTTP/MCP callers of agent deletion, crates/shadows/tests/architecture/, their tests
- Done when: affected construction/startup tests pass; services are built once, no service receives AppCore, and a restricted fixture exists; old CoreParts fields remain only for enumerated test callers until their owner tasks migrate them

### A-8b · The binary has one start and no second surface
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-8b--the-binary-has-one-start-and-no-second-surface
- Read: crates/shadows/src/{cli,daemon}/; docs/integration/2026-10-19-product-daemon-hermes-design.md; docs/aylul/specs/2026-10-19-aylul-claude-code-brain-design.md
- Touch: crates/shadows/src/, crates/shadows-http/src/ (the moved route), Core methods they need
- Needs: controller names each product-host operation's owner and exact paths; integration-spec amendment if ownership changes
- Done when: `cargo test -p shadows` passes; the binary holds only configuration, tracing, binding and signals

### A-8c · Settle the extraction dependencies before moving crates
- Status: done
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-8c--settle-the-extraction-dependencies-before-moving-crates
- Read: AppCore, service contracts and named implementations; db/{mod,command,journal,error,write_observer}.rs; owner spec §14.12
- Touch: docs/backend/plans/2026-10-21-architecture.md, docs/backend/stages.md, docs/superpowers/specs/2026-09-26-application-core-design.md
- Needs: controller performs this design task; it creates no crates
- Done when: the runtime/type/error/test dependency ledger is complete, each cycle and table has an owner, extraction order is verified, and exact scopes/test commands are recorded for the next preparation tasks

### A-8d · Foundation inputs carry no service types
- Status: done
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--establish-each-service-boundary-in-place and its "A-8c ledger" rows for A-8d
- Read: the ledger's Foundation table; db/{command,journal,error}.rs; command/; events/model.rs
- Touch: crates/shadows-core/src/{db,command,events,id.rs}/ and every call site of `user_command`, `append_event` and `Anchor` in crates/shadows-core/src/, crates/shadows-core/src/app.rs, crates/shadows/tests/architecture/ (allowlists, table-owners.txt), docs/codebase/README.md
- Needs: none
- Done when: `app::user_command` is gone (`CommandContext::user`); the journal takes `JournalEntry` with string ids; `Actor`, `Causation` and `newtype_id!` sit in `db/` (`Bus` stays, see the ledger); its domain methods no longer extend `Storage`; the resolved allowlist lines are deleted; `cargo test -p shadows --test architecture --test codemap` and `cargo test -p shadows-core --test storage_contract --test operation_lifecycle --test recovery` and `cargo test -p shadows --test protocol --test resync --test stream_frames` pass; clippy with and without `test-support` is clean for the touched crates; no crate move

> **Standing Touch rule for every remaining preparation task (controller, 2026-10-22).** When a task needs to read another owner's table, it may add a read-only method or associated `Owner::*_in(conn, …)` function to that owner (its `mod.rs`, store and `contract.yaml`), shaped like `Projects::require_live_in`, with one query per rule and no change to that owner's writes or behaviour. It is in Touch without a new ruling; the report names each one added.

### A-9 · Prepare the service boundary: instructions
- Status: done
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--establish-each-service-boundary-in-place and its "A-8c ledger" rows for A-9
- Read: instructions/contract.yaml; the ledger
- Touch: crates/shadows-core/src/instructions/, crates/shadows-core/src/projects/ (the new `require_live_in`), crates/shadows-core/src/app.rs, crates/shadows/tests/architecture/ (allowlists, table-owners.txt), docs/codebase/README.md; and, for every method this task moves to its owner, each caller's call site and import, the constructor parameter that hands that caller the owner (wired in `app.rs`), and the tests that call it (call-site changes only, no other edit in those files); docs/backend/stages.md (this task's Status)
- Needs: A-8d
- Done when: its domain methods no longer extend `Storage`; the resolved allowlist lines are deleted; `cargo test -p shadows --test architecture --test codemap` and `cargo test -p shadows --test planner_instructions` and `cargo test -p shadows-core --test project_contract` pass; clippy with and without `test-support` is clean for the touched crates; no crate move

### A-10 · Prepare the service boundary: secrets
- Status: done
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--establish-each-service-boundary-in-place and its "A-8c ledger" rows for A-10
- Read: secrets/contract.yaml; the ledger
- Touch: crates/shadows-core/src/secrets/ (the new `put_in`; the upward checks stay until A-14), crates/shadows-core/src/app.rs, crates/shadows/tests/architecture/ (allowlists, table-owners.txt), docs/codebase/README.md; and, for every method this task moves to its owner, each caller's call site and import, the constructor parameter that hands that caller the owner (wired in `app.rs`), and the tests that call it (call-site changes only, no other edit in those files); docs/backend/stages.md (this task's Status)
- Needs: A-9
- Done when: its domain methods no longer extend `Storage`; the resolved allowlist lines are deleted; `cargo test -p shadows --test architecture --test codemap` and `cargo test -p shadows-core --test secret_env --test aylul_profiles` pass; clippy with and without `test-support` is clean for the touched crates; no crate move

### A-11 · Prepare the service boundary: projects
- Status: done
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--establish-each-service-boundary-in-place and its "A-8c ledger" rows for A-11
- Read: projects/contract.yaml, threads/rules.rs; the ledger
- Touch: crates/shadows-core/src/projects/, crates/shadows-core/src/threads/rules.rs (`known_harness` moves to projects) and its callers, crates/shadows-core/src/app.rs, crates/shadows/tests/architecture/ (allowlists, table-owners.txt), docs/codebase/README.md; and, for every method this task moves to its owner, each caller's call site and import, the constructor parameter that hands that caller the owner (wired in `app.rs`), and the tests that call it (call-site changes only, no other edit in those files); docs/backend/stages.md (this task's Status)
- Needs: A-10
- Done when: `Projects::by_recent_activity` exists; its domain methods no longer extend `Storage`; the resolved allowlist lines are deleted; `cargo test -p shadows --test architecture --test codemap` and `cargo test -p shadows-core --test project_contract --test project_assistant` and `cargo test -p shadows --test project_directory --test project_assistant_route --test project_remove --test thread_contract` pass; clippy with and without `test-support` is clean for the touched crates; no crate move

### A-12 · Prepare the service boundary: code
- Status: done
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--establish-each-service-boundary-in-place and its "A-8c ledger" rows for A-12
- Read: code/contract.yaml; the ledger
- Touch: crates/shadows-core/src/code/ (link writes become `put_link_in`/`remove_link_in`; the plans notification stays at the call site until A-16), crates/shadows-core/src/projects/{mod.rs,store.rs,contract.yaml} (read-only `Projects::*_in(conn, …)` lookups that replace code's reads of the `project` table, shaped like `require_live_in`; no change to project removal), crates/shadows-core/src/app.rs, crates/shadows/tests/architecture/ (allowlists, table-owners.txt), docs/codebase/README.md; and, for every method this task moves to its owner, each caller's call site and import, the constructor parameter that hands that caller the owner (wired in `app.rs`), and the tests that call it (call-site changes only, no other edit in those files); docs/backend/stages.md (this task's Status)
- Needs: A-11
- Done when: `code_order` calls `Projects::by_recent_activity`; its domain methods no longer extend `Storage`; the resolved allowlist lines are deleted; `cargo test -p shadows --test architecture --test codemap` and `cargo test -p shadows-core --test code_index` and `cargo test -p shadows --test code_scope` pass; clippy with and without `test-support` is clean for the touched crates; no crate move

### A-12b · Prepare the service boundary: standards (out of design)
- Status: done
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--establish-each-service-boundary-in-place and its "A-8c ledger" rows for A-12b
- Read: design/standards.rs, design/store/standards.rs, design/contract.yaml; the ledger
- Touch: new crates/shadows-core/src/standards/, crates/shadows-core/src/design/{standards.rs,store/standards.rs,mod.rs}, the callers in harness/ and turns/, crates/shadows-core/src/turns/store/ (the invocation-pinned lookup: `agent_invocation` and `operation` belong to turns until A-17, so which standards version an operation heard is read there and `Standards` loads that version; any cross-owner line it still needs stays in the allowlist, named for A-17), crates/shadows-http/src/design.rs (the standards route), crates/shadows-core/src/app.rs, crates/shadows/tests/architecture/ (allowlists, table-owners.txt), docs/codebase/README.md; and, for every method this task moves to its owner, each caller's call site and import, the constructor parameter that hands that caller the owner (wired in `app.rs`), and the tests that call it (call-site changes only, no other edit in those files); docs/backend/stages.md (this task's Status)
- Needs: A-12
- Done when: `standards_additions_version` is owned by `standards`; its domain methods no longer extend `Storage`; the resolved allowlist lines are deleted; `cargo test -p shadows --test architecture --test codemap` and `cargo test -p shadows-core --test design_vision` and `cargo test -p shadows --test design_routes --test planner_mcp` pass; clippy with and without `test-support` is clean for the touched crates; no crate move

### A-14 · Prepare the service boundaries: providers, agents, extensions, usage, person
- Status: done
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--establish-each-service-boundary-in-place and its "A-8c ledger" rows for A-14
- Read: profiles/contract.yaml, harness/profile_home.rs; the ledger
- Touch: crates/shadows-core/src/profiles/ split into crates/shadows-core/src/{providers,agents,extensions,usage,person}/, harness/profile_home.rs (moves to agents), secrets/ (upward checks go), the callers in threads/, turns/store/usage.rs and harness/, crates/shadows-http/src/aylul_*.rs (provider update and delete call the `Agents` methods below), crates/shadows-core/src/instructions/store.rs (its journal append uses `JournalEntry`, not `events::DurableEvent`, as standards does), crates/shadows-core/src/{db/error.rs,error.rs} (the profile `StorageError` variants move to their owners' errors, as the ledger says), crates/shadows-core/src/db/mod.rs and crates/shadows-core/tests/storage_contract/txn.rs (the transaction error boundary in the second ruling), crates/shadows-core/src/app.rs, crates/shadows/tests/architecture/ (allowlists, table-owners.txt), docs/codebase/README.md; and, for every method this task moves to its owner, each caller's call site and import, the constructor parameter that hands that caller the owner (wired in `app.rs`), and the tests that call it (call-site changes only, no other edit in those files); docs/backend/stages.md (this task's Status)
- Ruling (controller, 2026-10-22): `providers` depends on `secrets` only, so it never reads `aylul_agent`. Checking that no agent still uses a provider or a removed model is an agents rule: `Agents::update_provider` and `Agents::delete_provider` run the check in their transaction and call `Providers::update_in` / `Providers::delete_in`. Agents and extensions prove an agent or provider exists through the owner's `*_in` method, never its table.
- Ruling 2 (controller, 2026-10-22, on the write_txn blocker): the foundation gets `Storage::write_txn_as<F, T, E>` where `E: From<StorageError>`, so a closure can refuse with its owner's error and the transaction still rolls back. It is the one implementation: `write_txn` becomes a thin call to it with `E = StorageError`, and the 75 existing callers do not change. Recovery, rollback and the committed-journal publish stay on the one path. `storage_contract/txn.rs` proves an owner error rolls back and the next write succeeds.
- Ruling 3 (controller, 2026-10-22, on the agent-deletion blocker): `Harness` already owns agent deletion (`Harness::delete_agent`, which also closes the agent's sessions) and sits above both owners, so it runs the one transaction: `Extensions::remove_agent_in(tx, slug)` and `Agents::delete_in(tx, slug)` (with the command record), then closes the sessions after commit, as today. `Profiles::delete_stored_agent` goes. The same rule settles any other atomic write that spans two of these owners: the service that already exposes that operation and sits above both owners in the resolved graph runs the transaction and calls each owner's `*_in`. Stop only if no such service exists.
- Needs: A-12b
- Done when: its domain methods no longer extend `Storage`; the resolved allowlist lines are deleted; `cargo test -p shadows --test architecture --test codemap` and `cargo test -p shadows-core --test aylul_profiles --test aylul_extensions --test aylul_usage --test profile_migration --test secret_env --test aylul_session` and `cargo test -p shadows-http --lib` pass; clippy with and without `test-support` is clean for the touched crates; no crate move

### A-15 · Prepare the service boundary: threads
- Status: done
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--establish-each-service-boundary-in-place and its "A-8c ledger" rows for A-15
- Read: threads/contract.yaml; the ledger
- Touch: crates/shadows-core/src/threads/ and the `append_entry_in`/`profile_in`/`title_from_*` call sites in design/, plans/, turns/, harness/, crates/shadows-core/src/app.rs, crates/shadows/tests/architecture/ (allowlists, table-owners.txt), docs/codebase/README.md; and, for every method this task moves to its owner, each caller's call site and import, the constructor parameter that hands that caller the owner (wired in `app.rs`), and the tests that call it (call-site changes only, no other edit in those files); docs/backend/stages.md (this task's Status)
- Ruling (controller, 2026-10-22, written before dispatch): `threads` depends on projects, code and agents only. Other services that read `planning_thread` or `thread_entry` SQL call a `Threads` method or an associated `Threads::*_in(conn, …)` read in the caller's transaction, like `Projects::require_live_in`. Writes that must share the caller's transaction use `Threads::*_in(&mut Tx, …)` and `write_txn_as` when the caller refuses with its own error. Cross-owner lines that a later task owns stay in the allowlist, re-keyed if the code moved, and the report names the task: `Threads::set_harness`'s session close (A-17), `Threads::remove`'s stop, close and revoke (A-18), the approval reads in `db/approval.rs` (A-13), project removal's thread check (A-16b). An atomic write across two owners runs in the service that already exposes it and sits above both (as A-14's Ruling 3); stop only if no such service exists.
- Ruling 2 (controller, 2026-10-22, on the operations blocker): the three edges from threads to operations are not A-15's. (a) Fork's operation checks (completion and busyness in its transaction) and (b) `Threads::operations` stay as they are, their allowlist lines kept and named for A-17b, where the fork entry becomes `Operations::fork_thread` (checks in its transaction, then `Threads::fork_in`) and the listing becomes an `Operations` read. (c) The `OperationId`, `WorkflowId` and `TaskId` imports in thread models stay; A-19 decides them with the extraction (opaque refs, as grants' workflow ref). A-15 does everything else.
- Needs: A-14
- Done when: its domain methods no longer extend `Storage`; the resolved allowlist lines are deleted; `cargo test -p shadows --test architecture --test codemap` and `cargo test -p shadows-core --test thread_session` and `cargo test -p shadows --test thread_contract --test thread_routes --test thread_title --test fork` pass; clippy with and without `test-support` is clean for the touched crates; no crate move

### A-13 · Prepare the service boundary: grants
- Status: done
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--establish-each-service-boundary-in-place and its "A-8c ledger" rows for A-13
- Read: grants/contract.yaml, command/approver.rs, db/approval.rs; the ledger
- Touch: crates/shadows-core/src/grants/, command/approver.rs and db/approval.rs (move to grants), the `check_writer`/`bind_draft_ref`/`issue_thread_grant` call sites in design/, plans/, harness/, crates/shadows-core/src/app.rs, crates/shadows/tests/architecture/ (allowlists, table-owners.txt), docs/codebase/README.md; and, for every method this task moves to its owner, each caller's call site and import, the constructor parameter that hands that caller the owner (wired in `app.rs`), and the tests that call it (call-site changes only, no other edit in those files); docs/backend/stages.md (this task's Status)
- Ruling (controller, 2026-10-22): A-13 moves the boundary only. `Grants::tools_for` moves with grants and keeps today's tool list exactly; which approve tools the Planner gets is A-6's ruling and A-6's change. Other services read `draft_intent`, grant and approval tables only through `Grants` methods or associated `Grants::*_in(conn, …)` reads; an atomic write across two owners runs in the service that already exposes it and sits above both (A-14 Ruling 3); a cross-owner line a later task owns stays, named with that task. Grants reads the request entry through `Threads::entry_in` (ledger). The `GrantInvalid` and `GrantScope` `StorageError` variants move to the grants error with `From<GrantsError> for CoreError`; codes and messages unchanged.
- Needs: A-15; A-6 for the tool policy
- Done when: `draft_intent` is owned by grants with an opaque workflow ref; its domain methods no longer extend `Storage`; the resolved allowlist lines are deleted; `cargo test -p shadows --test architecture --test codemap` and `cargo test -p shadows --test grants --test plan_grants --test agreement_mcp --test planner_mcp` pass; clippy with and without `test-support` is clean for the touched crates; no crate move

### A-17b · Prepare the service boundary: operations (out of turns and runtime)
- Status: done
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--establish-each-service-boundary-in-place and its "A-8c ledger" rows for A-17b
- Read: turns/contract.yaml, runtime/contract.yaml; the ledger
- Touch: new crates/shadows-core/src/operations/, crates/shadows-core/src/turns/{handles.rs,shutdown.rs,store/operation*.rs,store/transition.rs}, runtime/, the running-turn call sites in design/, plans/, threads/, harness/, crates/shadows-core/src/app.rs, crates/shadows/tests/architecture/ (allowlists, table-owners.txt), docs/codebase/README.md; and, for every method this task moves to its owner, each caller's call site and import, the constructor parameter that hands that caller the owner (wired in `app.rs`), and the tests that call it (call-site changes only, no other edit in those files); docs/backend/stages.md (this task's Status)
- From A-15 (Ruling 2): fork moves to `Operations::fork_thread` (operation checks in its transaction, then `Threads::fork_in`), and `Threads::operations` becomes an `Operations` read; the threads/ call sites and the fork route are in Touch.
- Ruling (controller, 2026-10-22, written before dispatch): `operations` depends on threads and runtime only. It owns the `operation` table, `LiveHandles`, the running-turn check (`Turns::require_running_in`, added in A-13, becomes `Operations::require_running_in`), stop, transitions and recovery. Turns creates and finishes operations through `Operations::*_in(&mut Tx, …)` in its own transactions, and registers the live handle through an `Operations` method; it keeps sending, queueing and steering. `ThreadBusy` and `QueuedMessageGone` stay with turns. Design, plans, harness and threads call `Operations`, never turns, for a running-turn check. The standing Touch rule (above A-9) and A-14's Ruling 3 apply; a cross-owner line a later task owns stays, named with that task.
- Ruling 2 (controller, 2026-10-22, on the live-turn blocker; the ledger's `operations` row is amended): `operations` owns the durable lifecycle only (rows, transitions, the running-turn check from the rows, recovery). `LiveHandles`, `LiveTurn`, stop and shutdown's live part stay in `turns`, which calls `Harness`/`Sessions` to close the session and `Operations::*_in` to record each transition. Callers below turns that need "is a turn running" use `Operations::require_running_in` (rows), never the live handles. `runtime` keeps register and stop as today if moving them would need turns or harness types.
- Ruling 3 (controller, 2026-10-22, on the invocation-read blocker): `Operations` reads its own rows only. The thread's operation listing that adds `Operation.invocation` (from `agent_invocation`, turns' until A-17) is composed by `Turns`, which sits above both: it takes `Operations`' rows and adds the invocation; the route calls that `Turns` method, and the HTTP shape is unchanged. The same rule settles any other read that joins two owners: the service above both that already exposes it composes it from each owner's read. Apply it and report each case; stop only if no such service exists.
- Needs: A-13
- Done when: recovery is `Operations::recover`; `Runtime` has no `pub storage`; its domain methods no longer extend `Storage`; the resolved allowlist lines are deleted; `cargo test -p shadows --test architecture --test codemap` and `cargo test -p shadows-core --test operation_lifecycle --test recovery` and `cargo test -p shadows --test shutdown --test core_turns --test message_queue` pass; clippy with and without `test-support` is clean for the touched crates; no crate move

### A-17 · Prepare the service boundary: harness
- Status: todo
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--establish-each-service-boundary-in-place and its "A-8c ledger" rows for A-17
- Read: harness/contract.yaml; the ledger
- Touch: crates/shadows-core/src/harness/, threads/mod.rs (`set_harness` close moves to `Harness::set_thread_harness`), the `agent_invocation` writers in turns/, crates/shadows-core/src/app.rs, crates/shadows/tests/architecture/ (allowlists, table-owners.txt), docs/codebase/README.md; and, for every method this task moves to its owner, each caller's call site and import, the constructor parameter that hands that caller the owner (wired in `app.rs`), and the tests that call it (call-site changes only, no other edit in those files); docs/backend/stages.md (this task's Status)
- Ruling (controller, 2026-10-23, on the stage-line blocker): the stage line a turn carries is composed by `Turns`, which sits above harness and design (the ledger's turns row lists design). Design owns the stage rule as one read method (the project's stage view turned into its line, what `stage::line(project_stage(..))` does today), with its line in design/contract.yaml. `Turns` reads the thread's context, leaves the line empty for an AYLUL thread exactly as today, calls that Design method, and hands the line to Harness as a fact for normal prompts (`context_facts`) and steered prompts (`steer.rs`) alike: one method in turns serves both. Harness keeps no Design read. Touch adds turns/spawn.rs, turns/steer.rs and the turns file that holds the shared stage method, the one Design method and its contract line, and their tests' call sites. Prompt text, order and the `every_turn_ends_with_the_stage_line` agreement stay unchanged.
- Needs: A-17b
- Done when: `Sessions` is private to `Harness`; `agent_invocation` is owned by harness; its domain methods no longer extend `Storage`; the resolved allowlist lines are deleted; `cargo test -p shadows --test architecture --test codemap` and `cargo test -p shadows-core --test sessions --test aylul_session --test harness_schema --test acp_connection --test thread_session` and `cargo test -p shadows --test core_harness --test harness_choices --test harness_config --test harness_effort --test harness_observation --test slash_commands` pass; clippy with and without `test-support` is clean for the touched crates; no crate move

### A-16 · Prepare the service boundaries: design and plans
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--establish-each-service-boundary-in-place and its "A-8c ledger" rows for A-16
- Read: design/contract.yaml, plans/contract.yaml; the ledger
- Touch: crates/shadows-core/src/{design,plans}/, crates/shadows-core/src/code/ (the link commands move to `Plans`), crates/shadows-http/src/{design.rs,plans*.rs} and crates/shadows-mcp/src/ for the moved methods, crates/shadows-core/src/app.rs, crates/shadows/tests/architecture/ (allowlists, table-owners.txt), docs/codebase/README.md; and, for every method this task moves to its owner, each caller's call site and import, the constructor parameter that hands that caller the owner (wired in `app.rs`), and the tests that call it (call-site changes only, no other edit in those files); docs/backend/stages.md (this task's Status)
- Needs: A-17
- Done when: design imports no plans type; `design_part_plan`/`design_outcome_plan` are owned by plans; its domain methods no longer extend `Storage`; the resolved allowlist lines are deleted; `cargo test -p shadows --test architecture --test codemap` and `cargo test -p shadows-core --test agreement_bindings --test agreement_lifecycle --test agreement_review --test agreement_validation --test design_outcomes --test design_parts --test design_vision --test design_migration --test cross_plan_links --test cross_plan_resolution --test cross_plan_views --test plan_migration --test workflow_rules --test planner_isolation` and `cargo test -p shadows --test agreement_journey --test agreement_mcp --test agreement_routes --test design_routes --test core_plans --test cross_plan_mcp --test plan_boundaries --test plan_in_conversation --test plan_map --test plan_routes --test plan_storage --test project_plans --test planner_mcp --test workspace_mcp` pass; clippy with and without `test-support` is clean for the touched crates; no crate move

### A-18 · Prepare the service boundary: turns
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--establish-each-service-boundary-in-place and its "A-8c ledger" rows for A-18
- Read: turns/contract.yaml; the ledger
- Touch: crates/shadows-core/src/turns/, threads/mod.rs (`remove`'s stop, close and revoke move to `Turns::remove_thread`), crates/shadows-http/src/ and crates/shadows-mcp/src/ callers of thread removal, crates/shadows-core/src/app.rs, crates/shadows/tests/architecture/ (allowlists, table-owners.txt), docs/codebase/README.md; and, for every method this task moves to its owner, each caller's call site and import, the constructor parameter that hands that caller the owner (wired in `app.rs`), and the tests that call it (call-site changes only, no other edit in those files); docs/backend/stages.md (this task's Status)
- Needs: A-16
- Done when: threads calls neither harness nor turns; `Bus` is owned by turns and `Events` subscribes through a `Turns` method; its domain methods no longer extend `Storage`; the resolved allowlist lines are deleted; `cargo test -p shadows --test architecture --test codemap` and `cargo test -p shadows --test core_turns --test message_queue --test planner_turn --test slash_commands --test turn_command --test planner_mcp --test thread_routes` pass; clippy with and without `test-support` is clean for the touched crates; no crate move

### A-16b · Project removal has its own owner
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--establish-each-service-boundary-in-place and its "A-8c ledger" rows for A-16b
- Read: projects/contract.yaml; the ledger's `project-removal` row
- Touch: new crates/shadows-core/src/project_removal/, crates/shadows-core/src/projects/, the project-removal route in crates/shadows-http/src/, crates/shadows-core/src/app.rs, crates/shadows/tests/architecture/ (allowlists, table-owners.txt), docs/codebase/README.md; and, for every method this task moves to its owner, each caller's call site and import, the constructor parameter that hands that caller the owner (wired in `app.rs`), and the tests that call it (call-site changes only, no other edit in those files); docs/backend/stages.md (this task's Status)
- Needs: A-18
- Done when: projects calls no code, plans, grants or threads; its domain methods no longer extend `Storage`; the resolved allowlist lines are deleted; `cargo test -p shadows --test architecture --test codemap` and `cargo test -p shadows --test project_remove` and `cargo test -p shadows-core --test project_contract` pass; clippy with and without `test-support` is clean for the touched crates; no crate move

### A-19 · Prepare the service boundary: events, runtime and command
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--establish-each-service-boundary-in-place and its "A-8c ledger" rows for A-19
- Read: events/, runtime/, command/ contracts; the ledger
- Touch: crates/shadows-core/src/{events,runtime,command}/, plans/ (the payload projection moves in), crates/shadows-core/src/app.rs, crates/shadows/tests/architecture/ (allowlists, table-owners.txt), docs/codebase/README.md; and, for every method this task moves to its owner, each caller's call site and import, the constructor parameter that hands that caller the owner (wired in `app.rs`), and the tests that call it (call-site changes only, no other edit in those files); docs/backend/stages.md (this task's Status)
- From A-15 (Ruling 2): decide the `OperationId`/`WorkflowId`/`TaskId` imports in thread models (opaque refs, as grants' workflow ref), so threads imports no service above it.
- Needs: A-16b
- Done when: the A-0 cross-owner allowlists are empty, or each remaining line is named in this task's report with its owner; its domain methods no longer extend `Storage`; the resolved allowlist lines are deleted; `cargo test -p shadows --test architecture --test codemap` and `cargo test -p shadows --test project_events --test resync --test stream_frames --test protocol` and `cargo test -p shadows-core --test recovery` pass; clippy with and without `test-support` is clean for the touched crates; no crate move

### A-19b · Extract the prepared database foundation
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-19b--extract-the-prepared-database-foundation
- Read: A-8c's resolved ledger; prepared db boundary and storage tests; owner spec §14.12
- Touch: controller enumerates crates/core-db/, old db paths, service imports, manifests/lockfile, fixtures, contracts and code map before dispatch
- Needs: A-9 to A-19 preparations complete; no domain impl Storage; persistence inputs/errors have no service dependencies
- Done when: foundation and affected cross-service tests prove rollback, replay/conflict, cancellation recovery and after-commit notification; workspace targets compile; core-db has no dependency on shadows-core or a service

### A-19c · Extract services in the verified order
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-19c--extract-services-in-the-verified-order
- Read: A-8c's runtime/type/test graph; owner spec §14.12
- Touch: controller expands this milestone into exact per-service tasks, including manifests, lockfile, re-exports, app wiring, callers, contracts, maps and tests
- Needs: A-19b; a concrete task per move in verified topological order
- Done when: each move passes affected public-API/caller tests and architecture checks; no cycles, cross-owner SQL or service-to-AppCore dependency; integration tests still run through AppCore

### A-20 · `shadows-http` and `shadows-mcp` split by job
- Status: todo
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-20--shadows-http-and-shadows-mcp-split-by-job
- Read: the plan section
- Touch: crates/shadows-http/src/, crates/shadows-mcp/src/
- Done when: `cargo test -p shadows-http -p shadows-mcp -p shadows` passes; `api/` unchanged

### A-21 · `shadows-agent` and `shadows-process` split by job
- Status: todo
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-21--shadows-agent-and-shadows-process-split-by-job
- Read: the plan section
- Touch: crates/shadows-agent/src/, crates/shadows-process/src/, crates/fake-acp/src/
- Done when: affected shadows-agent, shadows-process and fake-acp tests pass; compile the affected consumers

### A-22 · Split the repository into workspaces
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-22--split-the-repository-by-product
- Read: the plan section; CLAUDE.md "Workspaces"
- Touch: runtime/ and code-intelligence/ (new), Cargo.toml, CLAUDE.md, AGENTS.md, docs/codebase/README.md, the codemap test, .github/workflows/ci.yml
- Needs: A-19c; controller records all path/dev dependencies, test binary lookups, feature changes and native consumers; separate delivery branch
- Done when: each workspace's applicable gate passes from its own root and affected native consumers compile; Core retains its OpenAPI gate

### A-23 · No `§` and no history in code
- Status: todo
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-23--no--and-no-history-in-code
- Read: A-0's list (c)
- Touch: every remaining file on that list
- Done when: A-0's list (c) is empty

### B-10 · A conversation keeps its mode
- Status: blocked
- Plan: none. Owner spec: docs/superpowers/specs/2026-09-24-harness-controls-design.md §12.4 ("The mode is remembered per conversation"); request docs/shared/requests/2026-10-22-remembered-mode.md (accepted)
- Read: harness/contract.yaml, harness/store.rs, harness/settings.rs, crates/shadows-agent/src/policy.rs; the request
- Touch: crates/shadows-core/src/harness/ (store, settings, the new command, contract.yaml), a migration under crates/shadows-core/src/db/ for the per-conversation mode row, crates/shadows-http/src/harness.rs (the route), api/openapi.json (regenerated), the threads removal/fork call sites that must remove or copy the row, crates/shadows/tests/ (new acceptance tests), docs/codebase/README.md
- Needs: A-17 (the harness boundary is reshaped there)
- Done when: picking Auto without sending, then reopening the session and restarting Core, answers Auto; a mode the project disallows is refused before anything is written; a disallowed remembered mode falls back to the policy default on open; a running turn keeps its mode and the new one applies to the next turn; one conversation's mode does not reach another; a fork copies it; a replay with a different mode is `CommandConflict`; `cargo test -p shadows --test openapi` and the new tests pass

### B-11 · Images in a message
- Status: blocked
- Plan: none. Owner specs: docs/superpowers/specs/2026-09-21-domain-model-design.md §4.2 "Attachment", §12.11; request docs/shared/requests/2026-10-22-image-attachments.md (accepted)
- Read: threads/contract.yaml, turns/contract.yaml (prompt composition), harness/contract.yaml, crates/shadows-agent/src/acp.rs (`Connection::prompt`, steering, `initialize`); the request
- Touch: crates/shadows-core/src/threads/ (attachment record, bytes, upload/read, removal, fork), a migration under crates/shadows-core/src/db/, crates/shadows-core/src/turns/ (ids in turn/queue/steer commands and fingerprints; neutral image blocks in `Prompt::compose`), crates/shadows-core/src/harness/ (capabilities in the session answer), crates/shadows-agent/src/ (neutral blocks to ACP image blocks; keep `promptCapabilities`), crates/shadows-http/src/ (the two routes, the new fields), api/openapi.json (regenerated), crates/shadows/tests/ (new acceptance tests), docs/codebase/README.md
- Needs: A-18 (turns is reshaped there); B-10 is independent
- Done when: an uploaded PNG is refused when its bytes are not an image and over the limit; a turn with text plus two images sends the text and both images in order to the fake ACP agent (exact blocks asserted); an image-only turn works; an id from another thread is refused before anything is written; a replay with other ids is `CommandConflict`; a queued and a steered message keep their images; a fork copies the references; removing the thread removes the bytes; a harness without image capability refuses with its reason; no ACP type appears in HTTP or stored entries; `cargo test -p shadows --test openapi` and the new tests pass

### B-A2 · Prepare Core's scoped context contract
- Status: stopped (2026-10-19: its Hermes integration plan was deleted, Git `226d0a3`; AYLUL's work profile will define its own need)
- Plan: none
- Read: projects/threads/plans/design contracts; architecture section 1.7; approval section 24.6; integration T3.
- Touch: the required shared request/owner specification first; implementation paths only after that contract is accepted.
- Needs: accepted caller/sharing/authority contract and counterpart native/UI request.
- Done when: the scoped projection and its refusal/revocation/identity tests are agreed without assigning AYLUL or LSP runtime ownership to AppCore.
