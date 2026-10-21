# Core backend stages

This file tracks the existing Shadows Core backend only. See
[the documentation map](../README.md) for AYLUL and independent service work.

## Now

Branch `context-meter`: the architecture restructuring
([plan](plans/2026-10-21-architecture.md)). The rules are the root
`AGENTS.md` "Code structure", and its strict rule binds every task: start at
`AppCore`, work only in the owning service. Tasks run in the order below. Each
one keeps every existing test green and deletes its lines from the A-0
allowlists. Live keys: only `OPENROUTER` and `API_KEY_MINIMAX`, read with a
loop, never `. ./.env`, never printed or committed.

The settings dialog shipped in PR #40 (`1a2b601`). The context-meter probe
(B-1) is in Git at `e0478f7`; its corrections are A-7.

## Tasks

### A-0 · The architecture ratchet test
- Status: todo
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-0--the-architecture-ratchet-test
- Read: root `AGENTS.md` "Code structure"; the plan's audit table
- Touch: crates/shadows/tests/architecture/ (new)
- Done when: `cargo test -p shadows --test architecture` passes, and fails on one added violation of each kind

### A-1 · `AppCore` is the map
- Status: todo
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-1--appcore-is-the-map
- Read: root `AGENTS.md` "Code structure"; crates/shadows-core/src/app.rs
- Touch: crates/shadows-core/src/app.rs, docs/codebase/README.md, crates/shadows/tests/architecture/
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

### A-7 · Context-meter records corrected
- Status: todo
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-7--the-context-meter-records-corrected
- Read: docs/evidence/2026-10-21-context-meter-probe.md; §12.8
- Touch: that evidence file, docs/superpowers/specs/2026-09-24-harness-controls-design.md §12.8, the breakdown route and its web caller
- Done when: §12.8's OPEN is closed; the breakdown route's tests are removed and the rest pass

### A-8 · Every service is built in `AppCore::start`
- Status: todo
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-8--every-service-is-built-in-appcorestart
- Read: crates/shadows-core/src/app.rs, harness/sessions.rs, testing/
- Touch: crates/shadows-core/src/{app.rs,harness/sessions.rs,profiles/mod.rs,testing/}
- Done when: `cargo test --workspace` passes; `CoreParts` holds only `AppCore` and the database

### A-8b · The binary has one start and no second surface
- Status: todo
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-8b--the-binary-has-one-start-and-no-second-surface
- Read: crates/shadows/src/{cli,daemon}/
- Touch: crates/shadows/src/, crates/shadows-http/src/ (the moved route), Core methods they need
- Done when: `cargo test -p shadows` passes; the binary holds only configuration, tracing, binding and signals

### A-8c · `core-db`, the foundation crate
- Status: todo
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-8c--core-db-the-foundation-crate
- Read: root `AGENTS.md` "Code structure"; crates/shadows-core/src/db/
- Touch: crates/core-db/ (new), crates/shadows-core/src/db/, crates/shadows-core/Cargo.toml, Cargo.toml
- Done when: `cargo test --workspace` passes, and `core-db` names no service's table outside its migrations

### A-9 · One crate: `instructions`
- Status: todo
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--one-crate-per-service
- Read: its contract.yaml and its row in the plan's Phase 3 table
- Touch: crates/core-instructions/ (new, from crates/shadows-core/src/instructions/), crates/shadows-core/src/app.rs, Cargo.toml, the callers of what it stops exposing, its tests (moved into its crate)
- Done when: `cargo test --workspace` passes and its A-0 allowlist lines are gone

### A-10 · One crate: `secrets`
- Status: todo
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--one-crate-per-service
- Read: its contract.yaml and its row in the plan's Phase 3 table
- Touch: crates/core-secrets/ (new, from crates/shadows-core/src/secrets/), crates/shadows-core/src/app.rs, Cargo.toml, the callers of what it stops exposing, its tests (moved into its crate)
- Done when: `cargo test --workspace` passes and its A-0 allowlist lines are gone

### A-11 · One crate: `projects`
- Status: todo
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--one-crate-per-service
- Read: its contract.yaml and its row in the plan's Phase 3 table
- Touch: crates/core-projects/ (new, from crates/shadows-core/src/projects/), crates/shadows-core/src/app.rs, Cargo.toml, the callers of what it stops exposing, its tests (moved into its crate)
- Done when: `cargo test --workspace` passes and its A-0 allowlist lines are gone

