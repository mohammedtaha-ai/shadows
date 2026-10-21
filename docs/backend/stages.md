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
- Status: todo
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-2--turns-owns-what-a-turn-sends
- Read: turns/contract.yaml, harness/contract.yaml
- Touch: crates/shadows-core/src/turns/ (spawn.rs, steer.rs, new prompt.rs), crates/shadows-core/src/harness/setup.rs, both contracts, their tests
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
- Status: todo
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-8--every-service-is-built-in-appcorestart
- Read: crates/shadows-core/src/app.rs, harness/sessions.rs, testing/
- Touch: crates/shadows-core/src/{app.rs,harness/sessions.rs,profiles/mod.rs,testing/}
- Done when: affected construction/startup tests pass; services are built once, no service receives AppCore, and a restricted fixture exists; old CoreParts fields remain only for enumerated test callers until their owner tasks migrate them

### A-8b · The binary has one start and no second surface
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-8b--the-binary-has-one-start-and-no-second-surface
- Read: crates/shadows/src/{cli,daemon}/; docs/integration/2026-10-19-product-daemon-hermes-design.md; docs/aylul/specs/2026-10-19-aylul-claude-code-brain-design.md
- Touch: crates/shadows/src/, crates/shadows-http/src/ (the moved route), Core methods they need
- Needs: controller names each product-host operation's owner and exact paths; integration-spec amendment if ownership changes
- Done when: `cargo test -p shadows` passes; the binary holds only configuration, tracing, binding and signals

### A-8c · Settle the extraction dependencies before moving crates
- Status: todo
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-8c--settle-the-extraction-dependencies-before-moving-crates
- Read: AppCore, service contracts and named implementations; db/{mod,command,journal,error,write_observer}.rs; owner spec §14.12
- Touch: docs/backend/plans/2026-10-21-architecture.md, docs/backend/stages.md, docs/superpowers/specs/2026-09-26-application-core-design.md
- Needs: controller performs this design task; it creates no crates
- Done when: the runtime/type/error/test dependency ledger is complete, each cycle and table has an owner, extraction order is verified, and exact scopes/test commands are recorded for the next preparation tasks

### A-9 · Prepare the service boundary: instructions
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--establish-each-service-boundary-in-place
- Read: its current contract and the plan's Phase 3 row; A-8c's resolved dependency ledger
- Touch: crates/shadows-core/src/instructions/ and its contracts; controller enumerates the required app wiring, callers, scoped tests and map paths before dispatch
- Needs: A-8c's owner/type decisions and exact scope; prerequisite callee methods and fixtures
- Done when: its domain methods no longer extend Storage, cross-owner operations use service APIs, affected tests and ownership checks pass, and resolved A-0 violations are removed; no crate move in this task

### A-10 · Prepare the service boundary: secrets
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--establish-each-service-boundary-in-place
- Read: its current contract and the plan's Phase 3 row; A-8c's resolved dependency ledger
- Touch: crates/shadows-core/src/secrets/ and its contracts; controller enumerates the required app wiring, callers, scoped tests and map paths before dispatch
- Needs: A-8c's owner/type decisions and exact scope; prerequisite callee methods and fixtures
- Done when: its domain methods no longer extend Storage, cross-owner operations use service APIs, affected tests and ownership checks pass, and resolved A-0 violations are removed; no crate move in this task

### A-11 · Prepare the service boundary: projects
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--establish-each-service-boundary-in-place
- Read: its current contract and the plan's Phase 3 row; A-8c's resolved dependency ledger
- Touch: crates/shadows-core/src/projects/ and its contracts; controller enumerates the required app wiring, callers, scoped tests and map paths before dispatch
- Needs: A-8c's owner/type decisions and exact scope; prerequisite callee methods and fixtures
- Done when: its domain methods no longer extend Storage, cross-owner operations use service APIs, affected tests and ownership checks pass, and resolved A-0 violations are removed; no crate move in this task

### A-12 · Prepare the service boundary: code
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--establish-each-service-boundary-in-place
- Read: its current contract and the plan's Phase 3 row; A-8c's resolved dependency ledger
- Touch: crates/shadows-core/src/code/ and its contracts; controller enumerates the required app wiring, callers, scoped tests and map paths before dispatch
- Needs: A-8c's owner/type decisions and exact scope; prerequisite callee methods and fixtures
- Done when: its domain methods no longer extend Storage, cross-owner operations use service APIs, affected tests and ownership checks pass, and resolved A-0 violations are removed; no crate move in this task

