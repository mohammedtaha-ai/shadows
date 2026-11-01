# Native (Tauri, sidecar, hook) stages

Where this side's work is now. `native-lead` keeps this file; builders read
their task here and set its status. Task shape: [`AGENTS.md`](AGENTS.md#a-task-in-stagesmd).

## Now

Open: N-22 to N-25, the native half of the AYLUL agents and panel design (plan
`docs/frontend/plans/2026-11-01-aylul-agents-panel-frontend.md`, written by masada at the controller's request), on `feature/aylul-agents-panel`. They
build [`docs/aylul/shared/requests/2026-11-01-companion-panel.md`](../shared/requests/2026-11-01-companion-panel.md), accepted by the controller on
Mohammed's behalf on 2026-11-01. HTTP names: `docs/backend/plans/2026-11-01-aylul-agents-panel-backend.md` §3.

The desktop restructure is done and merged in PR #43 (`ca618f89`): its tasks N-8 to N-21 are
in this file at `ca618f89`. Its decision and open questions are in
[the desktop architecture spec](specs/2026-10-26-desktop-architecture-design.md).

Earlier finished work: N-6 and N-7 (the settings dialog request, PR #40) are in this file at
`1a2b601`; N-1 to N-5, the hook merge and the 2026-10-20 Core start-up are at `8580306`.

## Tasks

### N-22 · Companion publishes the bound agent's outfit
- Status: done
- Plan: docs/frontend/plans/2026-11-01-aylul-agents-panel-frontend.md#n-22
- Read: design §7.7; `docs/aylul/shared/requests/2026-11-01-companion-panel.md` (`appearance.outfit`); `apps/desktop/src-tauri/src/presence/contract.yaml`; `apps/desktop/src-tauri/src/windows/companion/contract.yaml`
- Touch: apps/desktop/src-tauri/src/presence/mod.rs, apps/desktop/src-tauri/src/presence/core_client/observer.rs, apps/desktop/src-tauri/src/presence/workflows.rs, apps/desktop/src-tauri/src/presence/appearance_tests.rs, apps/desktop/src-tauri/src/presence/binding_tests.rs, apps/desktop/src-tauri/src/presence/contract.yaml, apps/desktop/src-tauri/src/windows/companion/surface.rs, apps/desktop/src-tauri/src/windows/companion/state.rs, apps/desktop/src-tauri/src/windows/companion/contract.yaml, web/src/test/native-presence.json (rewritten by `write_presence_fixture`), docs/aylul/native/stages.md
- Needs: docs/aylul/shared/requests/2026-11-01-companion-panel.md (accepted). It reads `outfit` with a default, so it builds before backend §3.1; its real-app check waits on it.
- Done when: `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml appearance` and `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml companion_surface_wire_is_pinned` pass (outfit copied opaque beside colour; absent when the agent has none, when unbound or when unavailable), `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --test architecture` passes, and from `web/` `npx vitest run src/desktop/native-parity.test.ts` passes on the rewritten fixture.

### N-23 · Main destinations the panel opens
- Status: done (destination/navigation tests, 58 architecture checks and 6 web parity checks passed)
- Plan: docs/frontend/plans/2026-11-01-aylul-agents-panel-frontend.md#n-23
- Read: design §7.6; `docs/aylul/shared/requests/2026-11-01-companion-panel.md` (`open_main_from_companion`, `main-navigation`); `apps/desktop/src-tauri/src/presence/contract.yaml` (`destination`, `open_main_from_companion`)
- Touch: apps/desktop/src-tauri/src/navigation.rs, apps/desktop/src-tauri/src/presence/mod.rs, apps/desktop/src-tauri/src/presence/workflows.rs, apps/desktop/src-tauri/src/presence/binding_tests.rs, apps/desktop/src-tauri/src/presence/contract.yaml, apps/desktop/src-tauri/src/commands/companion.rs, apps/desktop/src-tauri/src/commands/contract.yaml, web/src/test/native-presence.json (rewritten by `write_presence_fixture`), docs/aylul/native/stages.md
- Needs: docs/aylul/shared/requests/2026-11-01-companion-panel.md (accepted); F-67 done
- Additional Touch: apps/desktop/src-tauri/build.rs, apps/desktop/src-tauri/src/contract.yaml, docs/aylul/codebase/README.md (Windows test manifest required when navigation workflow tests link native dialog imports).
- Done when: `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml destination` passes (each target resolves with reads only and creates no thread; `bound` keeps today's result; an unresolvable target falls back to `companionSettings`), `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml navigation` and `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --test architecture` pass, and from `web/` `npx vitest run src/desktop/native-parity.test.ts` passes on the rewritten fixture.

### N-24 · Panel surface, agent list and selection from the companion
- Status: done (16 window tests, 68 presence tests, caller guard and 58 architecture checks passed)
- Plan: docs/frontend/plans/2026-11-01-aylul-agents-panel-frontend.md#n-24
- Read: design §4, §7.6; `docs/aylul/shared/requests/2026-11-01-companion-panel.md` (`panel`, `companion_panel`, `select_companion_agent`); both companion contracts; desktop architecture spec "Owners" and "Allowed dependency directions"
- Touch: apps/desktop/src-tauri/src/windows/companion/mod.rs, apps/desktop/src-tauri/src/windows/companion/input.rs, apps/desktop/src-tauri/src/windows/companion/surface.rs, apps/desktop/src-tauri/src/windows/companion/state.rs, apps/desktop/src-tauri/src/windows/companion/contract.yaml, apps/desktop/src-tauri/src/presence/mod.rs, apps/desktop/src-tauri/src/presence/core_client/observer.rs, apps/desktop/src-tauri/src/presence/workflows.rs, apps/desktop/src-tauri/src/presence/appearance_tests.rs, apps/desktop/src-tauri/src/presence/contract.yaml, apps/desktop/src-tauri/src/commands/companion.rs, apps/desktop/src-tauri/src/commands/desktop.rs, apps/desktop/src-tauri/src/commands/mod.rs, apps/desktop/src-tauri/src/commands/contract.yaml, apps/desktop/src-tauri/src/lib.rs, apps/desktop/src-tauri/tests/architecture/baseline.txt (only if a guard or grant inventory line must move), docs/aylul/codebase/README.md, docs/aylul/native/stages.md
- Needs: docs/aylul/shared/requests/2026-11-01-companion-panel.md (accepted); N-22 done. It reads `description` and `outfit` with defaults, so it builds before backend §3.1.
- Additional Touch: apps/desktop/src-tauri/src/app.rs and src/contract.yaml (inject panel surface action); apps/desktop/src-tauri/build.rs, permissions/companion.toml, generated permissions/autogenerated/companion_panel.toml and gen/schemas/{acl-manifests,desktop-schema,windows-schema}.json (Cargo-generated IPC metadata); apps/desktop/src/platform/companion.ts and contract.yaml (thin wrapper required by native grant/wrapper parity before U-6 consumes the panel). The baseline may add exact State/DTO/registration inventory lines for this new guarded IPC, without changing a rule.
- Done when: `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml windows::companion` passes with new tests (panel open fits the whole rectangle and keeps the anchor; close and blur restore the compact bounds; the panel wire is pinned), `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml presence` passes (agents published within their bounds; selection from the companion surface goes through the same method and keeps the generation and target rules, `latest_explicit_retarget_wins_and_old_generation_is_rejected` unchanged), the command guard test that lists callers passes, and `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --test architecture` passes.

### N-25 · Waiting permissions: count, the bound request, `blocked`, the answer
- Status: blocked (code and tests are in; 15 watcher and 74 presence tests pass, but `--test architecture` fails until the lead adds, outside Touch, `answer_companion_permission` to build.rs, permissions/companion.toml, the generated permission and schema files, apps/desktop/src/platform/companion.ts, apps/desktop/tests/native-acl.test.mjs, and the new baseline.txt lines; the count and answer are unusable by the companion until then)
- Plan: docs/frontend/plans/2026-11-01-aylul-agents-panel-frontend.md#n-25
- Read: design §7.6, §7.8; `docs/backend/plans/2026-11-01-aylul-agents-panel-backend.md` §3.3; `docs/aylul/shared/requests/2026-11-01-companion-panel.md` (`waiting`, `answer_companion_permission`, `oldestWaiting`); `apps/desktop/src-tauri/src/presence/contract.yaml`
- Touch: apps/desktop/src-tauri/src/presence/core_client/mod.rs, apps/desktop/src-tauri/src/presence/core_client/observer.rs, apps/desktop/src-tauri/src/presence/core_client/blocking.rs, apps/desktop/src-tauri/src/presence/watcher.rs, apps/desktop/src-tauri/src/presence/watcher/feed.rs, apps/desktop/src-tauri/src/presence/watcher_fixture.rs, apps/desktop/src-tauri/src/presence/watcher_tests.rs, apps/desktop/src-tauri/src/presence/projection.rs, apps/desktop/src-tauri/src/presence/tests.rs, apps/desktop/src-tauri/src/presence/mod.rs, apps/desktop/src-tauri/src/presence/workflows.rs, apps/desktop/src-tauri/src/presence/binding_tests.rs, apps/desktop/src-tauri/src/presence/contract.yaml, apps/desktop/src-tauri/src/windows/companion/surface.rs, apps/desktop/src-tauri/src/windows/companion/state.rs, apps/desktop/src-tauri/src/windows/companion/contract.yaml, apps/desktop/src-tauri/src/commands/companion.rs, apps/desktop/src-tauri/src/commands/mod.rs, apps/desktop/src-tauri/src/commands/contract.yaml, apps/desktop/src-tauri/src/lib.rs, docs/aylul/codebase/README.md, docs/aylul/native/stages.md
- Needs: backend §3.3 built and in `api/openapi.json`; N-23 and N-24 done
- Done when: `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml watcher` passes with new fixture cases (`GET /api/aylul/permissions` is read at start and re-read on a `PermissionsChanged` frame of `/api/projects/{AgentWorkspace.project_id}/events`; a lagged or broken project feed recovers like the thread feed; the caps hold), `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml presence` passes (a request waiting on the bound thread makes the expression `blocked`; its end restores the actual status; the answer posts to `POST /api/threads/{id}/session/permissions/{request_id}` once per command id and maps `PERMISSION_NOT_FOUND`, `PERMISSION_SETTLED` and `PERMISSION_OPTION_NOT_OFFERED` to fixed codes; `oldestWaiting` resolves to the first entry's conversation), and `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --test architecture` passes (no `reqwest` outside `core_client`).