### A-12 · One crate: `code`
- Status: todo
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--one-crate-per-service
- Read: its contract.yaml and its row in the plan's Phase 3 table
- Touch: crates/core-code/ (new, from crates/shadows-core/src/code/), crates/shadows-core/src/app.rs, Cargo.toml, the callers of what it stops exposing, its tests (moved into its crate)
- Done when: `cargo test --workspace` passes and its A-0 allowlist lines are gone

### A-13 · One crate: `grants`
- Status: todo
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--one-crate-per-service
- Read: its contract.yaml and its row in the plan's Phase 3 table
- Touch: crates/core-grants/ (new, from crates/shadows-core/src/grants/), crates/shadows-core/src/app.rs, Cargo.toml, the callers of what it stops exposing, its tests (moved into its crate)
- Done when: `cargo test --workspace` passes and its A-0 allowlist lines are gone

### A-14 · One crate each: `profiles`, split into `core-providers`, `core-agents`, `core-extensions`, `core-usage`
- Status: todo
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--one-crate-per-service
- Read: its contract.yaml and its row in the plan's Phase 3 table
- Touch: crates/core-{providers,agents,extensions,usage}/ (new, from crates/shadows-core/src/profiles/), crates/shadows-core/src/app.rs, Cargo.toml, the callers of what it stops exposing, its tests (moved into its crate)
- Done when: `cargo test --workspace` passes and its A-0 allowlist lines are gone

### A-15 · One crate: `threads`
- Status: todo
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--one-crate-per-service
- Read: its contract.yaml and its row in the plan's Phase 3 table
- Touch: crates/core-threads/ (new, from crates/shadows-core/src/threads/), crates/shadows-core/src/app.rs, Cargo.toml, the callers of what it stops exposing, its tests (moved into its crate)
- Done when: `cargo test --workspace` passes and its A-0 allowlist lines are gone

### A-16 · One crate each: `design` and `plans`, as `core-design` and `core-plans`
- Status: todo
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--one-crate-per-service
- Read: its contract.yaml and its row in the plan's Phase 3 table
- Touch: crates/core-design/ and crates/core-plans/ (new, from crates/shadows-core/src/{design,plans}/), crates/shadows-core/src/app.rs, Cargo.toml, the callers of what it stops exposing, its tests (moved into its crate)
- Done when: `cargo test --workspace` passes and its A-0 allowlist lines are gone

### A-17 · One crate: `harness`
- Status: todo
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--one-crate-per-service
- Read: its contract.yaml and its row in the plan's Phase 3 table
- Touch: crates/core-harness/ (new, from crates/shadows-core/src/harness/), crates/shadows-core/src/app.rs, Cargo.toml, the callers of what it stops exposing, its tests (moved into its crate)
- Done when: `cargo test --workspace` passes and its A-0 allowlist lines are gone

### A-18 · One crate: `turns`
- Status: todo
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--one-crate-per-service
- Read: its contract.yaml and its row in the plan's Phase 3 table
- Touch: crates/core-turns/ (new, from crates/shadows-core/src/turns/), crates/shadows-core/src/app.rs, Cargo.toml, the callers of what it stops exposing, its tests (moved into its crate)
- Done when: `cargo test --workspace` passes and its A-0 allowlist lines are gone

### A-19 · One crate each: `events`, `runtime` and `command`
- Status: todo
- Plan: docs/backend/plans/2026-10-21-architecture.md#phase-3--one-crate-per-service
- Read: its contract.yaml and its row in the plan's Phase 3 table
- Touch: crates/core-events/, crates/core-runtime/, crates/core-command/ (new, from crates/shadows-core/src/{events,runtime,command}/), crates/shadows-core/src/app.rs, Cargo.toml, the callers of what it stops exposing, its tests (moved into its crate)
- Done when: `cargo test --workspace` passes and its A-0 allowlist lines are gone

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
- Done when: `cargo test --workspace` passes

### A-22 · Split the repository into workspaces
- Status: todo
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-22--split-the-repository-by-product
- Read: the plan section; CLAUDE.md "Workspaces"
- Touch: runtime/ and code-intelligence/ (new), Cargo.toml, CLAUDE.md, AGENTS.md, docs/codebase/README.md, the codemap test, .github/workflows/ci.yml
- Done when: each workspace's gate passes from its own root

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
