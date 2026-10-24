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
- Status: done
- Ruling (controller, 2026-10-23, on the adapter-handoff blocker): Turns settles the turn's model with Harness (`offer_for_model` and any adapter replacement it needs) before it leases the connection's events, then leases from the session Harness returns, so no turn holds an old `OpenSession` or receiver. Touch adds turns/mod.rs, turns/record.rs, the turns contract lines and the turn lifecycle tests they change. A running turn keeps its adapter; the switch applies from the next turn.
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-3--the-aylul-window-follows-the-current-model
- Read: harness/contract.yaml
- Touch: crates/shadows-core/src/harness/ (assistant.rs, live.rs, settings.rs), harness/contract.yaml, its tests
- Done when: `switching_model_uses_the_new_models_window` and the agreement test pass

### A-4 · One error mapping for HTTP and MCP
- Status: done
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-4--one-error-mapping-for-http-and-mcp
- Read: crates/shadows-core/src/error.rs
- Touch: crates/shadows-core/src/error.rs, crates/shadows-http/src/failure/, crates/shadows-mcp/src/refusal.rs, their tests
- Done when: the table test over every `CoreError` variant passes through both adapters

### A-5 · ACP connection safety
- Status: done
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
- Status: done
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
- Status: done
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--establish-each-service-boundary-in-place and its "A-8c ledger" rows for A-17
- Read: harness/contract.yaml; the ledger
- Touch: crates/shadows-core/src/harness/, threads/mod.rs (`set_harness` close moves to `Harness::set_thread_harness`), the `agent_invocation` writers in turns/, crates/shadows-core/src/app.rs, crates/shadows/tests/architecture/ (allowlists, table-owners.txt), docs/codebase/README.md; and, for every method this task moves to its owner, each caller's call site and import, the constructor parameter that hands that caller the owner (wired in `app.rs`), and the tests that call it (call-site changes only, no other edit in those files); docs/backend/stages.md (this task's Status)
- Ruling (controller, 2026-10-23, on the stage-line blocker): the stage line a turn carries is composed by `Turns`, which sits above harness and design (the ledger's turns row lists design). Design owns the stage rule as one read method (the project's stage view turned into its line, what `stage::line(project_stage(..))` does today), with its line in design/contract.yaml. `Turns` reads the thread's context, leaves the line empty for an AYLUL thread exactly as today, calls that Design method, and hands the line to Harness as a fact for normal prompts (`context_facts`) and steered prompts (`steer.rs`) alike: one method in turns serves both. Harness keeps no Design read. Touch adds turns/spawn.rs, turns/steer.rs and the turns file that holds the shared stage method, the one Design method and its contract line, and their tests' call sites. Prompt text, order and the `every_turn_ends_with_the_stage_line` agreement stay unchanged.
- Ruling 2 (controller, 2026-10-23, on the review's graph blocker): design and plans read `agent_invocation` facts through Harness read lookups (`pinned_standards_in`, `invocation_model_in`). Harness sits below both and depends on neither, so the edge is acyclic; the ledger's design and plans rows gain `harness` for these reads only. No orchestration moves.
- Needs: A-17b
- Done when: `Sessions` is private to `Harness`; `agent_invocation` is owned by harness; its domain methods no longer extend `Storage`; the resolved allowlist lines are deleted; `cargo test -p shadows --test architecture --test codemap` and `cargo test -p shadows-core --test sessions --test aylul_session --test harness_schema --test acp_connection --test thread_session` and `cargo test -p shadows --test core_harness --test harness_choices --test harness_config --test harness_effort --test harness_observation --test slash_commands` pass; clippy with and without `test-support` is clean for the touched crates; no crate move

### A-16 · Prepare the service boundaries: design and plans
- Status: done
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--establish-each-service-boundary-in-place and its "A-8c ledger" rows for A-16
- Read: design/contract.yaml, plans/contract.yaml; the ledger
- Touch: crates/shadows-core/src/{design,plans}/, crates/shadows-core/src/code/ (the link commands move to `Plans`), crates/shadows-http/src/{design.rs,plans*.rs} and crates/shadows-mcp/src/ for the moved methods, crates/shadows-core/src/app.rs, crates/shadows/tests/architecture/ (allowlists, table-owners.txt), docs/codebase/README.md; and, for every method this task moves to its owner, each caller's call site and import, the constructor parameter that hands that caller the owner (wired in `app.rs`), and the tests that call it (call-site changes only, no other edit in those files); docs/backend/stages.md (this task's Status)
- Ruling (controller, 2026-10-23, on the plan-show blocker): `plan_show` is composed by `Turns`, which owns `LiveHandles` and sits above plans. `Plans::show` records the card and returns what it recorded (with the operation and whether it was a replay) and holds no `LiveHandles`; a new `Turns::show_plan` calls it, names the tab with `client_tab_for`, and sends the live signal when it was not a replay, exactly as today. The MCP `plan_show` tool calls `Turns::show_plan`. Touch adds that Turns method and its contract line, its file in turns/, the UI-signal sender's wiring in app.rs, and the MCP tool's call site. Card, signal, tab and replay behaviour stay unchanged.
- Needs: A-17
- Done when: design imports no plans type; `design_part_plan`/`design_outcome_plan` are owned by plans; its domain methods no longer extend `Storage`; the resolved allowlist lines are deleted; `cargo test -p shadows --test architecture --test codemap` and `cargo test -p shadows-core --test agreement_bindings --test agreement_lifecycle --test agreement_review --test agreement_validation --test design_outcomes --test design_parts --test design_vision --test design_migration --test cross_plan_links --test cross_plan_resolution --test cross_plan_views --test plan_migration --test workflow_rules --test planner_isolation` and `cargo test -p shadows --test agreement_journey --test agreement_mcp --test agreement_routes --test design_routes --test core_plans --test cross_plan_mcp --test plan_boundaries --test plan_in_conversation --test plan_map --test plan_routes --test plan_storage --test project_plans --test planner_mcp --test workspace_mcp` pass; clippy with and without `test-support` is clean for the touched crates; no crate move

