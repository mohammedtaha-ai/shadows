# Native (Tauri, sidecar, hook) stages

Where this side's work is now. `native-lead` keeps this file; builders read
their task here and set its status. Task shape: [`AGENTS.md`](AGENTS.md#a-task-in-stagesmd).

## Now

The [settings dialog request][settings-request] is accepted: N-6 and N-7 below.
The desktop UI's U-1 and the web's F-12 and F-14 wait on them.

Finished work (N-1 to N-5, the hook merge, the 2026-10-20 Core start-up) is in this file at
`8580306`.

## Tasks

### N-6 · The companion wears its agent's colour, refreshed on save
- Status: blocked
- Blocker: steps written, waiting for the controller's review
- Plan: docs/aylul/native/plans/2026-10-21-settings-native.md#task-n-6
- Read: [the accepted request][settings-request]; native `presence/contract.yaml`,
  `windows/contract.yaml`, `windows/companion/contract.yaml`, `commands/contract.yaml`;
  docs/aylul/codebase/README.md
- Touch: `apps/desktop/src-tauri/src/presence/appearance_tests.rs`,
  `apps/desktop/src-tauri/src/presence/client.rs`,
  `apps/desktop/src-tauri/src/presence/mod.rs`,
  `apps/desktop/src-tauri/src/presence/watcher.rs`,
  `apps/desktop/src-tauri/src/presence/watcher/feed.rs`,
  `apps/desktop/src-tauri/src/presence/watcher_fixture.rs`,
  `apps/desktop/src-tauri/src/presence/watcher_tests.rs`,
  `apps/desktop/src-tauri/src/presence/contract.yaml`,
  `apps/desktop/src-tauri/src/windows/companion/mod.rs`,
  `apps/desktop/src-tauri/src/windows/companion/state.rs`,
  `apps/desktop/src-tauri/src/windows/companion/contract.yaml`,
  `apps/desktop/src-tauri/src/commands/desktop.rs`,
  `apps/desktop/src-tauri/src/commands/companion.rs`,
  `apps/desktop/src-tauri/src/commands/contract.yaml`,
  `apps/desktop/src-tauri/src/lib.rs`, `apps/desktop/src-tauri/build.rs`,
  `apps/desktop/src-tauri/permissions/companion.toml`,
  `apps/desktop/tests/native-acl.test.mjs`, `docs/aylul/codebase/README.md`,
  `docs/aylul/native/stages.md`
- Needs: the request (accepted)
- Done when: Task N-6's appearance, watcher, state, caller-matrix and generated-ACL checks pass;
  colour remains opaque, unavailable appearance is omitted, refresh acknowledges publication,
  and stale results cannot publish. Presence, companion and command contracts match.

### N-7 · Start with Windows, and closing the main window hides it or quits
- Status: blocked
- Blocker: steps written, waiting for the controller's review
- Plan: docs/aylul/native/plans/2026-10-21-settings-native.md#task-n-7
- Read: [the accepted request][settings-request]; native `windows/contract.yaml`,
  `commands/contract.yaml`; docs/aylul/codebase/README.md
- Touch: `apps/desktop/src-tauri/src/windows/preferences.rs`,
  `apps/desktop/src-tauri/src/windows/preferences_tests.rs`,
  `apps/desktop/src-tauri/src/commands/preferences.rs`,
  `apps/desktop/tests/desktop-preferences.mjs`,
  `apps/desktop/src-tauri/Cargo.toml`, `apps/desktop/src-tauri/Cargo.lock`,
  `apps/desktop/src-tauri/src/lib.rs`, `apps/desktop/src-tauri/src/windows/mod.rs`,
  `apps/desktop/src-tauri/src/windows/contract.yaml`,
  `apps/desktop/src-tauri/src/commands/mod.rs`,
  `apps/desktop/src-tauri/src/commands/companion.rs`,
  `apps/desktop/src-tauri/src/commands/contract.yaml`,
  `apps/desktop/src-tauri/build.rs`, `apps/desktop/src-tauri/permissions/companion.toml`,
  `apps/desktop/tests/native-acl.test.mjs`, `docs/aylul/codebase/README.md`,
  `docs/aylul/native/stages.md`
- Needs: the request (accepted)
- Done when: Task N-7's preference, caller-matrix, placement and generated-ACL checks pass;
  native close/tray/IPC acceptance passes, OS login registration is verified/restored,
  Tauri pins stay unchanged and contracts match. Report any unperformed real sign-in separately.

[settings-request]: ../shared/requests/2026-10-21-settings-dialog-native.md
