# Local hook and independent-companion merge

Date: 2026-10-17. Local Windows verification, before the merge commit.

## Scope and provenance

The human explicitly requested the merge into the existing
`codex/aylul-laptop-relay`, with a commit after each completed task and no
worktree. The receiving parent is `cb89a1a`; the incoming joint human/Claude
hook branch is `spec/aylul-hook-laptop` at `0d086b6`. The common ancestor is
`2867d48`. A normal two-parent merge preserves that history.

Five conflicts were resolved by retaining both owners' behavior: desktop npm
scripts/dependencies, native composition, main-shell composition, its contract
and the code map. Incoming hooks, SQLite/session/approval handling and CI checks
are preserved. The independent companion, account actions, development-only
studio and restricted companion permissions remain composed alongside them.

The existing sidecar command needed registration in the receiving branch's
newer native ACL system. A caller-matrix regression failed for `main aylul_call`,
and an independent cargo-generated ACL test failed because main lacked that
permission. Both passed after adding the command to the build registry and
main capability only. Generated ACL/schema files come from cargo, not manual
editing. CI checks the generated-permission regression after the native build.
Parent feature ownership explicitly excludes the separately owned companion.

## Observed local checks

- Frontend `npm run build`: passed, including TypeScript and both HTML entries.
- Frontend `npm test`: 7 passed, including generated native ACL access and the
  existing request context/rule-preview protections.
- Companion unit suite: 15 passed; production dependency-graph check: 1 passed.
- Chromium `shell.spec.ts`: 7 passed, including coexistence of account,
  companion and a local hook request/answer, and unavailable-sidecar recovery.
- Sidecar: dependency installation, typecheck and all 71 tests passed. The
  `AYLUL_HOOK_EXE` used the freshly built debug binary; both actual Windows
  named-pipe hook roundtrip/no-sidecar cases ran rather than being skipped.
- Hook: debug build and all 12 Rust tests passed; formatting and clippy passed.
- Desktop Rust: 29 tests passed; the one opt-in actual-Docker native auth
  acceptance test remained ignored. Native formatting and clippy passed.
- All 12 changed YAML files parsed successfully.

One independent read-only integration review found no merge-introduced blocker.
It independently ran the 7 focused UI/rule-preview/generated-permission tests,
checked lockfile/dependency agreement, and confirmed the combined lifecycle,
restricted capabilities and ownership maps. Native clippy passed and all five
conflicts were resolved. Final whitespace and relative-link checks passed.
No remote CI result is claimed.

## Limits and next work

Browser checks use real Chromium with synthetic IPC, not a physical Tauri
approval journey. Native sidecar lifetime unit checks use the existing echo
process; an actual Tauri shutdown/endpoint-removal journey remains separate.
Tests used their own temporary data/settings; this task installed no personal
Claude Code hooks, launched no app against personal settings and uploaded no
local history. The backend checkout was not changed.

The local hook/store/sidecar now form the integrated baseline for the
[relay planning discussion](../native/specs/2026-10-17-laptop-relay-planning-brief.md).
Generated cloud-artifact consumption, durable outgoing checkpoints and native
credential handoff are still to be designed and implemented together.
