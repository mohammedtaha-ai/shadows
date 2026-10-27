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

## Tasks

### B-13 · The person record takes a partial update
- Status: todo
- Read: the accepted request `docs/shared/requests/2026-10-26-partial-person-update.md`;
  `crates/shadows-core/src/app.rs` (`PersonSettings` owns the person record);
  `crates/core-person/contract.yaml`
- Touch: `crates/core-person/src/lib.rs` (`set_person` becomes `update_person`: validate only
  the sent fields, fingerprint them as sent, refuse a request with no field), `model.rs`
  (`PersonInput` becomes `PersonPatch`: each name and the language absent, `null` to clear,
  or a value; `theme` absent or a value), `store/mod.rs` (read the row, apply the patch and
  write it inside the one write transaction), `error.rs` (the no-field refusal), and
  `contract.yaml`; `crates/core-person/tests/person.rs`; the person type re-export in
  `crates/shadows-core/src/lib.rs`; `crates/shadows-http/src/aylul_person.rs`
  (`PUT /api/me` becomes `PATCH /api/me`) and its line in `crates/shadows-http/src/lib.rs`;
  `crates/shadows-http/src/aylul_profiles_tests.rs` (the person tests);
  `crates/shadows/tests/openapi.rs` if it names the route; `api/openapi.json` by
  `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi` only; `docs/codebase/README.md` if a
  row names `set_person`; this task's `Status:`
- Needs: none. F-28 lands on the same branch.
- Done when: a patch with one field keeps every other stored field; two patches of different
  fields, sent one after the other from the same earlier snapshot, both survive; `null` clears a
  name or the language; an empty patch is a 400; a replay with other fields is a 409; today's
  validation messages are unchanged; `cargo test -p core-person -p shadows-http`, the
  `shadows` openapi test, clippy with and without `test-support` and `cargo fmt --check` pass

### A-8b · The binary has one start and no second surface
- Status: blocked
- Plan: docs/backend/plans/2026-10-21-architecture.md#a-8b--the-binary-has-one-start-and-no-second-surface
- Read: crates/shadows/src/{cli,daemon}/; docs/integration/2026-10-19-product-daemon-hermes-design.md; docs/aylul/specs/2026-10-19-aylul-claude-code-brain-design.md
- Touch: crates/shadows/src/, crates/shadows-http/src/ (the moved route), Core methods they need
- Needs: controller names each product-host operation's owner and exact paths; integration-spec amendment if ownership changes
- Done when: `cargo test -p shadows` passes; the binary holds only configuration, tracing, binding and signals

### B-A2 · Prepare Core's scoped context contract
- Status: stopped (2026-10-19: its Hermes integration plan was deleted, Git `226d0a3`; AYLUL's work profile will define its own need)
- Plan: none
- Read: projects/threads/plans/design contracts; architecture section 1.7; approval section 24.6; integration T3.
- Touch: the required shared request/owner specification first; implementation paths only after that contract is accepted.
- Needs: accepted caller/sharing/authority contract and counterpart native/UI request.
- Done when: the scoped projection and its refusal/revocation/identity tests are agreed without assigning AYLUL or LSP runtime ownership to AppCore.