### A-18 · Prepare the service boundary: turns
- Status: done; reviewed (corrected Events' Bus owner and removed the schema-independent UUID comment exception)
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--establish-each-service-boundary-in-place and its "A-8c ledger" rows for A-18
- Read: turns/contract.yaml; the ledger
- Touch: crates/shadows-core/src/turns/, threads/mod.rs (`remove`'s stop, close and revoke move to `Turns::remove_thread`), crates/shadows-http/src/ and crates/shadows-mcp/src/ callers of thread removal, crates/shadows-core/src/app.rs, crates/shadows/tests/architecture/ (allowlists, table-owners.txt), docs/codebase/README.md; and, for every method this task moves to its owner, each caller's call site and import, the constructor parameter that hands that caller the owner (wired in `app.rs`), and the tests that call it (call-site changes only, no other edit in those files); docs/backend/stages.md (this task's Status)
- Needs: A-16
- Done when: threads calls neither harness nor turns; `Bus` is owned by turns and `Events` subscribes through a `Turns` method; its domain methods no longer extend `Storage`; the resolved allowlist lines are deleted; `cargo test -p shadows --test architecture --test codemap` and `cargo test -p shadows --test core_turns --test message_queue --test planner_turn --test slash_commands --test turn_command --test planner_mcp --test thread_routes` pass; clippy with and without `test-support` is clean for the touched crates; no crate move

### A-16b · Project removal has its own owner
- Status: done
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--establish-each-service-boundary-in-place and its "A-8c ledger" rows for A-16b
- Read: projects/contract.yaml; the ledger's `project-removal` row
- Touch: new crates/shadows-core/src/project_lifecycle/, crates/shadows-core/src/projects/, the project-removal route in crates/shadows-http/src/, crates/shadows-core/src/app.rs, crates/shadows/tests/architecture/ (allowlists, table-owners.txt), docs/codebase/README.md; and, for every method this task moves to its owner, each caller's call site and import, the constructor parameter that hands that caller the owner (wired in `app.rs`), and the tests that call it (call-site changes only, no other edit in those files); docs/backend/stages.md (this task's Status)
- Ruling (controller, 2026-10-23, on the creation blocker): `projects` depends on nothing, so `Projects::create` stops calling `Code::touch`. The new owner is `project_lifecycle` (crates/shadows-core/src/project_lifecycle/, ledger row 19 renamed, same dependencies): it owns a project's cross-owner lifecycle. `ProjectLifecycle::create` calls `Projects::create`, then `Code::touch` exactly as today (also on a replay); `ProjectLifecycle::remove` is the removal this task already moves. The HTTP and MCP create and remove callers call it. Touch adds the project create route and tool call sites. Behaviour, codes and indexing timing unchanged.
- Needs: A-18
- Done when: projects calls no code, plans, grants or threads; its domain methods no longer extend `Storage`; the resolved allowlist lines are deleted; `cargo test -p shadows --test architecture --test codemap` and `cargo test -p shadows --test project_remove` and `cargo test -p shadows-core --test project_contract` pass; clippy with and without `test-support` is clean for the touched crates; no crate move

### A-19 · Prepare the service boundary: events, runtime and command
- Status: done
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--establish-each-service-boundary-in-place and its "A-8c ledger" rows for A-19
- Read: events/, runtime/, command/ contracts; the ledger
- Touch: crates/shadows-core/src/{events,runtime,command}/, plans/ (the payload projection moves in), crates/shadows-core/src/app.rs, crates/shadows/tests/architecture/ (allowlists, table-owners.txt), docs/codebase/README.md; and, for every method this task moves to its owner, each caller's call site and import, the constructor parameter that hands that caller the owner (wired in `app.rs`), and the tests that call it (call-site changes only, no other edit in those files); docs/backend/stages.md (this task's Status)
- From A-15 (Ruling 2): decide the `OperationId`/`WorkflowId`/`TaskId` imports in thread models (opaque refs, as grants' workflow ref), so threads imports no service above it.
- Needs: A-16b
- Done when: the A-0 cross-owner allowlists are empty, or each remaining line is named in this task's report with its owner; its domain methods no longer extend `Storage`; the resolved allowlist lines are deleted; `cargo test -p shadows --test architecture --test codemap` and `cargo test -p shadows --test project_events --test resync --test stream_frames --test protocol` and `cargo test -p shadows-core --test recovery` pass; clippy with and without `test-support` is clean for the touched crates; no crate move

### A-19b · Extract the prepared database foundation
- Status: done
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-19b--extract-the-prepared-database-foundation
- Read: A-8c's resolved ledger; prepared db boundary and storage tests; owner spec §14.12
- Touch: Enumerated for this dispatch before implementation:
  - New foundation: `crates/core-db/Cargo.toml`, `crates/core-db/src/`,
    `crates/core-db/tests/`, `crates/core-db/contract.yaml`, `crates/core-db/migrations/`.
  - Move every file in `crates/shadows-core/src/db/` and
    `crates/shadows-core/migrations/`; move the database tests from
    `crates/shadows-core/tests/storage_contract/` (retain an application test entry
    only if needed for the assigned cross-owner check).
  - Manifests/features: `Cargo.toml`, `crates/shadows-core/Cargo.toml`, `Cargo.lock`;
    forward `test-support` for the moved observer without enabling shipping hooks.
  - Service imports, re-exports, command/journal call sites, test fixtures,
    contracts and architecture path checks (exact existing paths):
    - `crates/shadows-core/src/agents/contract.yaml`
    - `crates/shadows-core/src/agents/error.rs`
    - `crates/shadows-core/src/agents/home.rs`
    - `crates/shadows-core/src/agents/mod.rs`
    - `crates/shadows-core/src/agents/store.rs`
    - `crates/shadows-core/src/app.rs`
    - `crates/shadows-core/src/code/contract.yaml`
    - `crates/shadows-core/src/code/links.rs`
    - `crates/shadows-core/src/code/mod.rs`
    - `crates/shadows-core/src/code/store/links.rs`
    - `crates/shadows-core/src/code/store/mod.rs`
    - `crates/shadows-core/src/code/store/reach.rs`
    - `crates/shadows-core/src/code/watch.rs`
    - `crates/shadows-core/src/db/journal.rs`
    - `crates/shadows-core/src/db/mod.rs`
    - `crates/shadows-core/src/db/write_observer.rs`
    - `crates/shadows-core/src/design/agreement.rs`
    - `crates/shadows-core/src/design/agreement_scope.rs`
    - `crates/shadows-core/src/design/agreements.rs`
    - `crates/shadows-core/src/design/contract.yaml`
    - `crates/shadows-core/src/design/map_proposals.rs`
    - `crates/shadows-core/src/design/mod.rs`
    - `crates/shadows-core/src/design/model.rs`
    - `crates/shadows-core/src/design/outcomes.rs`
    - `crates/shadows-core/src/design/parts.rs`
    - `crates/shadows-core/src/design/proposal_context.rs`
    - `crates/shadows-core/src/design/proposals.rs`
    - `crates/shadows-core/src/design/roadmap_proposals.rs`
    - `crates/shadows-core/src/design/store/agreement_binding.rs`
    - `crates/shadows-core/src/design/store/agreement_write.rs`
    - `crates/shadows-core/src/design/store/agreements.rs`
    - `crates/shadows-core/src/design/store/map_approval.rs`
    - `crates/shadows-core/src/design/store/map_proposals.rs`
    - `crates/shadows-core/src/design/store/map_versions.rs`
    - `crates/shadows-core/src/design/store/mod.rs`
    - `crates/shadows-core/src/design/store/proposal_approval.rs`
    - `crates/shadows-core/src/design/store/proposals.rs`
    - `crates/shadows-core/src/design/store/roadmap_approval.rs`
    - `crates/shadows-core/src/design/store/roadmap_proposals.rs`
    - `crates/shadows-core/src/design/store/roadmap_seed.rs`
    - `crates/shadows-core/src/design/store/stage.rs`
    - `crates/shadows-core/src/design/store/structure.rs`
    - `crates/shadows-core/src/design/store/vision.rs`
    - `crates/shadows-core/src/design/structure_create.rs`
    - `crates/shadows-core/src/error.rs`
    - `crates/shadows-core/src/events/contract.yaml`
    - `crates/shadows-core/src/events/mod.rs`
    - `crates/shadows-core/src/events/model.rs`
    - `crates/shadows-core/src/events/store.rs`
    - `crates/shadows-core/src/extensions/contract.yaml`
    - `crates/shadows-core/src/extensions/error.rs`
    - `crates/shadows-core/src/extensions/mod.rs`
    - `crates/shadows-core/src/extensions/store.rs`
    - `crates/shadows-core/src/grants/approver.rs`
    - `crates/shadows-core/src/grants/contract.yaml`
    - `crates/shadows-core/src/grants/error.rs`
    - `crates/shadows-core/src/grants/mod.rs`
    - `crates/shadows-core/src/grants/model.rs`
    - `crates/shadows-core/src/grants/store.rs`
    - `crates/shadows-core/src/grants/testing.rs`
    - `crates/shadows-core/src/grants/writer.rs`
    - `crates/shadows-core/src/harness/access.rs`
    - `crates/shadows-core/src/harness/contract.yaml`
    - `crates/shadows-core/src/harness/fork.rs`
    - `crates/shadows-core/src/harness/live.rs`
    - `crates/shadows-core/src/harness/mod.rs`
    - `crates/shadows-core/src/harness/mode.rs`
    - `crates/shadows-core/src/harness/setup.rs`
    - `crates/shadows-core/src/harness/store/invocation.rs`
    - `crates/shadows-core/src/harness/store/invocations.rs`
    - `crates/shadows-core/src/harness/store/limits.rs`
    - `crates/shadows-core/src/harness/store/mod.rs`
    - `crates/shadows-core/src/harness/store/mode.rs`
    - `crates/shadows-core/src/harness/store/preferences.rs`
    - `crates/shadows-core/src/harness/store/reads.rs`
    - `crates/shadows-core/src/harness/store/thread.rs`
    - `crates/shadows-core/src/harness/titles.rs`
    - `crates/shadows-core/src/instructions/contract.yaml`
    - `crates/shadows-core/src/instructions/mod.rs`
    - `crates/shadows-core/src/instructions/store.rs`
    - `crates/shadows-core/src/lib.rs`
    - `crates/shadows-core/src/operations/contract.yaml`
    - `crates/shadows-core/src/operations/mod.rs`
    - `crates/shadows-core/src/operations/model.rs`
    - `crates/shadows-core/src/operations/store/checks.rs`
    - `crates/shadows-core/src/operations/store/lifecycle.rs`
    - `crates/shadows-core/src/operations/store/mod.rs`
    - `crates/shadows-core/src/operations/store/read.rs`
    - `crates/shadows-core/src/operations/store/recovery.rs`
    - `crates/shadows-core/src/operations/store/runtime_stop.rs`
    - `crates/shadows-core/src/operations/store/transition.rs`
    - `crates/shadows-core/src/operations/testing.rs`
    - `crates/shadows-core/src/person/contract.yaml`
    - `crates/shadows-core/src/person/mod.rs`
    - `crates/shadows-core/src/person/store.rs`
    - `crates/shadows-core/src/plans/agreements.rs`
    - `crates/shadows-core/src/plans/binding_edit.rs`
    - `crates/shadows-core/src/plans/contract.yaml`
    - `crates/shadows-core/src/plans/design_edit.rs`
    - `crates/shadows-core/src/plans/mod.rs`
    - `crates/shadows-core/src/plans/model.rs`
    - `crates/shadows-core/src/plans/project_links.rs`
    - `crates/shadows-core/src/plans/roadmap_approval.rs`
    - `crates/shadows-core/src/plans/scope.rs`
    - `crates/shadows-core/src/plans/store/agreement_review.rs`
    - `crates/shadows-core/src/plans/store/binding_participants.rs`
    - `crates/shadows-core/src/plans/store/bindings.rs`
    - `crates/shadows-core/src/plans/store/dependencies.rs`
    - `crates/shadows-core/src/plans/store/draft.rs`
    - `crates/shadows-core/src/plans/store/edit.rs`
    - `crates/shadows-core/src/plans/store/event_projection.rs`
    - `crates/shadows-core/src/plans/store/graph.rs`
    - `crates/shadows-core/src/plans/store/linked.rs`
    - `crates/shadows-core/src/plans/store/map.rs`
    - `crates/shadows-core/src/plans/store/mod.rs`
    - `crates/shadows-core/src/plans/store/notifications.rs`
    - `crates/shadows-core/src/plans/store/plan.rs`
    - `crates/shadows-core/src/plans/store/project_links.rs`
    - `crates/shadows-core/src/plans/store/read.rs`
    - `crates/shadows-core/src/plans/store/task.rs`
    - `crates/shadows-core/src/plans/store/view.rs`
    - `crates/shadows-core/src/plans/testing.rs`
    - `crates/shadows-core/src/project_lifecycle/contract.yaml`
    - `crates/shadows-core/src/project_lifecycle/mod.rs`
    - `crates/shadows-core/src/project_lifecycle/store.rs`
    - `crates/shadows-core/src/projects/contract.yaml`
    - `crates/shadows-core/src/projects/mod.rs`
    - `crates/shadows-core/src/projects/model.rs`
    - `crates/shadows-core/src/projects/store.rs`
    - `crates/shadows-core/src/providers/contract.yaml`
    - `crates/shadows-core/src/providers/error.rs`
    - `crates/shadows-core/src/providers/mod.rs`
    - `crates/shadows-core/src/providers/store.rs`
    - `crates/shadows-core/src/runtime/contract.yaml`
    - `crates/shadows-core/src/runtime/mod.rs`
    - `crates/shadows-core/src/runtime/store.rs`
    - `crates/shadows-core/src/secrets/contract.yaml`
    - `crates/shadows-core/src/secrets/mod.rs`
    - `crates/shadows-core/src/secrets/store.rs`
    - `crates/shadows-core/src/standards/contract.yaml`
    - `crates/shadows-core/src/standards/mod.rs`
    - `crates/shadows-core/src/standards/store.rs`
    - `crates/shadows-core/src/testing/acp.rs`
    - `crates/shadows-core/src/testing/mod.rs`
    - `crates/shadows-core/src/testing/turn.rs`
    - `crates/shadows-core/src/threads/attachment.rs`
    - `crates/shadows-core/src/threads/contract.yaml`
    - `crates/shadows-core/src/threads/error.rs`
    - `crates/shadows-core/src/threads/mod.rs`
    - `crates/shadows-core/src/threads/model.rs`
    - `crates/shadows-core/src/threads/store/attachment.rs`
    - `crates/shadows-core/src/threads/store/entry.rs`
    - `crates/shadows-core/src/threads/store/fork.rs`
    - `crates/shadows-core/src/threads/store/mod.rs`
    - `crates/shadows-core/src/threads/store/records.rs`
    - `crates/shadows-core/src/threads/store/remove.rs`
    - `crates/shadows-core/src/threads/store/thread.rs`
    - `crates/shadows-core/src/threads/store/title.rs`
    - `crates/shadows-core/src/threads/testing.rs`
    - `crates/shadows-core/src/turns/contract.yaml`
    - `crates/shadows-core/src/turns/mod.rs`
    - `crates/shadows-core/src/turns/model.rs`
    - `crates/shadows-core/src/turns/queue.rs`
    - `crates/shadows-core/src/turns/record.rs`
    - `crates/shadows-core/src/turns/remove.rs`
    - `crates/shadows-core/src/turns/shutdown.rs`
    - `crates/shadows-core/src/turns/spawn.rs`
    - `crates/shadows-core/src/turns/store/mod.rs`
    - `crates/shadows-core/src/turns/store/operation.rs`
    - `crates/shadows-core/src/turns/store/operation_read.rs`
    - `crates/shadows-core/src/turns/store/queue.rs`
    - `crates/shadows-core/src/turns/store/remove.rs`
    - `crates/shadows-core/src/turns/store/standards.rs`
    - `crates/shadows-core/src/turns/store/steer.rs`
    - `crates/shadows-core/src/turns/store/turn.rs`
    - `crates/shadows-core/src/turns/testing.rs`
    - `crates/shadows-core/src/turns/turn.rs`
    - `crates/shadows-core/src/usage/contract.yaml`
    - `crates/shadows-core/src/usage/mod.rs`
    - `crates/shadows-core/src/usage/report.rs`
    - `crates/shadows-core/src/usage/store.rs`
    - `crates/shadows-core/tests/aylul_extensions.rs`
    - `crates/shadows-core/tests/aylul_profiles/person.rs`
    - `crates/shadows-core/tests/aylul_profiles/probe.rs`
    - `crates/shadows-core/tests/contracts.rs`
    - `crates/shadows-core/tests/storage_contract/journal.rs`
    - `crates/shadows-core/tests/storage_contract/txn.rs`
    - `crates/shadows/tests/architecture/README.md`
    - `crates/shadows/tests/architecture/comments.allowlist`
    - `crates/shadows/tests/architecture/fixtures.rs`
    - `crates/shadows/tests/architecture/main.rs`
    - `crates/shadows/tests/architecture/ownership.allowlist`
    - `crates/shadows/tests/architecture/resolution.rs`
    - `crates/shadows/tests/architecture/scanner.rs`
    - `crates/shadows/tests/architecture/sql.rs`
    - `crates/shadows/tests/architecture/table-owners.txt`
    - `crates/shadows/tests/codemap/main.rs`
    - `crates/shadows/tests/image_attachments.rs`
    - `crates/shadows/tests/openapi/read_only.rs`
    - `crates/shadows-core/src/harness/sessions.rs`
    - `crates/shadows-core/src/turns/stage.rs`
    - `crates/shadows-core/src/turns/steer.rs`
    - `crates/shadows-core/src/turns/store/images.rs`
    - `crates/shadows-core/src/turns/store/usage.rs`
    - `crates/shadows-core/src/design/store/hierarchy.rs`
    - `crates/shadows-core/src/design/store/outcomes.rs`
    - `crates/shadows-core/src/design/store/outcome_edit.rs`
    - `crates/shadows-core/src/design/store/parts.rs`
    - `crates/shadows-core/src/design/store/part_edit.rs`
    - `crates/shadows-core/build.rs` (move to `crates/core-db/build.rs`)
    - `crates/shadows-core/tests/design_migration.rs`
    - `crates/shadows-core/tests/plan_migration.rs`
    - `crates/shadows-core/tests/profile_migration.rs`
  - Documentation: `docs/codebase/README.md`, `docs/backend/stages.md`,
    `docs/backend/plans/2026-10-21-architecture.md` (A-19b move evidence only).
  - Resolve and delete the three A-19b SQL-review allowlist records; add none.
- Needs: A-9 to A-19 preparations complete; no domain impl Storage; persistence inputs/errors have no service dependencies
- Done when: foundation and affected cross-service tests prove rollback, replay/conflict, cancellation recovery and after-commit notification; workspace targets compile; core-db has no dependency on shadows-core or a service

- Verification (2026-10-24): exact CI awk length check, fmt, workspace clippy with all
  targets/test-support, production clippy, production feature tree and unchanged `api/` passed.
  `core-db`: 20 passed; assigned Core integration binaries: 31 passed; Core units: 40 passed;
  assigned adapter binaries: 113 passed. Extra moved-migration callers: 6 passed, 1 existing ignored.
- Scope gaps fixed: grouped foundation imports, the migration build script and three migration
  test call sites are enumerated above. The event conversion is an explicit local method because
  a cross-crate `From` implementation made the private event type reachable in production.
- Allowlist: deleted all three A-19b SQL-review records and 17 foundation comment records;
  retained the 13 unrelated ownership records (three resolve to the moved Storage type),
  251 comments, nine public-function records and zero registration records; no new exemption.

- Review (2026-10-24): reproduced and fixed writer poisoning after BEGIN timeout
  using the SQLx BEGIN IMMEDIATE transaction guard; added the timeout regression.
  A pinned catalog checks all 37 embedded migration versions, descriptions and
  SHA-384 checksums against `0327292`. Corrected local CRLF bytes in migration 37
  without changing its Git blob. Eight moved tests retain their assertions;
  pinned identity test bodies match `6c4505a`. Review test counts: core-db 22;
  assigned Core command 77 passed, 1 existing ignored; assigned architecture/adapter
  command 129 passed. Moved test comments no longer carry history/spec references.

### A-19c · Extract services in the verified order
- Status: doing
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-19c--extract-services-in-the-verified-order
- Read: A-8c's runtime/type/test graph; owner spec §14.12
- Touch: controller expands this milestone into exact per-service tasks, including manifests, lockfile, re-exports, app wiring, callers, contracts, maps and tests
- Needs: A-19b; a concrete task per move in verified topological order
- Done when: each move passes affected public-API/caller tests and architecture checks; no cycles, cross-owner SQL or service-to-AppCore dependency; integration tests still run through AppCore


#### Baseline graph inspection (2026-10-24, base `e89e9c4`; superseded by the controller continuation)

Rust LSP module references, service-struct references, `CoreError`,
`DurableEvent`, `UiSignal` and `AylulOwners` references rechecked the ledger.
The table includes runtime calls and compile-time type/import edges; `error`
is the current `shadows-core/src/error.rs`, not an extracted crate.
`core-db` and existing infrastructure libraries are implicit.

| Service | Current service/type/import dependencies |
|---|---|
| `runtime` | none |
| `secrets` | `error` |
| `projects` | `error`, `events` |
| `instructions` | `error`, `projects` |
| `standards` | `error`, `projects` |
| `code` | `error`, `events`, `projects` |
| `providers` | `error`, `secrets` |
| `agents` | `error`, `projects`, `providers`, `secrets` |
| `extensions` | `agents`, `error` |
| `usage` | `error` |
| `person` | `error` |
| `threads` | `agents`, `code`, `error`, `projects` |
| `grants` | `error`, `events`, `projects`, `threads` |
| `operations` | `error`, `runtime`, `threads` |
| `harness` | `agents`, `app::AylulOwners`, `error`, `extensions`, `grants`, `instructions`, `operations`, `person`, `projects`, `providers`, `secrets`, `standards`, `threads` |
| `design` | `error`, `events`, `grants`, `harness`, `operations`, `projects`, `standards`, `threads` |
| `plans` | `code`, `design`, `error`, `events`, `grants`, `harness`, `operations`, `projects`, `threads`, `turns` |
| `turns` | `code`, `design`, `error`, `events`, `grants`, `harness`, `instructions`, `operations`, `plans`, `projects`, `runtime`, `standards`, `threads`, `usage` |
| `project-lifecycle` | `code`, `error`, `grants`, `plans`, `projects`, `threads` |
| `events` | `error`, `harness`, `plans`, `projects`, `threads`, `turns` |

Differences from A-8c:
- `core-error` does not exist. All 19 non-runtime services reference `CoreError`.
  It still wraps `turns::StartError` and `projects::DirectoryError`, and its
  conversions import `harness::OpenError`. Moving it unchanged would introduce
  upward dependencies and cycles. This is a preparation blocker, not a new
  foundation to improvise inside a service move.
- `projects`, `code`, `grants`, `design`, `plans` and `turns` still construct
  `events::DurableEvent`; `turns` also uses `events::UiSignal`. Events already
  depends on those lower owners. JournalEntry conversion did not remove these
  type/runtime cycles. Example: projects/store.rs -> events/model.rs -> projects.
- Plans imports through `turns` in store/{mod,edit,draft,view}.rs. Some imports
  are re-exports (`OperationId` is owned by operations and `InvocationView` by
  harness); these source-path edges must be redirected to their actual owners
  before treating the graph as acyclic. No domain owner changes are authorized.
- Harness takes and stores `app::AylulOwners`, a composition-owned aggregate
  containing providers, agents, extensions and person. It cannot depend on the
  composition crate after extraction; replace that edge with declared owners in
  a separately scoped preparation. This adds the person dependency absent in A-8c.
- Usage no longer depends on agents: persisted usage contains string slugs.
- Events' `OperationId` imports currently go through turns (operations owns the
  type). Harness reaches agents/providers/person through AylulOwners, rather than
  direct constructor parameters. The table preserves these actual import edges.

Test edges were checked through module/service references and references to the
`testing` module. Unit fixtures in code/{mod,store/links,store/reach}.rs,
standards/store.rs, project_lifecycle/mod.rs and turns/store/{operation_read,standards}.rs
use Core testing helpers. Those application fixtures stay in Core integration
binaries when extracting; isolated owner tests must use public constructors.
Runtime has no local tests to relocate and no Core/AppCore test dependency.
Existing recovery and operation lifecycle integration tests stay in Core.
Macro-generated IDs are not addressable by Serena find_referencing_symbols;
module references and their owning macro declarations establish their provenance.

**Baseline result at e89e9c4: runtime only.** That inspection found cycles
and blocked the remaining moves. The controller continuation and its verified
order below supersede those blocked statuses.

Each move runs the dispatch's line-length, fmt, all-target clippy, owner tests,
Core lib tests, architecture/codemap/openapi tests and its named integration
binaries. Final checks also run production clippy, production feature isolation,
all Core binaries and the unchanged-api diff. Never run workspace-wide tests.

Controller continuation: the preparation tasks above supersede the prior blocked
statuses. After their boundary changes, the service order is runtime (done),
secrets, projects, instructions, standards, code, providers, agents, extensions,
usage, person, threads, grants, operations, harness, design, plans, turns,
project-lifecycle, events. Error splits accompany the corresponding moves rather
than requiring an all-service prerequisite commit. Recheck this order with Rust
LSP after preparation; a new edge is reported, never hidden by a facade.

#### Verified continuation order after A-19c.0a and A-19c.0b

Direct Rust LSP references (test-support enabled) rechecked all service module
edges, the relocated signal and the Harness bundle. The graph below includes
runtime and type-only owner edges; core-db and infrastructure are implicit.
CoreError references remain in unextracted services and are removed by their
own A-19c.0 error split in the corresponding extraction commit. Owner-local
fixtures must also shed Core testing dependencies during those moves; application
fixtures remain Core integration tests. This is the extraction dependency graph,
not a claim that every unextracted error/fixture is already prepared.

| Move | Service | Earlier owner dependencies |
|---|---|---|
| A-19c.1 | runtime | none |
| A-19c.2 | secrets | none |
| A-19c.3 | projects | none |
| A-19c.4 | instructions | projects |
| A-19c.5 | standards | projects |
| A-19c.6 | code | projects |
| A-19c.7 | providers | secrets |
| A-19c.8 | agents | projects, providers, secrets |
| A-19c.9 | extensions | agents |
| A-19c.10 | usage | none |
| A-19c.11 | person | none |
| A-19c.12 | threads | agents, code, projects |
| A-19c.13 | grants | projects, threads |
| A-19c.14 | operations | runtime, threads |
| A-19c.15 | harness | agents, extensions, grants, instructions, operations, person, projects, providers, secrets, standards, threads |
| A-19c.16 | design | grants, harness, operations, projects, standards, threads |
| A-19c.17 | plans | code, design, grants, harness, operations, projects, threads |
| A-19c.18 | turns | code, design, grants, harness, instructions, operations, plans, projects, runtime, standards, threads, usage |
| A-19c.19 | project-lifecycle | code, grants, plans, projects, threads |
| A-19c.20 | events | harness, operations, plans, projects, threads, turns |

The graph is acyclic in this order. No publisher imports Events; Harness does
not import app; Plans does not import Turns. Usage has no agents dependency;
Harness includes person. Runtime, Secrets and Providers are extracted. The controller has unblocked Projects comment-path relocation. The continuation order is Projects, Usage, Person, Instructions, Standards, Code, Agents, Extensions, Threads, Grants, Operations, Harness, Design, Plans, Turns, ProjectLifecycle, Events; the verified dependency edges do not change.

#### A-19c.0 · Give each service its own error
- Status: doing
- Progress: Secrets, Providers and Projects error splits are complete in A-19c.2, A-19c.7 and A-19c.3. Remaining owner error splits accompany their moves; Runtime still needs its owned wrapper.
- Ruling: controller continuation at 9b3735e supersedes the error-vocabulary blocker; no shared error crate. Apply each owner's split in its extraction commit, lowest first. Runtime currently exposes only StorageError; its owned wrapper remains part of A-19c.0 completion.
- Touch: each service's error.rs, model and methods; its declared callers and tests; shadows-core/src/error.rs From conversions and public error re-exports; adapter translation conversions and failure-case tests where a service error crosses the application boundary; affected contracts and code-map rows; manifests and Cargo.lock. No route documentation or wire shape change.
- Done when: no service names CoreError; each owns only its failures plus Storage where applicable and declared callee errors; existing HTTP status/code/message and MCP failure tests pass; api/ is unchanged.

#### A-19c.0a · Remove the event-vocabulary cycles
- Status: done
- Evidence: core-db 23 passed; all Core binaries 324 passed, three existing ignored; architecture 27, codemap 1, openapi 7, resync 8 and stream_frames 1 passed. Both workspace clippy modes, fmt, exact CI line length, LF and unchanged api checks pass; production feature grep has no matches. UUID moved from dev-only to production infrastructure in core-db, with no new external package. No allowlist line changed.
- Small gap fixed: the Design agreement pointed at events::with_project; it now names db::with_project. Contract lookup resolves extracted owners with exact-case directory matching and an adversarial case regression. A new snapshot pins every plan-show place spelling, field order and nulls.
- Needs: A-19c.1
- Ruling: use core-db JournalEntry directly for durable publication; each service converts its ids to strings. Move the unchanged UiSignal value to Plans. The owner plan records the decision.
- Touch: crates/core-db/src/journal.rs and its contract; the old events/model.rs vocabulary and events re-exports; Plans model/conversation and contract; every existing DurableEvent/UiSignal publisher, reader and test-support conversion in Core; their contracts; Core public re-exports; docs/codebase/README.md; stages.md; architecture checks and existing allowlist deletions only. No publisher payload, row or append-order changes.
- Done when: core-db tests, Core lib and affected storage tests, stream_frames and resync pass; architecture/codemap/openapi and both clippy modes pass; no lower publisher imports events; serialized signal and stored journal bytes retain their shape and order.

#### A-19c.0b · Let Harness declare its configuration dependencies
- Status: done
- Evidence: Rust LSP module references confirm Plans no longer imports Turns and Harness declares agents/providers/extensions/person directly. Core lib plus sessions, aylul_session, aylul_profiles, workflow_rules and contracts: 143 passed, one existing live-key probe ignored. Architecture 27, codemap 1 and openapi 7 passed; all-target workspace clippy, fmt, exact line length, LF and api diff pass. No allowlist line changed.
- Needs: A-19c.1
- Touch: harness/owners.rs, harness/{mod,access,sessions}.rs and contract.yaml; app.rs and testing/acp.rs; Plans store/{mod,edit,draft,view}.rs imports and contract; docs/codebase/README.md; stages.md; architecture allowlist deletions only.
- Done when: AylulOwners is declared by Harness, its build remains AppCore composition, Harness imports nothing from app; Plans imports OperationId and InvocationView from their owner; affected harness/session/plan tests and architecture checks pass.

#### A-19c.1 · Extract core-runtime
- Status: done
- Caller gap: the fixed private-field shape replaces Runtime.instance_id field reads with instance_id(); LSP identified Core turns/record.rs plus the harness_schema and aylul_usage Core fixtures and adapter test binaries core_harness, core_turns, planner_turn, protocol, resync, shutdown and stream_frames. These mechanical test call sites are included under the dispatch standing rule; product adapter source and serialized types do not change.
- Needs: A-19b
- Touch: `crates/core-runtime/` (`Cargo.toml`, `src/lib.rs`, `src/model.rs`, `src/store/`, `contract.yaml`, `tests/`); old `crates/shadows-core/src/runtime/`; `crates/shadows-core/src/{lib,app,testing/**}.rs`; `Cargo.toml`, `Cargo.lock`, `crates/shadows-core/Cargo.toml`; caller imports in `crates/shadows-core/src/operations/model.rs`, `crates/shadows-core/src/operations/store/lifecycle.rs`, `crates/shadows-core/src/operations/store/read.rs`, `crates/shadows-core/src/operations/store/recovery.rs`, `crates/shadows-core/src/operations/store/runtime_stop.rs`, `crates/shadows-core/src/operations/testing.rs`, `crates/shadows-core/src/turns/mod.rs`, `crates/shadows-core/src/turns/model.rs`, `crates/shadows-core/src/turns/shutdown.rs`, `crates/shadows-core/src/turns/spawn.rs`, `crates/shadows-core/src/turns/store/operation.rs`, `crates/shadows-core/src/turns/store/turn.rs`, `crates/shadows-core/src/turns/testing.rs`; those callers' owner contracts; moved owner-local tests and their old registrations; `crates/shadows-core/tests/contracts.rs`; `docs/codebase/README.md`; `docs/backend/stages.md`; `crates/shadows/tests/architecture/{scanner,resolution,sql,fixtures,main}.rs`, `table-owners.txt` and allowlists (delete lines only, except exact moved-file comment path relocations); `crates/shadows/tests/codemap/main.rs`. AppCore integration tests remain in Core.
- Done when: `cargo test -p core-runtime`, `cargo test -p shadows-core --lib` and Core integration binaries `operation_lifecycle`, `recovery` pass; affected callers and architecture/codemap/openapi checks pass; no Cargo cycle, cross-owner SQL, dependency on `shadows-core` or `AppCore`; stores are private; adapter re-exports and schemas stay stable.

Runtime move verification (2026-10-24): core-runtime's three public-constructor
integration tests pass. Core lib: 40 passed. Full Core: 322 passed across 32
binaries, three existing tests ignored (live extension acceptance, live provider
keys, manual migration copy). Included operation_lifecycle 12, recovery 6,
harness_schema 5 and aylul_usage 6. Architecture 27, codemap 1 and openapi 7
passed. Affected adapter binaries: core_harness 5, core_turns 8, planner_turn 10,
protocol 8, resync 8, shutdown 6 and stream_frames 1: all 46 passed.

The exact CI awk line-length check, fmt, workspace all-target/test-support
clippy and production clippy pass. Production feature grep prints nothing;
api diff, whitespace and LF checks are clean. No allowlist lines were added,
renamed or deleted. No new external dependency: the moved crate reuses core-db
and existing serde/serde_json/sqlx/uuid/utoipa infrastructure. `from_recorded`
preserves the stored runtime-id text without introducing validation; Operations
uses it where the id macro's crate-private constructor no longer crosses the
crate boundary. The store remains private. Runtime's instance_id is private
with a borrowing accessor; all LSP field callers were mechanically updated.
Product adapter source, schema and system event semantics are unchanged.

At 9b3735e, A-19c remained blocked: all remaining moves required the error-vocabulary
preparation, and the additional cyclic edges above remain. No independent
service remains to extract. Existing CLAUDE.md crate-table wording that SQLite
lives only in shadows-core is outside this task's Touch. These checks are local
automation; no browser/native/provider/production acceptance was performed.

#### A-19c.2 · Extract core-secrets
- Status: done
- Evidence: owner package 19 passed (13 public-constructor persistence tests and six pure crypto invariants retained under tests/); Core lib 34 passed; all Core binaries 306 passed, three existing ignored. Architecture 29, codemap 1 and openapi 7 passed; HTTP lib 24 and MCP lib 1 passed, including the full application error mapping cases. The final compatibility-validator/opening-error test and Core lib passed 35 tests together. Both clippy modes, fmt, exact line length, LF, api diff and production feature isolation pass.
- Error split: SecretsError is defined only by core-secrets, and CoreError converts every variant. Harness wraps SecretsError transparently on opening; the existing public validate_env_name API remains a thin application error translator over the sole Secrets validation method. No new external dependency; crypto, serde, SQLx and supporting infrastructure keep their existing versions.
- Ratchet: deleted four free-function records, one crypto-test glob record and the old fixture-constructor bypass record; zero entries added or renamed. Validators and crypto helpers are owner methods. Store construction is classified only in the actual service entry module; fixtures prove other services cannot construct Secrets, adapters cannot resolve plaintext or reach its store, and cross-owner SQL remains rejected. Full-call classification does not mistake error variants named Storage for live registry access.
- Small caller gap: the direct Secrets assertion in aylul_profiles/agents.rs now matches SecretsError; its previous CoreError match was not the error returned by the extracted service.
- Needs: A-19c.0 error split in this move; A-19c.0a and A-19c.0b; A-19b
- Touch: `crates/core-secrets/` (`Cargo.toml`, `src/lib.rs`, `src/model.rs`, `src/store/`, `contract.yaml`, `tests/`); old `crates/shadows-core/src/secrets/`; `crates/shadows-core/src/{lib,app,testing/**}.rs`; `Cargo.toml`, `Cargo.lock`, `crates/shadows-core/Cargo.toml`; caller imports in `crates/shadows-core/src/agents/mod.rs`, `crates/shadows-core/src/harness/profile_home.rs`, `crates/shadows-core/src/harness/sessions.rs`, `crates/shadows-core/src/providers/mod.rs`; those callers' owner contracts; moved owner-local tests and their old registrations; `crates/shadows-core/tests/contracts.rs`; `docs/codebase/README.md`; `docs/backend/stages.md`; `crates/shadows/tests/architecture/{scanner,resolution,sql,fixtures,main}.rs`, `table-owners.txt` and allowlists (delete lines only, except exact moved-file comment path relocations); `crates/shadows/tests/codemap/main.rs`. AppCore integration tests remain in Core.
- Done when: `cargo test -p core-secrets`, `cargo test -p shadows-core --lib` and Core integration binaries `secret_env` pass; affected callers and architecture/codemap/openapi checks pass; no Cargo cycle, cross-owner SQL, dependency on `shadows-core` or `AppCore`; stores are private; adapter re-exports and schemas stay stable.

#### A-19c.3 · Extract core-projects
- Status: done
- Ruling: Controller permits one-for-one relocation of moved-file comment records, preserving text and count. API documentation moves unchanged. This applies to all subsequent moves.
- Caller gap: grouped imports in Design and Turns plus the Design agreement row mapper now import the project owner directly. The HTTP failure bridge accepts ProjectsError through CoreError without changing responses.
- Relocated comment records: projects/model.rs to core-projects/src/model.rs: Project [81088cef37f25415] #1; Project [f62a4ea16ef252ea] #1; module [2e3e8754146bae4f] #1; module [80f2a97e1598e288] #1. Each retains its exact text and count.
- Evidence: core-projects 8 passed; Core lib 29 passed; all Core binaries 296 passed with three existing ignores, including project_assistant 2 and project_contract 1. Architecture 31, codemap 1, openapi 7, HTTP lib 24 and MCP lib 1 passed. Both workspace clippy modes, fmt, exact CI line length, LF and unchanged api checks pass. Production feature grep prints nothing.
- Ratchet: six records deleted (browse blocking/create_subdirectory/list, directory canonical_dir/utf8, testing projects constructor); exactly four comment records relocated above. Transaction-contract coverage now includes public owner methods implemented within private store modules, held by its regression test.
- Dependencies: no new external package; reuses core-db and existing shadows-agent policy, dunce, serde/serde_json, SQLx, Tokio, tracing, UUID and utoipa infrastructure.
- LSP: installed rust-analyzer JSON-RPC references rechecked Projects, its constructor and its module callers; 440 workspace packages loaded. Projects has no lower service edge or AppCore/test-fixture dependency after this move.
- Needs: A-19c.0 error split in this move; A-19c.0a and A-19c.0b; A-19b
- Touch: `crates/core-projects/` (`Cargo.toml`, `src/lib.rs`, `src/model.rs`, `src/store/`, `contract.yaml`, `tests/`); old `crates/shadows-core/src/projects/`; `crates/shadows-core/src/{lib,app,testing/**}.rs`; `Cargo.toml`, `Cargo.lock`, `crates/shadows-core/Cargo.toml`; caller imports in `crates/shadows-core/src/agents/mod.rs`, `crates/shadows-core/src/code/active.rs`, `crates/shadows-core/src/code/links.rs`, `crates/shadows-core/src/code/mod.rs`, `crates/shadows-core/src/code/scan.rs`, `crates/shadows-core/src/code/scope.rs`, `crates/shadows-core/src/code/store/links.rs`, `crates/shadows-core/src/code/store/mod.rs`, `crates/shadows-core/src/code/store/reach.rs`, `crates/shadows-core/src/code/watch.rs`, `crates/shadows-core/src/design/mod.rs`, `crates/shadows-core/src/design/store/parts.rs`, `crates/shadows-core/src/design/store/proposal_read.rs`, `crates/shadows-core/src/design/store/stage.rs`, `crates/shadows-core/src/design/store/structure.rs`, `crates/shadows-core/src/design/store/vision.rs`, `crates/shadows-core/src/error.rs`, `crates/shadows-core/src/events/mod.rs`, `crates/shadows-core/src/events/model.rs`, `crates/shadows-core/src/events/store.rs`, `crates/shadows-core/src/events/subscription.rs`, `crates/shadows-core/src/grants/mod.rs`, `crates/shadows-core/src/grants/model.rs`, `crates/shadows-core/src/grants/store.rs`, `crates/shadows-core/src/grants/testing.rs`, `crates/shadows-core/src/harness/mod.rs`, `crates/shadows-core/src/harness/setup.rs`, `crates/shadows-core/src/harness/store/mode.rs`, `crates/shadows-core/src/instructions/mod.rs`, `crates/shadows-core/src/instructions/store.rs`, `crates/shadows-core/src/plans/dependencies.rs`, `crates/shadows-core/src/plans/map.rs`, `crates/shadows-core/src/plans/mod.rs`, `crates/shadows-core/src/plans/model.rs`, `crates/shadows-core/src/plans/project_links.rs`, `crates/shadows-core/src/plans/scope.rs`, `crates/shadows-core/src/plans/store/draft.rs`, `crates/shadows-core/src/plans/store/edit.rs`, `crates/shadows-core/src/plans/store/graph.rs`, `crates/shadows-core/src/plans/store/linked.rs`, `crates/shadows-core/src/plans/store/map.rs`, `crates/shadows-core/src/plans/store/mod.rs`, `crates/shadows-core/src/plans/store/notifications.rs`, `crates/shadows-core/src/plans/store/plan.rs`, `crates/shadows-core/src/plans/store/project_links.rs`, `crates/shadows-core/src/plans/store/read.rs`, `crates/shadows-core/src/plans/testing.rs`, `crates/shadows-core/src/project_lifecycle/mod.rs`, `crates/shadows-core/src/project_lifecycle/store.rs`, `crates/shadows-core/src/standards/store.rs`, `crates/shadows-core/src/threads/mod.rs`, `crates/shadows-core/src/threads/model.rs`, `crates/shadows-core/src/threads/store/records.rs`, `crates/shadows-core/src/threads/store/remove.rs`, `crates/shadows-core/src/threads/store/thread.rs`, `crates/shadows-core/src/threads/testing.rs`, `crates/shadows-core/src/turns/mod.rs`, `crates/shadows-core/src/turns/store/standards.rs`; those callers' owner contracts; moved owner-local tests and their old registrations; `crates/shadows-core/tests/contracts.rs`; `docs/codebase/README.md`; `docs/backend/stages.md`; `crates/shadows/tests/architecture/{scanner,resolution,sql,fixtures,main}.rs`, `table-owners.txt` and allowlists (delete lines only, except exact moved-file comment path relocations); `crates/shadows/tests/codemap/main.rs`. AppCore integration tests remain in Core.
- Done when: `cargo test -p core-projects`, `cargo test -p shadows-core --lib` and Core integration binaries `project_assistant`, `project_contract` pass; affected callers and architecture/codemap/openapi checks pass; no Cargo cycle, cross-owner SQL, dependency on `shadows-core` or `AppCore`; stores are private; adapter re-exports and schemas stay stable.

#### A-19c.4 · Extract core-instructions
- Status: todo
- Needs: A-19c.0 error split in this move; A-19c.0a and A-19c.0b; A-19b ; A-19c.3
- Touch: `crates/core-instructions/` (`Cargo.toml`, `src/lib.rs`, `src/model.rs`, `src/store/`, `contract.yaml`, `tests/`); old `crates/shadows-core/src/instructions/`; `crates/shadows-core/src/{lib,app,testing/**}.rs`; `Cargo.toml`, `Cargo.lock`, `crates/shadows-core/Cargo.toml`; caller imports in `crates/shadows-core/src/harness/mod.rs`, `crates/shadows-core/src/harness/sessions.rs`, `crates/shadows-core/src/harness/setup.rs`, `crates/shadows-core/src/turns/mod.rs`; those callers' owner contracts; moved owner-local tests and their old registrations; `crates/shadows-core/tests/contracts.rs`; `docs/codebase/README.md`; `docs/backend/stages.md`; `crates/shadows/tests/architecture/{scanner,resolution,sql,fixtures,main}.rs`, `table-owners.txt` and allowlists (delete lines only, except exact moved-file comment path relocations); `crates/shadows/tests/codemap/main.rs`. AppCore integration tests remain in Core.
- Done when: `cargo test -p core-instructions`, `cargo test -p shadows-core --lib` and Core integration binaries `workflow_rules` pass; affected callers and architecture/codemap/openapi checks pass; no Cargo cycle, cross-owner SQL, dependency on `shadows-core` or `AppCore`; stores are private; adapter re-exports and schemas stay stable.

#### A-19c.5 · Extract core-standards
- Status: todo
- Needs: A-19c.0 error split in this move; A-19c.0a and A-19c.0b; A-19b ; A-19c.3
- Touch: `crates/core-standards/` (`Cargo.toml`, `src/lib.rs`, `src/model.rs`, `src/store/`, `contract.yaml`, `tests/`); old `crates/shadows-core/src/standards/`; `crates/shadows-core/src/{lib,app,testing/**}.rs`; `Cargo.toml`, `Cargo.lock`, `crates/shadows-core/Cargo.toml`; caller imports in `crates/shadows-core/src/design/map_rules.rs`, `crates/shadows-core/src/design/store/map_proposal_shape.rs`, `crates/shadows-core/src/design/store/map_versions.rs`, `crates/shadows-core/src/design/store/proposals.rs`, `crates/shadows-core/src/design/store/stage.rs`, `crates/shadows-core/src/harness/setup.rs`, `crates/shadows-core/src/harness/standards.rs`, `crates/shadows-core/src/turns/mod.rs`, `crates/shadows-core/src/turns/record.rs`, `crates/shadows-core/src/turns/store/standards.rs`; those callers' owner contracts; moved owner-local tests and their old registrations; `crates/shadows-core/tests/contracts.rs`; `docs/codebase/README.md`; `docs/backend/stages.md`; `crates/shadows/tests/architecture/{scanner,resolution,sql,fixtures,main}.rs`, `table-owners.txt` and allowlists (delete lines only, except exact moved-file comment path relocations); `crates/shadows/tests/codemap/main.rs`. AppCore integration tests remain in Core.
- Done when: `cargo test -p core-standards`, `cargo test -p shadows-core --lib` and Core integration binaries `design_vision` pass; affected callers and architecture/codemap/openapi checks pass; no Cargo cycle, cross-owner SQL, dependency on `shadows-core` or `AppCore`; stores are private; adapter re-exports and schemas stay stable.

#### A-19c.6 · Extract core-code
- Status: todo
- Needs: A-19c.0 error split in this move; A-19c.0a and A-19c.0b; A-19b ; A-19c.3
- Touch: `crates/core-code/` (`Cargo.toml`, `src/lib.rs`, `src/model.rs`, `src/store/`, `contract.yaml`, `tests/`); old `crates/shadows-core/src/code/`; `crates/shadows-core/src/{lib,app,testing/**}.rs`; `Cargo.toml`, `Cargo.lock`, `crates/shadows-core/Cargo.toml`; caller imports in `crates/shadows-core/src/plans/project_links.rs`, `crates/shadows-core/src/plans/store/graph.rs`, `crates/shadows-core/src/plans/store/linked.rs`, `crates/shadows-core/src/plans/store/map.rs`, `crates/shadows-core/src/plans/store/project_links.rs`, `crates/shadows-core/src/plans/store/read.rs`, `crates/shadows-core/src/project_lifecycle/mod.rs`, `crates/shadows-core/src/project_lifecycle/store.rs`, `crates/shadows-core/src/threads/mod.rs`, `crates/shadows-core/src/turns/mod.rs`; those callers' owner contracts; moved owner-local tests and their old registrations; `crates/shadows-core/tests/contracts.rs`; `docs/codebase/README.md`; `docs/backend/stages.md`; `crates/shadows/tests/architecture/{scanner,resolution,sql,fixtures,main}.rs`, `table-owners.txt` and allowlists (delete lines only, except exact moved-file comment path relocations); `crates/shadows/tests/codemap/main.rs`. AppCore integration tests remain in Core.
- Done when: `cargo test -p core-code`, `cargo test -p shadows-core --lib` and Core integration binaries `code_index` pass; affected callers and architecture/codemap/openapi checks pass; no Cargo cycle, cross-owner SQL, dependency on `shadows-core` or `AppCore`; stores are private; adapter re-exports and schemas stay stable.

#### A-19c.7 · Extract core-providers
- Status: done
- Evidence: owner package 7 passed (five unchanged listing invariants under tests/ and two public-constructor persistence tests); Core lib 29 passed; all Core binaries 302 passed, three existing ignored. Architecture 30, codemap 1, openapi 7, HTTP lib 24 and MCP lib 1 passed. Both workspace clippy modes, fmt, exact line length, LF, unchanged api and production feature isolation pass. No allowlist line changed.
- Error split: ProvidersError owns provider validation, missing-provider/key, Storage and declared Secrets causes. CoreError performs the application conversion; HTTP translates through that conversion and Harness wraps the owner error transparently. All status/code/message and provider-in-use tests remain green.
- Infrastructure: reuse existing reqwest, Tokio (the bounded probe timeout), SQLx, serde and supporting packages; add no external package or shared error crate. Public transaction methods use core-db Tx and the store stays private.
- Small gaps fixed: validated is the same shared rule Agents calls and is public across the crate boundary; four direct provider/probe assertions now match the owner error. Pure listing tests moved physically without removing assertions; AppCore integration fixtures remain in Core.
- Execution order: independent of the blocked Projects move; only Secrets and core-db are prerequisites.
- Needs: A-19c.0 error split in this move; A-19c.0a and A-19c.0b; A-19b ; A-19c.2
- Touch: `crates/core-providers/` (`Cargo.toml`, `src/lib.rs`, `src/model.rs`, `src/store/`, `contract.yaml`, `tests/`); old `crates/shadows-core/src/providers/`; `crates/shadows-core/src/{lib,app,testing/**}.rs`; `Cargo.toml`, `Cargo.lock`, `crates/shadows-core/Cargo.toml`; caller imports in `crates/shadows-core/src/agents/mod.rs`, `crates/shadows-core/src/agents/store.rs`; those callers' owner contracts; moved owner-local tests and their old registrations; `crates/shadows-core/tests/contracts.rs`; `docs/codebase/README.md`; `docs/backend/stages.md`; `crates/shadows/tests/architecture/{scanner,resolution,sql,fixtures,main}.rs`, `table-owners.txt` and allowlists (delete lines only, except exact moved-file comment path relocations); `crates/shadows/tests/codemap/main.rs`. AppCore integration tests remain in Core.
- Done when: `cargo test -p core-providers`, `cargo test -p shadows-core --lib` and Core integration binaries `aylul_profiles` pass; affected callers and architecture/codemap/openapi checks pass; no Cargo cycle, cross-owner SQL, dependency on `shadows-core` or `AppCore`; stores are private; adapter re-exports and schemas stay stable.

#### A-19c.8 · Extract core-agents
- Status: todo
- Needs: A-19c.0 error split in this move; A-19c.0a and A-19c.0b; A-19b ; A-19c.2, A-19c.3, A-19c.7
- Touch: `crates/core-agents/` (`Cargo.toml`, `src/lib.rs`, `src/model.rs`, `src/store/`, `contract.yaml`, `tests/`); old `crates/shadows-core/src/agents/`; `crates/shadows-core/src/{lib,app,testing/**}.rs`; `Cargo.toml`, `Cargo.lock`, `crates/shadows-core/Cargo.toml`; caller imports in `crates/shadows-core/src/extensions/error.rs`, `crates/shadows-core/src/extensions/store.rs`, `crates/shadows-core/src/threads/store/thread.rs`; those callers' owner contracts; moved owner-local tests and their old registrations; `crates/shadows-core/tests/contracts.rs`; `docs/codebase/README.md`; `docs/backend/stages.md`; `crates/shadows/tests/architecture/{scanner,resolution,sql,fixtures,main}.rs`, `table-owners.txt` and allowlists (delete lines only, except exact moved-file comment path relocations); `crates/shadows/tests/codemap/main.rs`. AppCore integration tests remain in Core.
- Done when: `cargo test -p core-agents`, `cargo test -p shadows-core --lib` and Core integration binaries `aylul_profiles`, `aylul_session` pass; affected callers and architecture/codemap/openapi checks pass; no Cargo cycle, cross-owner SQL, dependency on `shadows-core` or `AppCore`; stores are private; adapter re-exports and schemas stay stable.

#### A-19c.9 · Extract core-extensions
- Status: todo
- Needs: A-19c.0 error split in this move; A-19c.0a and A-19c.0b; A-19b ; A-19c.8
- Touch: `crates/core-extensions/` (`Cargo.toml`, `src/lib.rs`, `src/model.rs`, `src/store/`, `contract.yaml`, `tests/`); old `crates/shadows-core/src/extensions/`; `crates/shadows-core/src/{lib,app,testing/**}.rs`; `Cargo.toml`, `Cargo.lock`, `crates/shadows-core/Cargo.toml`; caller imports in `crates/shadows-core/src/harness/extension_home.rs`, `crates/shadows-core/src/harness/mod.rs`; those callers' owner contracts; moved owner-local tests and their old registrations; `crates/shadows-core/tests/contracts.rs`; `docs/codebase/README.md`; `docs/backend/stages.md`; `crates/shadows/tests/architecture/{scanner,resolution,sql,fixtures,main}.rs`, `table-owners.txt` and allowlists (delete lines only, except exact moved-file comment path relocations); `crates/shadows/tests/codemap/main.rs`. AppCore integration tests remain in Core.
- Done when: `cargo test -p core-extensions`, `cargo test -p shadows-core --lib` and Core integration binaries `aylul_extensions` pass; affected callers and architecture/codemap/openapi checks pass; no Cargo cycle, cross-owner SQL, dependency on `shadows-core` or `AppCore`; stores are private; adapter re-exports and schemas stay stable.

#### A-19c.10 · Extract core-usage
- Status: todo
- Needs: A-19c.0 error split in this move; A-19c.0a and A-19c.0b; A-19b
- Touch: `crates/core-usage/` (`Cargo.toml`, `src/lib.rs`, `src/model.rs`, `src/store/`, `contract.yaml`, `tests/`); old `crates/shadows-core/src/usage/`; `crates/shadows-core/src/{lib,app,testing/**}.rs`; `Cargo.toml`, `Cargo.lock`, `crates/shadows-core/Cargo.toml`; caller imports in `crates/shadows-core/src/turns/store/usage.rs`; those callers' owner contracts; moved owner-local tests and their old registrations; `crates/shadows-core/tests/contracts.rs`; `docs/codebase/README.md`; `docs/backend/stages.md`; `crates/shadows/tests/architecture/{scanner,resolution,sql,fixtures,main}.rs`, `table-owners.txt` and allowlists (delete lines only, except exact moved-file comment path relocations); `crates/shadows/tests/codemap/main.rs`. AppCore integration tests remain in Core.
- Done when: `cargo test -p core-usage`, `cargo test -p shadows-core --lib` and Core integration binaries `aylul_usage` pass; affected callers and architecture/codemap/openapi checks pass; no Cargo cycle, cross-owner SQL, dependency on `shadows-core` or `AppCore`; stores are private; adapter re-exports and schemas stay stable.

#### A-19c.11 · Extract core-person
- Status: todo
- Needs: A-19c.0 error split in this move; A-19c.0a and A-19c.0b; A-19b
- Touch: `crates/core-person/` (`Cargo.toml`, `src/lib.rs`, `src/model.rs`, `src/store/`, `contract.yaml`, `tests/`); old `crates/shadows-core/src/person/`; `crates/shadows-core/src/{lib,app,testing/**}.rs`; `Cargo.toml`, `Cargo.lock`, `crates/shadows-core/Cargo.toml`; caller imports in (no other source caller); those callers' owner contracts; moved owner-local tests and their old registrations; `crates/shadows-core/tests/contracts.rs`; `docs/codebase/README.md`; `docs/backend/stages.md`; `crates/shadows/tests/architecture/{scanner,resolution,sql,fixtures,main}.rs`, `table-owners.txt` and allowlists (delete lines only, except exact moved-file comment path relocations); `crates/shadows/tests/codemap/main.rs`. AppCore integration tests remain in Core.
- Done when: `cargo test -p core-person`, `cargo test -p shadows-core --lib` and Core integration binaries `aylul_profiles` pass; affected callers and architecture/codemap/openapi checks pass; no Cargo cycle, cross-owner SQL, dependency on `shadows-core` or `AppCore`; stores are private; adapter re-exports and schemas stay stable.

#### A-19c.12 · Extract core-threads
- Status: todo
- Needs: A-19c.0 error split in this move; A-19c.0a and A-19c.0b; A-19b ; A-19c.3, A-19c.6, A-19c.8
- Touch: `crates/core-threads/` (`Cargo.toml`, `src/lib.rs`, `src/model.rs`, `src/store/`, `contract.yaml`, `tests/`); old `crates/shadows-core/src/threads/`; `crates/shadows-core/src/{lib,app,testing/**}.rs`; `Cargo.toml`, `Cargo.lock`, `crates/shadows-core/Cargo.toml`; caller imports in `crates/shadows-core/src/design/model.rs`, `crates/shadows-core/src/design/proposals.rs`, `crates/shadows-core/src/design/store/map_proposals.rs`, `crates/shadows-core/src/design/store/proposals.rs`, `crates/shadows-core/src/design/store/roadmap_proposals.rs`, `crates/shadows-core/src/design/vision_items.rs`, `crates/shadows-core/src/events/mod.rs`, `crates/shadows-core/src/events/model.rs`, `crates/shadows-core/src/events/store.rs`, `crates/shadows-core/src/events/subscription.rs`, `crates/shadows-core/src/grants/approval.rs`, `crates/shadows-core/src/grants/approver.rs`, `crates/shadows-core/src/grants/mod.rs`, `crates/shadows-core/src/grants/model.rs`, `crates/shadows-core/src/grants/store.rs`, `crates/shadows-core/src/grants/testing.rs`, `crates/shadows-core/src/grants/writer.rs`, `crates/shadows-core/src/harness/access.rs`, `crates/shadows-core/src/harness/assistant.rs`, `crates/shadows-core/src/harness/commands.rs`, `crates/shadows-core/src/harness/context.rs`, `crates/shadows-core/src/harness/fork.rs`, `crates/shadows-core/src/harness/live.rs`, `crates/shadows-core/src/harness/mod.rs`, `crates/shadows-core/src/harness/mode.rs`, `crates/shadows-core/src/harness/offers.rs`, `crates/shadows-core/src/harness/sessions.rs`, `crates/shadows-core/src/harness/settings.rs`, `crates/shadows-core/src/harness/setup.rs`, `crates/shadows-core/src/harness/store/invocations.rs`, `crates/shadows-core/src/harness/store/mode.rs`, `crates/shadows-core/src/harness/store/thread.rs`, `crates/shadows-core/src/harness/titles.rs`, `crates/shadows-core/src/operations/fork.rs`, `crates/shadows-core/src/operations/mod.rs`, `crates/shadows-core/src/operations/store/checks.rs`, `crates/shadows-core/src/operations/store/lifecycle.rs`, `crates/shadows-core/src/operations/store/read.rs`, `crates/shadows-core/src/operations/store/transition.rs`, `crates/shadows-core/src/plans/conversation.rs`, `crates/shadows-core/src/plans/mod.rs`, `crates/shadows-core/src/plans/model.rs`, `crates/shadows-core/src/plans/scope.rs`, `crates/shadows-core/src/plans/store/edit.rs`, `crates/shadows-core/src/plans/store/read.rs`, `crates/shadows-core/src/plans/store/view.rs`, `crates/shadows-core/src/plans/testing.rs`, `crates/shadows-core/src/project_lifecycle/store.rs`, `crates/shadows-core/src/turns/handles.rs`, `crates/shadows-core/src/turns/mod.rs`, `crates/shadows-core/src/turns/model.rs`, `crates/shadows-core/src/turns/queue.rs`, `crates/shadows-core/src/turns/record.rs`, `crates/shadows-core/src/turns/remove.rs`, `crates/shadows-core/src/turns/spawn.rs`, `crates/shadows-core/src/turns/stage.rs`, `crates/shadows-core/src/turns/steer.rs`, `crates/shadows-core/src/turns/store/images.rs`, `crates/shadows-core/src/turns/store/operation.rs`, `crates/shadows-core/src/turns/store/operation_read.rs`, `crates/shadows-core/src/turns/store/queue.rs`, `crates/shadows-core/src/turns/store/remove.rs`, `crates/shadows-core/src/turns/store/standards.rs`, `crates/shadows-core/src/turns/store/steer.rs`, `crates/shadows-core/src/turns/store/turn.rs`, `crates/shadows-core/src/turns/testing.rs`, `crates/shadows-core/src/turns/turn.rs`; those callers' owner contracts; moved owner-local tests and their old registrations; `crates/shadows-core/tests/contracts.rs`; `docs/codebase/README.md`; `docs/backend/stages.md`; `crates/shadows/tests/architecture/{scanner,resolution,sql,fixtures,main}.rs`, `table-owners.txt` and allowlists (delete lines only, except exact moved-file comment path relocations); `crates/shadows/tests/codemap/main.rs`. AppCore integration tests remain in Core.
- Done when: `cargo test -p core-threads`, `cargo test -p shadows-core --lib` and Core integration binaries `thread_session` pass; affected callers and architecture/codemap/openapi checks pass; no Cargo cycle, cross-owner SQL, dependency on `shadows-core` or `AppCore`; stores are private; adapter re-exports and schemas stay stable.

#### A-19c.13 · Extract core-grants
- Status: todo
- Needs: A-19c.0 error split in this move; A-19c.0a and A-19c.0b; A-19b ; A-19c.3, A-19c.12
- Touch: `crates/core-grants/` (`Cargo.toml`, `src/lib.rs`, `src/model.rs`, `src/store/`, `contract.yaml`, `tests/`); old `crates/shadows-core/src/grants/`; `crates/shadows-core/src/{lib,app,testing/**}.rs`; `Cargo.toml`, `Cargo.lock`, `crates/shadows-core/Cargo.toml`; caller imports in `crates/shadows-core/src/design/agreement_scope.rs`, `crates/shadows-core/src/design/map_proposals.rs`, `crates/shadows-core/src/design/mod.rs`, `crates/shadows-core/src/design/proposal_context.rs`, `crates/shadows-core/src/design/proposals.rs`, `crates/shadows-core/src/design/roadmap_proposals.rs`, `crates/shadows-core/src/design/store/agreement_write.rs`, `crates/shadows-core/src/design/store/agreements.rs`, `crates/shadows-core/src/design/store/map_approval.rs`, `crates/shadows-core/src/design/store/map_proposal_read.rs`, `crates/shadows-core/src/design/store/map_proposals.rs`, `crates/shadows-core/src/design/store/proposal_approval.rs`, `crates/shadows-core/src/design/store/proposal_read.rs`, `crates/shadows-core/src/design/store/proposals.rs`, `crates/shadows-core/src/design/store/roadmap_approval.rs`, `crates/shadows-core/src/design/store/roadmap_proposal_read.rs`, `crates/shadows-core/src/design/store/roadmap_proposals.rs`, `crates/shadows-core/src/harness/mod.rs`, `crates/shadows-core/src/harness/sessions.rs`, `crates/shadows-core/src/harness/setup.rs`, `crates/shadows-core/src/plans/agreements.rs`, `crates/shadows-core/src/plans/binding_edit.rs`, `crates/shadows-core/src/plans/mod.rs`, `crates/shadows-core/src/plans/model.rs`, `crates/shadows-core/src/plans/roadmap_approval.rs`, `crates/shadows-core/src/plans/scope.rs`, `crates/shadows-core/src/plans/store/draft.rs`, `crates/shadows-core/src/plans/store/edit.rs`, `crates/shadows-core/src/plans/store/mod.rs`, `crates/shadows-core/src/plans/store/read.rs`, `crates/shadows-core/src/plans/store/view.rs`, `crates/shadows-core/src/plans/testing.rs`, `crates/shadows-core/src/project_lifecycle/store.rs`, `crates/shadows-core/src/turns/store/remove.rs`; those callers' owner contracts; moved owner-local tests and their old registrations; `crates/shadows-core/tests/contracts.rs`; `docs/codebase/README.md`; `docs/backend/stages.md`; `crates/shadows/tests/architecture/{scanner,resolution,sql,fixtures,main}.rs`, `table-owners.txt` and allowlists (delete lines only, except exact moved-file comment path relocations); `crates/shadows/tests/codemap/main.rs`. AppCore integration tests remain in Core.
- Done when: `cargo test -p core-grants`, `cargo test -p shadows-core --lib` and Core integration binaries `planner_isolation` pass; affected callers and architecture/codemap/openapi checks pass; no Cargo cycle, cross-owner SQL, dependency on `shadows-core` or `AppCore`; stores are private; adapter re-exports and schemas stay stable.

#### A-19c.14 · Extract core-operations
- Status: todo
- Needs: A-19c.0 error split in this move; A-19c.0a and A-19c.0b; A-19b ; A-19c.1, A-19c.12
- Touch: `crates/core-operations/` (`Cargo.toml`, `src/lib.rs`, `src/model.rs`, `src/store/`, `contract.yaml`, `tests/`); old `crates/shadows-core/src/operations/`; `crates/shadows-core/src/{lib,app,testing/**}.rs`; `Cargo.toml`, `Cargo.lock`, `crates/shadows-core/Cargo.toml`; caller imports in `crates/shadows-core/src/design/mod.rs`, `crates/shadows-core/src/design/store/map_approval.rs`, `crates/shadows-core/src/design/store/map_proposals.rs`, `crates/shadows-core/src/design/store/proposal_approval.rs`, `crates/shadows-core/src/design/store/proposals.rs`, `crates/shadows-core/src/design/store/roadmap_approval.rs`, `crates/shadows-core/src/design/store/roadmap_proposals.rs`, `crates/shadows-core/src/harness/fork.rs`, `crates/shadows-core/src/harness/mod.rs`, `crates/shadows-core/src/harness/store/invocations.rs`, `crates/shadows-core/src/harness/store/mode.rs`, `crates/shadows-core/src/harness/store/thread.rs`, `crates/shadows-core/src/plans/mod.rs`, `crates/shadows-core/src/plans/store/edit.rs`, `crates/shadows-core/src/plans/testing.rs`, `crates/shadows-core/src/turns/mod.rs`, `crates/shadows-core/src/turns/model.rs`, `crates/shadows-core/src/turns/shutdown.rs`, `crates/shadows-core/src/turns/store/operation.rs`, `crates/shadows-core/src/turns/store/operation_read.rs`, `crates/shadows-core/src/turns/store/queue.rs`, `crates/shadows-core/src/turns/store/standards.rs`, `crates/shadows-core/src/turns/store/turn.rs`, `crates/shadows-core/src/turns/store/usage.rs`; those callers' owner contracts; moved owner-local tests and their old registrations; `crates/shadows-core/tests/contracts.rs`; `docs/codebase/README.md`; `docs/backend/stages.md`; `crates/shadows/tests/architecture/{scanner,resolution,sql,fixtures,main}.rs`, `table-owners.txt` and allowlists (delete lines only, except exact moved-file comment path relocations); `crates/shadows/tests/codemap/main.rs`. AppCore integration tests remain in Core.
- Done when: `cargo test -p core-operations`, `cargo test -p shadows-core --lib` and Core integration binaries `operation_lifecycle`, `recovery` pass; affected callers and architecture/codemap/openapi checks pass; no Cargo cycle, cross-owner SQL, dependency on `shadows-core` or `AppCore`; stores are private; adapter re-exports and schemas stay stable.

#### A-19c.15 · Extract core-harness
- Status: todo
- Needs: A-19c.0 error split in this move; A-19c.0a and A-19c.0b; A-19b ; A-19c.2, A-19c.3, A-19c.4, A-19c.5, A-19c.7, A-19c.8, A-19c.9, A-19c.11, A-19c.12, A-19c.13, A-19c.14
- Touch: `crates/core-harness/` (`Cargo.toml`, `src/lib.rs`, `src/model.rs`, `src/store/`, `contract.yaml`, `tests/`); old `crates/shadows-core/src/harness/`; `crates/shadows-core/src/{lib,app,testing/**}.rs`; `Cargo.toml`, `Cargo.lock`, `crates/shadows-core/Cargo.toml`; caller imports in `crates/shadows-core/src/design/store/map_proposal_shape.rs`, `crates/shadows-core/src/design/store/proposals.rs`, `crates/shadows-core/src/error.rs`, `crates/shadows-core/src/events/mod.rs`, `crates/shadows-core/src/events/subscription.rs`, `crates/shadows-core/src/plans/store/read.rs`, `crates/shadows-core/src/turns/handles.rs`, `crates/shadows-core/src/turns/mod.rs`, `crates/shadows-core/src/turns/model.rs`, `crates/shadows-core/src/turns/record.rs`, `crates/shadows-core/src/turns/shutdown.rs`, `crates/shadows-core/src/turns/spawn.rs`, `crates/shadows-core/src/turns/store/operation.rs`, `crates/shadows-core/src/turns/store/operation_read.rs`, `crates/shadows-core/src/turns/store/remove.rs`, `crates/shadows-core/src/turns/store/standards.rs`, `crates/shadows-core/src/turns/store/turn.rs`, `crates/shadows-core/src/turns/store/usage.rs`, `crates/shadows-core/src/turns/testing.rs`, `crates/shadows-core/src/turns/turn.rs`; those callers' owner contracts; moved owner-local tests and their old registrations; `crates/shadows-core/tests/contracts.rs`; `docs/codebase/README.md`; `docs/backend/stages.md`; `crates/shadows/tests/architecture/{scanner,resolution,sql,fixtures,main}.rs`, `table-owners.txt` and allowlists (delete lines only, except exact moved-file comment path relocations); `crates/shadows/tests/codemap/main.rs`. AppCore integration tests remain in Core.
- Done when: `cargo test -p core-harness`, `cargo test -p shadows-core --lib` and Core integration binaries `aylul_session`, `sessions`, `thread_session` pass; affected callers and architecture/codemap/openapi checks pass; no Cargo cycle, cross-owner SQL, dependency on `shadows-core` or `AppCore`; stores are private; adapter re-exports and schemas stay stable.

#### A-19c.16 · Extract core-design
- Status: todo
- Needs: A-19c.0 error split in this move; A-19c.0a and A-19c.0b; A-19b ; A-19c.3, A-19c.5, A-19c.12, A-19c.13, A-19c.14, A-19c.15
- Touch: `crates/core-design/` (`Cargo.toml`, `src/lib.rs`, `src/model.rs`, `src/store/`, `contract.yaml`, `tests/`); old `crates/shadows-core/src/design/`; `crates/shadows-core/src/{lib,app,testing/**}.rs`; `Cargo.toml`, `Cargo.lock`, `crates/shadows-core/Cargo.toml`; caller imports in `crates/shadows-core/src/plans/agreement_review.rs`, `crates/shadows-core/src/plans/agreements.rs`, `crates/shadows-core/src/plans/bindings.rs`, `crates/shadows-core/src/plans/design_edit.rs`, `crates/shadows-core/src/plans/design_ops.rs`, `crates/shadows-core/src/plans/design_views.rs`, `crates/shadows-core/src/plans/mod.rs`, `crates/shadows-core/src/plans/ops.rs`, `crates/shadows-core/src/plans/roadmap_approval.rs`, `crates/shadows-core/src/plans/scope.rs`, `crates/shadows-core/src/plans/store/agreement_review.rs`, `crates/shadows-core/src/plans/store/bindings.rs`, `crates/shadows-core/src/plans/store/design_links.rs`, `crates/shadows-core/src/plans/store/design_parts.rs`, `crates/shadows-core/src/plans/store/draft.rs`, `crates/shadows-core/src/plans/store/edit.rs`, `crates/shadows-core/src/plans/testing.rs`, `crates/shadows-core/src/turns/stage.rs`; those callers' owner contracts; moved owner-local tests and their old registrations; `crates/shadows-core/tests/contracts.rs`; `docs/codebase/README.md`; `docs/backend/stages.md`; `crates/shadows/tests/architecture/{scanner,resolution,sql,fixtures,main}.rs`, `table-owners.txt` and allowlists (delete lines only, except exact moved-file comment path relocations); `crates/shadows/tests/codemap/main.rs`. AppCore integration tests remain in Core.
- Done when: `cargo test -p core-design`, `cargo test -p shadows-core --lib` and Core integration binaries `agreement_lifecycle`, `design_outcomes`, `design_parts`, `design_vision` pass; affected callers and architecture/codemap/openapi checks pass; no Cargo cycle, cross-owner SQL, dependency on `shadows-core` or `AppCore`; stores are private; adapter re-exports and schemas stay stable.

#### A-19c.17 · Extract core-plans
- Status: todo
- Needs: A-19c.0 error split in this move; A-19c.0a and A-19c.0b; A-19b ; A-19c.3, A-19c.6, A-19c.12, A-19c.13, A-19c.14, A-19c.15, A-19c.16
- Touch: `crates/core-plans/` (`Cargo.toml`, `src/lib.rs`, `src/model.rs`, `src/store/`, `contract.yaml`, `tests/`); old `crates/shadows-core/src/plans/`; `crates/shadows-core/src/{lib,app,testing/**}.rs`; `Cargo.toml`, `Cargo.lock`, `crates/shadows-core/Cargo.toml`; caller imports in `crates/shadows-core/src/events/model.rs`, `crates/shadows-core/src/events/store.rs`, `crates/shadows-core/src/project_lifecycle/store.rs`, `crates/shadows-core/src/turns/mod.rs`, `crates/shadows-core/src/turns/model.rs`, `crates/shadows-core/src/turns/plan_show.rs`, `crates/shadows-core/src/turns/prompt.rs`, `crates/shadows-core/src/turns/record.rs`, `crates/shadows-core/src/turns/store/queue.rs`, `crates/shadows-core/src/turns/store/turn.rs`; those callers' owner contracts; moved owner-local tests and their old registrations; `crates/shadows-core/tests/contracts.rs`; `docs/codebase/README.md`; `docs/backend/stages.md`; `crates/shadows/tests/architecture/{scanner,resolution,sql,fixtures,main}.rs`, `table-owners.txt` and allowlists (delete lines only, except exact moved-file comment path relocations); `crates/shadows/tests/codemap/main.rs`. AppCore integration tests remain in Core.
- Done when: `cargo test -p core-plans`, `cargo test -p shadows-core --lib` and Core integration binaries `agreement_bindings`, `cross_plan_links`, `cross_plan_resolution`, `cross_plan_views`, `workflow_rules` pass; affected callers and architecture/codemap/openapi checks pass; no Cargo cycle, cross-owner SQL, dependency on `shadows-core` or `AppCore`; stores are private; adapter re-exports and schemas stay stable.

#### A-19c.18 · Extract core-turns
- Status: todo
- Needs: A-19c.0 error split in this move; A-19c.0a and A-19c.0b; A-19b ; A-19c.1, A-19c.3, A-19c.4, A-19c.5, A-19c.6, A-19c.10, A-19c.12, A-19c.13, A-19c.14, A-19c.15, A-19c.16, A-19c.17
- Touch: `crates/core-turns/` (`Cargo.toml`, `src/lib.rs`, `src/model.rs`, `src/store/`, `contract.yaml`, `tests/`); old `crates/shadows-core/src/turns/`; `crates/shadows-core/src/{lib,app,testing/**}.rs`; `Cargo.toml`, `Cargo.lock`, `crates/shadows-core/Cargo.toml`; caller imports in `crates/shadows-core/src/error.rs`, `crates/shadows-core/src/events/mod.rs`, `crates/shadows-core/src/events/model.rs`, `crates/shadows-core/src/events/store.rs`, `crates/shadows-core/src/events/subscription.rs`, `crates/shadows-core/src/plans/store/draft.rs`, `crates/shadows-core/src/plans/store/edit.rs`, `crates/shadows-core/src/plans/store/mod.rs`, `crates/shadows-core/src/plans/store/view.rs`; those callers' owner contracts; moved owner-local tests and their old registrations; `crates/shadows-core/tests/contracts.rs`; `docs/codebase/README.md`; `docs/backend/stages.md`; `crates/shadows/tests/architecture/{scanner,resolution,sql,fixtures,main}.rs`, `table-owners.txt` and allowlists (delete lines only, except exact moved-file comment path relocations); `crates/shadows/tests/codemap/main.rs`. AppCore integration tests remain in Core.
- Done when: `cargo test -p core-turns`, `cargo test -p shadows-core --lib` and Core integration binaries `aylul_usage`, `thread_session` pass; affected callers and architecture/codemap/openapi checks pass; no Cargo cycle, cross-owner SQL, dependency on `shadows-core` or `AppCore`; stores are private; adapter re-exports and schemas stay stable.

#### A-19c.19 · Extract core-project-lifecycle
- Status: todo
- Needs: A-19c.0 error split in this move; A-19c.0a and A-19c.0b; A-19b ; A-19c.3, A-19c.6, A-19c.12, A-19c.13, A-19c.17
- Touch: `crates/core-project-lifecycle/` (`Cargo.toml`, `src/lib.rs`, `src/model.rs`, `src/store/`, `contract.yaml`, `tests/`); old `crates/shadows-core/src/project_lifecycle/`; `crates/shadows-core/src/{lib,app,testing/**}.rs`; `Cargo.toml`, `Cargo.lock`, `crates/shadows-core/Cargo.toml`; caller imports in (no other source caller); those callers' owner contracts; moved owner-local tests and their old registrations; `crates/shadows-core/tests/contracts.rs`; `docs/codebase/README.md`; `docs/backend/stages.md`; `crates/shadows/tests/architecture/{scanner,resolution,sql,fixtures,main}.rs`, `table-owners.txt` and allowlists (delete lines only, except exact moved-file comment path relocations); `crates/shadows/tests/codemap/main.rs`. AppCore integration tests remain in Core.
- Done when: `cargo test -p core-project-lifecycle`, `cargo test -p shadows-core --lib` and Core integration binaries `project_assistant`, `project_contract` pass; affected callers and architecture/codemap/openapi checks pass; no Cargo cycle, cross-owner SQL, dependency on `shadows-core` or `AppCore`; stores are private; adapter re-exports and schemas stay stable.

#### A-19c.20 · Extract core-events
- Status: todo
- Needs: A-19c.0 error split in this move; A-19c.0a and A-19c.0b; A-19b ; A-19c.3, A-19c.12, A-19c.14, A-19c.15, A-19c.17, A-19c.18
- Touch: `crates/core-events/` (`Cargo.toml`, `src/lib.rs`, `src/model.rs`, `src/store/`, `contract.yaml`, `tests/`); old `crates/shadows-core/src/events/`; `crates/shadows-core/src/{lib,app,testing/**}.rs`; `Cargo.toml`, `Cargo.lock`, `crates/shadows-core/Cargo.toml`; caller imports in `crates/shadows-core/src/code/store/links.rs`, `crates/shadows-core/src/design/store/agreement_write.rs`, `crates/shadows-core/src/design/store/map_approval.rs`, `crates/shadows-core/src/design/store/map_proposals.rs`, `crates/shadows-core/src/design/store/proposal_approval.rs`, `crates/shadows-core/src/design/store/proposals.rs`, `crates/shadows-core/src/design/store/roadmap_approval.rs`, `crates/shadows-core/src/design/store/roadmap_proposals.rs`, `crates/shadows-core/src/design/store/structure.rs`, `crates/shadows-core/src/design/store/vision.rs`, `crates/shadows-core/src/grants/store.rs`, `crates/shadows-core/src/plans/store/edit.rs`, `crates/shadows-core/src/plans/store/mod.rs`, `crates/shadows-core/src/plans/store/notifications.rs`, `crates/shadows-core/src/plans/store/plan.rs`, `crates/shadows-core/src/plans/store/project_links.rs`, `crates/shadows-core/src/plans/store/view.rs`, `crates/shadows-core/src/projects/store.rs`, `crates/shadows-core/src/turns/mod.rs`, `crates/shadows-core/src/turns/plan_show.rs`, `crates/shadows-core/src/turns/store/queue.rs`, `crates/shadows-core/src/turns/store/turn.rs`; those callers' owner contracts; moved owner-local tests and their old registrations; `crates/shadows-core/tests/contracts.rs`; `docs/codebase/README.md`; `docs/backend/stages.md`; `crates/shadows/tests/architecture/{scanner,resolution,sql,fixtures,main}.rs`, `table-owners.txt` and allowlists (delete lines only, except exact moved-file comment path relocations); `crates/shadows/tests/codemap/main.rs`. AppCore integration tests remain in Core.
- Done when: `cargo test -p core-events`, `cargo test -p shadows-core --lib` and Core integration binaries `thread_session` pass; affected callers and architecture/codemap/openapi checks pass; no Cargo cycle, cross-owner SQL, dependency on `shadows-core` or `AppCore`; stores are private; adapter re-exports and schemas stay stable.

### A-20 · `shadows-http` and `shadows-mcp` split by job
- Status: done
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-20--shadows-http-and-shadows-mcp-split-by-job
- Read: the plan section
- Touch: crates/shadows-http/src/, crates/shadows-mcp/src/
- Done when: `cargo test -p shadows-http -p shadows-mcp -p shadows` passes; `api/` unchanged

### A-21 · `shadows-agent` and `shadows-process` split by job
- Status: done
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-21--shadows-agent-and-shadows-process-split-by-job
- Read: the plan section
- Ruling (controller, 2026-10-23, on the caller-scope blocker): Touch also covers every caller of a field made private or a free function turned into a method: its call site and import (in harness/ and turns/ named in the blocker, and any other), the contract lines and code-map rows naming them, and the tests that call them. Call-site changes only in those files.
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
- Status: done
- Plan: none. Owner spec: docs/superpowers/specs/2026-09-24-harness-controls-design.md §12.4 ("The mode is remembered per conversation"); request docs/shared/requests/2026-10-22-remembered-mode.md (accepted)
- Read: harness/contract.yaml, harness/store.rs, harness/settings.rs, crates/shadows-agent/src/policy.rs; the request
- Touch: crates/shadows-core/src/harness/ (store, settings, the new command, contract.yaml), one new migration file in crates/shadows-core/migrations/ for the per-conversation mode row, crates/shadows-http/src/harness.rs (the route), crates/shadows-http/src/lib.rs (`routes`, the route's registration line only), api/openapi.json (regenerated), the threads removal/fork call sites that must remove or copy the row, crates/shadows/tests/ (new acceptance tests), docs/codebase/README.md
- Ruling (controller, 2026-10-23, on two scope blockers): Touch also covers every registration line the new route, migration and table need (the `routes` table, migrations, table-owners.txt, app.rs wiring, the architecture and contract fixtures that list routes or tables). A missing registration file of this kind is a small gap: fix it and report it, do not block.
- Needs: A-17 (the harness boundary is reshaped there)
- Done when: picking Auto without sending, then reopening the session and restarting Core, answers Auto; a mode the project disallows is refused before anything is written; a disallowed remembered mode falls back to the policy default on open; a running turn keeps its mode and the new one applies to the next turn; one conversation's mode does not reach another; a fork copies it; a replay with a different mode is `CommandConflict`; `cargo test -p shadows --test openapi` and the new tests pass

### B-11 · Images in a message
- Status: done
- Plan: none. Owner specs: docs/superpowers/specs/2026-09-21-domain-model-design.md §4.2 "Attachment", §12.11; request docs/shared/requests/2026-10-22-image-attachments.md (accepted)
- Read: threads/contract.yaml, turns/contract.yaml (prompt composition), harness/contract.yaml, crates/shadows-agent/src/acp.rs (`Connection::prompt`, steering, `initialize`); the request
- Touch: crates/shadows-core/src/threads/ (attachment record, bytes, upload/read, removal, fork), a migration under crates/shadows-core/src/db/, crates/shadows-core/src/turns/ (ids in turn/queue/steer commands and fingerprints; neutral image blocks in `Prompt::compose`), crates/shadows-core/src/harness/ (capabilities in the session answer), crates/shadows-agent/src/ (neutral blocks to ACP image blocks; keep `promptCapabilities`), crates/shadows-http/src/ (the two routes, the new fields), api/openapi.json (regenerated), crates/shadows/tests/ (new acceptance tests), docs/codebase/README.md
- Ruling (controller, 2026-10-23, on the size-limit blocker): 5 MiB (5 242 880 bytes) per image; PNG, JPEG, GIF, WebP by content. Written into the Attachment section of the domain-model spec. One constant in threads owns it.
- Needs: A-18 (turns is reshaped there); B-10 is independent
- Done when: an uploaded PNG is refused when its bytes are not an image and over the limit; a turn with text plus two images sends the text and both images in order to the fake ACP agent (exact blocks asserted); an image-only turn works; an id from another thread is refused before anything is written; a replay with other ids is `CommandConflict`; a queued and a steered message keep their images; a fork copies the references; removing the thread removes the bytes; a harness without image capability refuses with its reason; no ACP type appears in HTTP or stored entries; `cargo test -p shadows --test openapi` and the new tests pass

### B-A2 · Prepare Core's scoped context contract
- Status: stopped (2026-10-19: its Hermes integration plan was deleted, Git `226d0a3`; AYLUL's work profile will define its own need)
- Plan: none
- Read: projects/threads/plans/design contracts; architecture section 1.7; approval section 24.6; integration T3.
- Touch: the required shared request/owner specification first; implementation paths only after that contract is accepted.
- Needs: accepted caller/sharing/authority contract and counterpart native/UI request.
- Done when: the scoped projection and its refusal/revocation/identity tests are agreed without assigning AYLUL or LSP runtime ownership to AppCore.
