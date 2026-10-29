# Core backend stages

This file tracks the existing Shadows Core backend only. See
[the documentation map](../README.md) for AYLUL and independent service work.

## Now

The architecture restructuring ([plan](plans/2026-10-21-architecture.md)) shipped in
PR #41 (`c7e7bac4`): A-0 to A-21, A-23, B-10 and B-11 are in this file at `c7e7bac4`.
The rules are CLAUDE.md "Code structure"; its strict rule binds every task: start at
`AppCore`, work only in the owning service. Live keys: only `OPENROUTER` and
`API_KEY_MINIMAX`, read with a loop, never `. ./.env`, never printed or committed.

Before dispatch the controller expands each task's named scope into exact caller,
test, contract and map paths and records the affected test commands. A blocked
task is not a builder assignment. Per-task checks are focused; the full gate and
whole-branch review run once before each delivery PR.

A-6 is settled with no code change (Mohammed confirmed the approval-authority spec's §24.2
on 2026-10-27); its entry is in this file at `4d5c022d`.

A-7b (the breakdown route goes) and B-12 (test fixtures close their database before their
folder goes) shipped in PR #45 (`5a0af4ff`); their entries are in this file at `29ddfd88`.

A-22 (product workspaces) is deferred on 2026-10-28: its trigger is in
[docs/future.md](../future.md) "Housekeeping"; its entry is in this file at `3dc2437f`.

B-13 (the person record takes a partial update, `PATCH /api/me`) shipped in PR #46
(`e95b79c3`); its entry is in this file at `e95b79c3`.

## Tasks

### A-8b1 · Delete the Hermes product daemon
- Status: done
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-8b--the-binary-has-one-start-and-no-second-surface
- Read: `crates/shadows/src/cli/args.rs`; `crates/shadows/src/daemon/contract.yaml`;
  docs/integration/2026-10-19-product-daemon-hermes-design.md
- Touch: `crates/shadows/src/cli/args.rs` (the `Daemon` command and its tests);
  `crates/shadows/src/lib.rs`; delete `crates/shadows/src/daemon/`,
  `crates/shadows/tests/assistant_daemon.rs`, `crates/shadows/tests/fixtures/assistant_process.rs`,
  `packages/hermes-client/`, `services/assistant/`; the `assistant-fixture` `[[bin]]` in
  `crates/shadows/Cargo.toml`; `Cargo.lock` if it changes; the `daemon/`, `daemon/bridge.rs`,
  `daemon/http.rs` and `daemon/descriptor.rs` rows in `docs/codebase/README.md`; the
  `daemon/descriptor.rs` entry in `crates/shadows/tests/architecture/ownership.allowlist`;
  the integration spec, amended to say Hermes is removed
- Needs: none (Mohammed decided the deletion on 2026-10-29)
- Done when: `shadows daemon` is refused as an unknown command; `cargo test -p shadows`,
  including the `codemap` and `architecture` tests, passes; no `.rs`, `Cargo.toml` or workflow file
  names `daemon::`, `assistant-fixture`, `hermes-client` or `services/assistant`

### A-8b2 · Move the remaining rules out of the binary
- Status: todo
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-8b--the-binary-has-one-start-and-no-second-surface
- Read: `crates/shadows/src/{aylul,master_key,config}.rs`, `master_key.contract.yaml`;
  `crates/core-secrets/contract.yaml`; `crates/core-projects/contract.yaml`;
  `crates/shadows-core/src/app.rs`