### A-13 · Prepare the service boundary: grants
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--establish-each-service-boundary-in-place
- Read: its current contract and the plan's Phase 3 row; A-8c's resolved dependency ledger
- Touch: crates/shadows-core/src/grants/ and its contracts; controller enumerates the required app wiring, callers, scoped tests and map paths before dispatch
- Needs: A-8c's owner/type decisions and exact scope; prerequisite callee methods and fixtures; A-6 for the tool policy
- Done when: its domain methods no longer extend Storage, cross-owner operations use service APIs, affected tests and ownership checks pass, and resolved A-0 violations are removed; no crate move in this task

### A-14 · Prepare the service boundary: profiles, including person
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--establish-each-service-boundary-in-place
- Read: its current contract and the plan's Phase 3 row; A-8c's resolved dependency ledger
- Touch: crates/shadows-core/src/profiles/ and its contracts; controller enumerates the required app wiring, callers, scoped tests and map paths before dispatch
- Needs: A-8c's owner/type decisions and exact scope; prerequisite callee methods and fixtures
- Done when: its domain methods no longer extend Storage, cross-owner operations use service APIs, affected tests and ownership checks pass, and resolved A-0 violations are removed; no crate move in this task

### A-15 · Prepare the service boundary: threads
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--establish-each-service-boundary-in-place
- Read: its current contract and the plan's Phase 3 row; A-8c's resolved dependency ledger
- Touch: crates/shadows-core/src/threads/ and its contracts; controller enumerates the required app wiring, callers, scoped tests and map paths before dispatch
- Needs: A-8c's owner/type decisions and exact scope; prerequisite callee methods and fixtures
- Done when: its domain methods no longer extend Storage, cross-owner operations use service APIs, affected tests and ownership checks pass, and resolved A-0 violations are removed; no crate move in this task

### A-16 · Prepare the service boundary: design and plans
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--establish-each-service-boundary-in-place
- Read: its current contract and the plan's Phase 3 row; A-8c's resolved dependency ledger
- Touch: crates/shadows-core/src/{design,plans}/ and its contracts; controller enumerates the required app wiring, callers, scoped tests and map paths before dispatch
- Needs: A-8c's owner/type decisions and exact scope; prerequisite callee methods and fixtures
- Done when: its domain methods no longer extend Storage, cross-owner operations use service APIs, affected tests and ownership checks pass, and resolved A-0 violations are removed; no crate move in this task

### A-17 · Prepare the service boundary: harness
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--establish-each-service-boundary-in-place
- Read: its current contract and the plan's Phase 3 row; A-8c's resolved dependency ledger
- Touch: crates/shadows-core/src/harness/ and its contracts; controller enumerates the required app wiring, callers, scoped tests and map paths before dispatch
- Needs: A-8c's owner/type decisions and exact scope; prerequisite callee methods and fixtures
- Done when: its domain methods no longer extend Storage, cross-owner operations use service APIs, affected tests and ownership checks pass, and resolved A-0 violations are removed; no crate move in this task

### A-18 · Prepare the service boundary: turns
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--establish-each-service-boundary-in-place
- Read: its current contract and the plan's Phase 3 row; A-8c's resolved dependency ledger
- Touch: crates/shadows-core/src/turns/ and its contracts; controller enumerates the required app wiring, callers, scoped tests and map paths before dispatch
- Needs: A-8c's owner/type decisions and exact scope; prerequisite callee methods and fixtures
- Done when: its domain methods no longer extend Storage, cross-owner operations use service APIs, affected tests and ownership checks pass, and resolved A-0 violations are removed; no crate move in this task

### A-19 · Prepare the service boundary: events, runtime and command
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--establish-each-service-boundary-in-place
- Read: its current contract and the plan's Phase 3 row; A-8c's resolved dependency ledger
- Touch: crates/shadows-core/src/{events,runtime,command}/ and its contracts; controller enumerates the required app wiring, callers, scoped tests and map paths before dispatch
- Needs: A-8c's owner/type decisions and exact scope; prerequisite callee methods and fixtures
- Done when: its domain methods no longer extend Storage, cross-owner operations use service APIs, affected tests and ownership checks pass, and resolved A-0 violations are removed; no crate move in this task

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

### B-A2 · Prepare Core's scoped context contract
- Status: stopped (2026-10-19: its Hermes integration plan was deleted, Git `226d0a3`; AYLUL's work profile will define its own need)
- Plan: none
- Read: projects/threads/plans/design contracts; architecture section 1.7; approval section 24.6; integration T3.
- Touch: the required shared request/owner specification first; implementation paths only after that contract is accepted.
- Needs: accepted caller/sharing/authority contract and counterpart native/UI request.
- Done when: the scoped projection and its refusal/revocation/identity tests are agreed without assigning AYLUL or LSP runtime ownership to AppCore.
