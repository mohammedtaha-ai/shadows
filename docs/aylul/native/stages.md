# Native (Tauri, sidecar, hook) stages

Where this side's work is now. `native-lead` keeps this file; builders read
their task here and set its status. Task shape: [`AGENTS.md`](AGENTS.md#a-task-in-stagesmd).

## Now

The settings dialog request (`docs/aylul/shared/requests/2026-10-21-settings-dialog-native.md`,
accepted) is delivered by PR #40, merged at `1a2b601`: its tasks N-6 and N-7 are in this
file at `1a2b601`. No native task is open.

Finished work (N-1 to N-5, the hook merge, the 2026-10-20 Core start-up) is in this file at
`8580306`.

The desktop restructure ([plan](plans/2026-10-25-desktop-architecture.md), D-0 decided
2026-10-25) starts with its ratchet. Its tasks enter here one at a time.

## Tasks

### N-8 · D-1: a shrinking architecture ratchet for the native crate
- Status: done
- Plan: docs/aylul/native/plans/2026-10-25-desktop-architecture.md#decision-d-0 and
  #ratchet-design (items 1 to 4, 6 and 7; sidecar and TypeScript checks wait for D-8 and D-10)
- Read: the plan's sections 2, 3 and 6; `crates/shadows/tests/architecture/` for the
  pattern Core uses; `apps/desktop/src-tauri/src/*/contract.yaml`;
  `docs/aylul/codebase/README.md`
- Touch: `apps/desktop/src-tauri/tests/architecture/` (new: the test, its baseline, its
  fixtures); `apps/desktop/src-tauri/Cargo.toml` and its `Cargo.lock` (`syn` and similar as dev-dependencies
  only); `docs/aylul/codebase/README.md` (its row); this task's `Status:`
- Needs: none
- Done when: `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --test architecture`
  passes on today's tree with every current violation listed exactly in the baseline
  (owner edges outside the D-0 graph, managed-state lookups outside composition, service
  construction outside composition, cross-owner reach into a submodule, Core HTTP outside
  its owners, command inventory drift between `lib.rs`, `build.rs` and the permission
  files); a new violation fails; a stale baseline entry fails; fixture sources prove each
  rule catches its bypass (a `use` alias, a qualified path, a re-export,
  `app.state::<T>()`, `try_state`); no product source changes; CI picks it up through the
  existing `cargo test` step.