- Touch: `crates/shadows/src/{aylul,aylul_tests,master_key,master_key_tests,config,lib}.rs`,
  `master_key.contract.yaml`, `cli/mod.rs`, `cli/args.rs` if the AYLUL section is read there,
  a `config_tests.rs` beside `config.rs` for the moved AYLUL tests; `crates/shadows/Cargo.toml`
  (`keyring` and `rand` leave it); `crates/core-secrets/` (key loading, its `Cargo.toml` takes
  `keyring`, contract, tests); `crates/shadows-core/src/app.rs` (`AppCore::start` loads the key
  and ensures AYLUL's project), `crates/shadows-core/src/lib.rs` for a re-export and
  `crates/shadows-core/src/error.rs` and `crates/shadows-core/tests/secrets_error_mapping.rs`
  for the new `SecretsError` variant; the
  `StartConfig` builders `crates/shadows-core/src/testing/mod.rs` and
  `crates/shadows-http/src/aylul_env_tests.rs`; `crates/core-projects/contract.yaml` if its
  refusal wording moves; `Cargo.lock`; the `master_key*`, `aylul*` and `config*` rows in
  `docs/codebase/README.md`, and rows for the new `core-secrets` files
- Needs: A-8b1 (done; Mohammed confirmed the owners on 2026-10-29)
- Done when: the binary holds only configuration, tracing, binding and signals; no file under
  `crates/shadows/src` names `keyring` or `ensure_assistant`; `cargo test -p shadows`,
  `-p core-secrets`, `-p core-projects` and `-p shadows-core` and the `shadows-http` library
  tests pass

### B-14 · Provider refusals name the field
- Status: blocked
- Plan: docs/frontend/plans/2026-10-25-web-architecture.md#w-11--remove-copied-core-decisions-after-api-support-exists (packet 1)
- Read: docs/shared/requests/2026-10-29-provider-refusals.md; `crates/core-providers/contract.yaml`;
  `crates/core-providers/src/validation.rs`
- Touch: `crates/core-providers/src/{validation,error}.rs`, its tests and contract; the
  `ProvidersError` mapping in `crates/shadows-core/src/error.rs`; the error body in
  `crates/shadows-http/src/failure/` (`mod.rs` and the case files) if it gains `field`; the
  create route in `aylul_providers.rs` and the edit route in `aylul_agents.rs` only if their
  answers change; `crates/shadows-http/src/aylul_profiles_tests.rs`; `api/openapi.json`
- Needs: the request accepted
- Done when: each refused rule answers its own field and message from both create and edit,
  and a model sent with no context window is saved with Core's default; `cargo test -p
  core-providers` and HTTP tests refusing one create and one edit pass

### B-15 · The active-projects setting carries its bounds
- Status: blocked
- Plan: docs/frontend/plans/2026-10-25-web-architecture.md#w-11--remove-copied-core-decisions-after-api-support-exists (packet 2)
- Read: docs/shared/requests/2026-10-29-active-limit-bounds.md; `crates/core-code/contract.yaml`
- Touch: `crates/core-code/src/` (one constant pair the check and the read share), its
  contract and tests; `crates/shadows-http/src/code.rs`; `api/openapi.json`
- Needs: the request accepted
- Done when: `GET /api/code/settings` returns the bounds the write enforces;
  `cargo test -p core-code` and the code-settings HTTP tests pass

### B-16 · Each harness lists the modes Shadows allows
- Status: blocked
- Plan: docs/frontend/plans/2026-10-25-web-architecture.md#w-11--remove-copied-core-decisions-after-api-support-exists (packet 3)
- Read: docs/shared/requests/2026-10-29-harness-mode-policy.md; `crates/shadows-agent/src/policy.rs`;
  `crates/core-harness/contract.yaml`
- Touch: `crates/shadows-agent/src/policy.rs` (labels and descriptions beside the ids, and
  the unattended mode); `crates/core-harness/src/{model,lib}.rs`, contract and tests;
  `api/openapi.json`
- Needs: the request accepted
- Done when: `GET /api/harnesses` lists each harness's modes, initial and unattended mode
  from `policy.rs` alone; `cargo test -p shadows-agent` and `-p core-harness` pass

### B-17 · The design stage says how many files a structure writes
- Status: blocked
- Plan: docs/frontend/plans/2026-10-25-web-architecture.md#w-11--remove-copied-core-decisions-after-api-support-exists (packet 4)
- Read: docs/shared/requests/2026-10-29-structure-file-count.md; `crates/core-design/contract.yaml`;
  `crates/core-design/src/structure_create.rs`
- Touch: `crates/core-design/src/` (one count that `create_structure` and the stage read share;
  `StageView` gains the map revision; `create_structure` takes the confirmed revision and
  refuses a moved map before writing), contract and tests; the stage and structure routes in
  `crates/shadows-http/src/design.rs`; the callers of the changed signature and body:
  `crates/shadows-core/tests/design_parts/{structure,roadmap}.rs`,
  `crates/shadows/tests/workspace_mcp/roadmap.rs` and `crates/shadows/tests/design_routes.rs`;
  `api/openapi.json`
- Needs: the request accepted
- Done when:
  - a test holds the stage's count equal to the files `create_structure` writes;
  - a test changes the map between the stage read and the create, and nothing is written;
  - a retry with the same `CommandId` and revision returns the first result, and the same
    `CommandId` with another revision is `CommandConflict`;
  - `cargo test -p core-design`, `cargo test -p shadows-core --test design_parts`,
    `cargo test -p shadows --test workspace_mcp` and `cargo test -p shadows --test
    design_routes` pass

### B-A2 · Prepare Core's scoped context contract
- Status: stopped (2026-10-19: its Hermes integration plan was deleted, Git `226d0a3`; AYLUL's work profile will define its own need)
- Plan: none
- Read: projects/threads/plans/design contracts; architecture section 1.7; approval section 24.6; integration T3.
- Touch: the required shared request/owner specification first; implementation paths only after that contract is accepted.
- Needs: accepted caller/sharing/authority contract and counterpart native/UI request.
- Done when: the scoped projection and its refusal/revocation/identity tests are agreed without assigning AYLUL or LSP runtime ownership to AppCore.
