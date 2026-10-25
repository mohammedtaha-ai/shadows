# N-10 ratchet audit (2026-10-25)

The review of `4ae9d466` (D-3, one `DesktopApp`) checked every baseline entry the change
removed. Fixed in `8c70631c`.


This audits every entry removed relative to `4ae9d466^`. Ordinals refer to that
parent baseline; ranges include every ordinal. A removed written type/import is
distinguished from an owner dependency that survives through an accessor.

## `owner_edge`: 34 removed, 28 legitimate removals, 6 hidden accesses

Commands may use DesktopApp under D-0. Their direct owner type/import references
disappear; the typed accessor dependency belongs to composition. These 26 removed
entries are legitimate, without claiming that the workflows have moved behind
their final owners (D-7):

| File under `src/` | Removed target and ordinals | Source evidence |
|---|---|---|
| commands/companion.rs | navigation::Navigation #1 | open_main_from_companion uses root.navigation |
| commands/companion.rs | presence::Presence #1–2 | companion_send/open_main_from_companion use root.presence |
| commands/desktop.rs | core_connection::Connection #1–5 | Import gone; four connection commands use root.connection |
| commands/desktop.rs | navigation::Navigation #1–2 | navigation_snapshot/ack_navigation use root.navigation |
| commands/desktop.rs | presence::Presence #1–5 | Five binding/appearance commands use root.presence |
| commands/desktop.rs | presence::watcher::PresenceWatcher #1–2 | target_companion_thread/refresh_companion_appearance use root.watcher |
| commands/mod.rs | auth::AuthController #1–2 | auth_action/cancel_login use root.auth |
| commands/mod.rs | sidecar::SidecarState #1 | aylul_call uses root.sidecar |
| commands/preferences.rs | windows::preferences::PluginLoginItem #1–3 | Import gone; two worker calls use root.login_item |
| commands/preferences.rs | windows::preferences::Store #1–3 | Import gone; two commands use root.preferences |

The remaining eight entries require individual treatment:

| File under `src/` | Removed target and ordinals | Verdict / repair |
|---|---|---|
| windows/companion/input.rs | presence::Presence #1 | Hidden: refresh still obtains Presence.context/view through root.presence; restored identically |
| windows/companion/mod.rs | app_paths::AppPaths #1–2 | Hidden: setup_companion still reads webview/browser_args through root.paths; restored identically |
| windows/main_policy.rs | app_paths::AppPaths #1 | Real removal: the use import is gone |
| windows/main_policy.rs | app_paths::AppPaths #2–3 | Hidden: build still reads webview/browser_args through root.paths; restored as #1–2 |
| windows/main_policy.rs | core_connection::Connection #1 | Real removal: the use import is gone |
| windows/main_policy.rs | core_connection::Connection #2 | Hidden: build still captures Connection for CSP policy through root.connection; restored as #1 |

The blanket MainWindow/CompanionWindow → composition allowances are removed.
Seventeen explicit DesktopApp type references in window code are additionally
baselined (input.rs: 4, companion/mod.rs: 9, main_policy.rs: 3, windows/mod.rs: 1).
They are existing source debt, not permission to discover owners through a root.

## `private_access`: 9 removed, 6 legitimate removals, 2 relocations, 1 hidden access

| File under `src/` | Removed target and ordinals | Verdict / source evidence |
|---|---|---|
| commands/desktop.rs | presence::watcher::PresenceWatcher #1–2 | Real: the two command parameters are root parameters; watcher access uses the allowed root entry |
| commands/preferences.rs | windows::preferences::Store #1–3 | Real: import/two parameters gone; commands use the allowed root entry |
| lib.rs | presence::watcher::PresenceWatcher #1 | Real: explicit managed-state shutdown type gone; root holds/exposes its lifetime type under N-10's composition exception |
| lib.rs | presence::watcher::PresenceWatcher::new #1 | Relocated unchanged to app.rs; constructor remains a private implementation reference |
| lib.rs | windows::companion::input::refresh #1 | Relocated unchanged to app.rs; notifier remains a private implementation reference |
| windows/mod.rs | windows::preferences::Store #1 | Hidden: setup's close callback still calls private Store.close_to_tray through root.preferences; restored identically |

