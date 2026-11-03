# Desktop UI stages

Where this side's work is now. Masada keeps this file; builders read
their task here and set its status. Task shape: [`AGENTS.md`](../AGENTS.md#a-task-in-stagesmd).

## Now

The panel is UI-owned (agents and panel design §7.8); the
character side owns only the drawing. Finished tasks are in Git: U-5 to U-8 at `b40e077b`
(PR #49), U-4 at `ca618f89`, U-1 and U-3 at `1a2b601`, U-2 at `c7e7bac4`, the text input,
the shared dashboard, U-W1 to U-W3 and H-2 at `8580306`.

## Tasks

U-9 and U-10 are built on `docs/agent-messaging-design`. The physical Windows journey is
tracked in the [group UI plan](../../frontend/plans/2026-11-03-agent-groups-ui.md#acceptance).

### U-9 · Open a group from the desktop
- Status: done
- Plan: docs/frontend/plans/2026-11-03-agent-groups-ui.md#task-u-9-open-a-group-from-the-desktop
- Read: apps/desktop/src-tauri/src/contract.yaml; web/src/desktop/contract.yaml
- Touch: apps/desktop/src-tauri/src/navigation.rs, contract.yaml; the web side's pending
  navigation handler: web/src/desktop/navigation.ts, index.ts, native-parity.test.ts,
  contract.yaml; web/src/main.tsx
- Needs: F-84 (the route `/aylul/groups/$groupId`)
- Done when: `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml navigation`
  (`a_group_destination_uses_the_closed_wire_shape`) and `npx vitest run
  src/desktop/native-parity.test.ts` (`a group destination opens the group page`) pass.

### U-10 · "Needs you" on the panel, and the alert
- Status: done
- Plan: docs/frontend/plans/2026-11-03-agent-groups-ui.md#task-u-10-needs-you-on-the-panel-and-the-alert
- Read: apps/desktop/src-tauri/src/presence/contract.yaml; messaging spec §6.7
- Touch: apps/desktop/src-tauri/src/presence/ (mod.rs, waiting_items.rs, watcher.rs,
  watcher/feed.rs, core_client/mod.rs, core_client/blocking.rs, core_client/observer.rs,
  watcher_fixture.rs, watcher_tests.rs, binding_tests.rs, contract.yaml);
  apps/desktop/src-tauri/src/windows/companion/surface.rs, contract.yaml;
  apps/desktop/src/features/companion/DesktopCompanion.tsx, panel/CompanionPanel.tsx,
  contract.yaml; apps/desktop/src/platform/companion-state.ts, companion-state.test.ts,
  contract.yaml; docs/aylul/codebase/README.md
- Widened (recorded): the `MessageAdded` payload gained `calls_person`, so the watcher alerts
  without reading message history: crates/core-messages/src/store/write.rs, contract.yaml;
  crates/core-events/contract.yaml; crates/shadows/tests/group_routes.rs; messaging spec §6.7.
  The watcher also runs with no bound agent, so the pill counts groups before a binding.
- Needs: B-46 (`Group.needs_you_since`)
- Done when: `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml presence` passes,
  with `a_message_to_mohammed_raises_the_waiting_state`,
  `a_group_with_needs_you_and_two_requests_counts_once`,
  `a_reconnect_restores_the_count_without_an_old_tone`,
  `a_new_call_rings_while_an_older_request_waits`, `an_older_request_closing_rings_nothing`,
  `startup_is_silent_and_a_call_during_a_drop_rings_once`, `a_message_to_others_does_not`,
  `the_pill_counts_a_group_once` and `the_oldest_waiting_group_opens_the_group_page`.