## Other rules: every removed entry

| Rule / file under `src/` | Removed target and ordinals | Verdict / source evidence |
|---|---|---|
| construction / commands/preferences.rs | windows::preferences::PluginLoginItem #1–2 | Two violations resolved: construction is in DesktopApp.login_item; worker invocation timing is retained |
| managed_state / commands/companion.rs | Navigation state #1; Presence state #1–2 | Three one-for-one relocations: navigation root.state #1 and two injected root States |
| managed_state / commands/desktop.rs | tauri::State #12–13 | Two lookups genuinely consolidated: target_companion_thread and refresh_companion_appearance each inject one root instead of separate Presence/watcher values; 13 → 11 |
| managed_state / windows/companion/input.rs | Store state #1–3; Presence state #1 | Four one-for-one root-state relocations, at fit's two origins, refresh and set_input |
| managed_state / windows/companion/mod.rs | Store state #1–7; AppPaths state #1–2 | Nine one-for-one root-state relocations at unchanged model/tray/rebuild/placement/visibility operations |
| managed_state / windows/main_policy.rs | AppPaths state #1–2; Connection state #1 | Three one-for-one root-state relocations in build |
| managed_state / windows/mod.rs | preferences::Store state #1 | One-for-one root-state relocation in the close callback |
| owner_cycle / windows/companion/input.rs | CompanionWindow → Companion (cycle edge) | Hidden, restored: refresh still consumes Presence; the return edge remains in presence/binding_tests.rs |

Across all 68 removed entries: **38 legitimate removals, 22 relocations, 8 scanner
blind spots**. The owner-edge reduction alone was 28 legitimate removals and six
blind spots. Private-access reduction was six legitimate removals, two relocations
and one blind spot. No remaining violation is described as a product fix.

## Behavior and lifetime evidence

- `DesktopApp::start` runs only in `lib::run`'s Tauri setup closure, before
  windows::setup. Actual constructor order is unchanged: resolve paths,
  preferences, account, companion Store, connection, Presence, watcher, Navigation,
  sidecar. This is the parent's constructor order; its manage order differed
  because connection/Presence were built before registration. Field declaration
  order matches the nine old registrations: preferences, account, companion,
  watcher, Presence, Navigation, connection, paths, sidecar.
- The watcher publisher, both notifiers, their weak Presence/Connection captures,
  binding guards, publication acknowledgement and retry-before-sidecar sequence
  retain their operations/order. Accessor methods borrow the same owners. No owner
  stores DesktopApp, nor does a notifier capture Arc<DesktopApp>; preference
  workers hold a temporary root Arc only for their existing blocking operation.
- Window mutex guards retain the same full-expression/local-block lifetimes.
  Tauri 2.11.5's StateManager::try_get (`src/state.rs` in the locked dependency)
  returns a borrowed State after releasing its registry mutex; retaining State or
  a root borrow does not retain that mutex. No new guard crosses an await/call.
- Event strings, filenames, path helpers, window labels, capability/handler
  inventory and command guards are unchanged. PluginLoginItem is still created
  inside each preference worker, immediately before read/set.
- Explicit Exit shutdown remains watcher.stop then sidecar.stop. The old Tauri
  StateManager used a HashMap of separately managed values, so it guaranteed no
  insertion-based drop order. DesktopApp fields now have declaration-order drops;
  owner dependency lifetimes still use Arc, and shutdown effects remain explicit.
  Window stores/paths have ordinary owned-data drops; this change adds no custom
  Drop and no new strong owner cycle. Physical native acceptance is separate.
- The root has nine private fields and shared typed accessors, no mutable-reference
  accessor. `app.manage(desktop)` is the only registration and start is called once.
  Comments in the touched production source describe purpose/constraints, not
  migration history. The root/commands/window contracts describe these accessors;
  outstanding owner/workflow debt remains in the baseline for D-4/D-5/D-7.

Reviewed baseline: construction 37, coverage 1, managed_state 36, owner_cycle 6,
owner_edge 82, private_access 51; command_inventory/core_http/unresolved_import 0.
