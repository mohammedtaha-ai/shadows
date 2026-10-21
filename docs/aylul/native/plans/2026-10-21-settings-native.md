# Settings: native companion appearance and desktop preferences

**Goal:** Deliver N-6 and N-7 for the settings dialog's native consumers.

**Request:**
[Accepted settings request](../../shared/requests/2026-10-21-settings-dialog-native.md).
Its Answer is binding. In particular, `closeToTray` defaults to true.

**Constraints:** Work on `improvements`; no worktree, branch switch or push. Released by the controller,
2026-10-21. Keep Core, sidecar, renderer and
web ownership intact. Use the desktop's independent Cargo manifest and existing debug cache.
Update each changed module's contract with its methods, obligations and named regressions.
Run only focused checks while building. Commands below are for implementation, not this
planning run.

## Task N-6

**Files:**

- Create: `apps/desktop/src-tauri/src/presence/appearance_tests.rs`.
- Modify: `apps/desktop/src-tauri/src/presence/client.rs`,
  `apps/desktop/src-tauri/src/presence/mod.rs`,
  `apps/desktop/src-tauri/src/presence/watcher.rs`,
  `apps/desktop/src-tauri/src/presence/watcher/feed.rs`,
  `apps/desktop/src-tauri/src/presence/contract.yaml`,
  `apps/desktop/src-tauri/src/windows/companion/mod.rs`,
  `apps/desktop/src-tauri/src/windows/companion/state.rs`,
  `apps/desktop/src-tauri/src/windows/companion/contract.yaml`,
  `apps/desktop/src-tauri/src/commands/desktop.rs`,
  `apps/desktop/src-tauri/src/commands/companion.rs`,
  `apps/desktop/src-tauri/src/commands/contract.yaml`,
  `apps/desktop/src-tauri/src/lib.rs`, `apps/desktop/src-tauri/build.rs`,
  `apps/desktop/src-tauri/permissions/companion.toml`,
  `docs/aylul/codebase/README.md`, `docs/aylul/native/stages.md`.
- Test: the new `presence/appearance_tests.rs` above;
  `apps/desktop/src-tauri/src/presence/watcher_fixture.rs`,
  `apps/desktop/src-tauri/src/presence/watcher_tests.rs`,
  `apps/desktop/src-tauri/src/windows/companion/state.rs` (existing test modules),
  `apps/desktop/src-tauri/src/commands/companion.rs` (existing caller matrix),
  `apps/desktop/tests/native-acl.test.mjs`.
- Read only: `apps/desktop/src-tauri/capabilities/main.json` and
  `apps/desktop/src-tauri/capabilities/companion.json`. They already select `main-actions` and
  `companion-actions` respectively. Extend the former permission, never the latter capability.

**Interfaces:**

- Add `presence::CompanionAppearance { pub color: String }`, deriving Clone, Debug, PartialEq and
  Serialize with camelCase wire names. Native has no palette, default colour or id validation.
  The agent DTO in `client.rs` gains `#[serde(default)] character_color: Option<String>`.
  Missing/null stays absent; every string is copied byte for byte, including unknown or empty ids.
- Add `Client::validate_appearance(&self, binding: &BindingView)` returning
  `Result<Option<CompanionAppearance>, Problem>`. Share the existing `validate_inner` agent read,
  identity/provider/key/target checks and timeout. Keep `Client::validate`'s existing unit result
  for callers that only need validation. Do not fetch the agent list twice for one check.
- `watcher::Update` carries `appearance: Option<CompanionAppearance>` separately from its
  `projection`. Feed metadata checks populate it on selection and each existing 10-second check,
  both with and without a target thread. Unbound, unavailable and disconnected updates clear it.
  Conversation-only updates retain the last validated appearance until the next metadata result.
- `windows::companion::state::CompanionState` gains
  `#[serde(skip_serializing_if = "Option::is_none")] pub appearance: Option<CompanionAppearance>`.
  `Model::present` and `windows::companion::present` accept presence plus appearance and update
  them in one revision. A colour-only change advances `revision`; `publish` sends the same
  `companion-state` shape to its existing recipients. It must not grant rendering or focus.
- Add `RefreshAppearanceReply { pub ok: bool }`, Serialize; success always sets `ok: true`.
  `commands::desktop::refresh_companion_appearance` is an async Tauri command with injected
  window/presence/watcher state, no caller arguments, returning
  `Result<RefreshAppearanceReply, String>`. Wire success is exactly `{ "ok": true }`.
- Add `PresenceWatcher::refresh_appearance`: an acknowledged request to rerun the same metadata
  check once, without replacing the binding, restarting SSE or replaying completion. Serialize
  explicit and periodic metadata checks in the watcher so an older poll cannot undo a refresh.
  A stopped feed must still allow this explicit check. Retain generation, epoch, configuration,
  runtime and current-binding guards through UI publication; acknowledge only after publication.
  If superseded, refuse with a fixed error rather than publish the former agent's colour.
- With no bound agent, republish the current state and return `{ "ok": true }` without Core I/O.
  A Core read failure returns fixed `companion_core_unavailable`, clears unavailable appearance
  through the guarded publication path and leaves normal polling/recovery behavior intact.
  Never expose transport text or mutate Core. Do not change binding preferences to store colour.
- Enforce `main_only` before work, add the command to `allowed_caller`, `generate_handler!`,
  `AppManifest::commands` and only `main-actions`. Companion/unknown labels are refused.

**Other sides:** U-1 owns parsing in `apps/desktop/src/platform/companion-state.ts`. F-14 owns
`refreshCompanionAppearance(): Promise<void>` in `web/src/desktop/presence.ts` and the successful
agent-save call. Its older plan wording says the native command returns state; the accepted
request's `{ ok: true }` is authoritative. This task builds neither wrapper nor parser. F-12 owns
General controls; U-2/U-3 own colour meaning and drawing. Dependency added: native N-6 enables
U-1 and F-14, with no new library or endpoint; Core must expose the optional `character_color`.

- [ ] **Step 1: Write failing tests first.** Register `appearance_tests` in `presence/mod.rs`;
  extend the existing HTTP fixture to vary colour, availability and delayed metadata responses.
  - `appearance_check_copies_opaque_colour`: `blue`, `teal`, empty and a 33-character string
    are returned unchanged; missing/null omits appearance; all requests remain reads.
  - `appearance_selection_and_liveness_publish_colour`: initial selection and a later metadata
    tick publish different colours, both for an agent draft and a targeted conversation.
  - `appearance_unbound_and_unavailable_are_absent`: no selection, deleted agent, missing key,
    wrong runtime and transport loss serialize no `appearance` key and retain existing status rules.
  - `appearance_refresh_waits_for_recheck_and_publication`: change fixture colour; before the
    next timer tick, refresh rereads once and acknowledges only after the new state is published.
    It neither recreates a subscription nor celebrates an earlier completed operation.
  - `appearance_refresh_unbound_republishes_without_core`: success is `{ "ok": true }`,
    the state is emitted and the HTTP request count stays zero.
  - `appearance_refresh_cannot_publish_a_superseded_binding`: hold a response, switch agent,
    configuration or runtime, then release it; no old appearance reaches the new state.
    Also delay a periodic response across refresh; the refreshed colour cannot be overwritten.
  - `appearance_refresh_failure_is_fixed_and_polling_recovers`: unreachable Core yields only
    `companion_core_unavailable`; a later successful normal check publishes the new colour.
  - In companion state tests, `appearance_changes_revision_without_render_permission`: changing
    only colour advances revision once, clearing omits the key, and hidden/failed/paused/composer
    state remains unchanged. No render permission is granted and no error is cleared.
  - Extend `companion_callers_are_restricted_for_every_operation` for the new command.
    In `native-acl.test.mjs`, add `appearance refresh is granted only to main`: generated ACL
    metadata allows main, denies companion, and includes generated command permission metadata.
- [ ] **Step 2: Run focused commands below; expect FAIL before implementation.**
- [ ] **Step 3: Implement** the shared check, watcher publication and acknowledged refresh;
  wire the command and ACL. Add the new test-file code-map row and amend the three affected
  contracts (presence, companion window, commands). No parent windows behavior changes here.
- [ ] **Step 4: Run the focused commands; expect PASS.** From the repository root, PowerShell:

  ```powershell
  $nativeManifest = 'apps/desktop/src-tauri/Cargo.toml'
  cargo test --manifest-path $nativeManifest appearance
  cargo test --manifest-path $nativeManifest presence::watcher_tests
  cargo test --manifest-path $nativeManifest windows::companion::state
  cargo test --manifest-path $nativeManifest companion_callers_are_restricted_for_every_operation
  node --test apps/desktop/tests/native-acl.test.mjs
  cargo fmt --manifest-path $nativeManifest --check
  git diff --check
  ```

  Run the ACL test after Cargo regenerates its schema metadata; do not hand-edit generated files.
- [ ] **Step 5: Done when** the named tests pass and all three public contracts describe the
  new behavior. Report fixture coverage separately from real companion repaint acceptance,
  which requires U-1/U-3 and F-14. Set N-6's stage status and commit its exact files.

## Task N-7

**Files:**

- Create: `apps/desktop/src-tauri/src/windows/preferences.rs`,
  `apps/desktop/src-tauri/src/windows/preferences_tests.rs`,
  `apps/desktop/src-tauri/src/commands/preferences.rs`,
  `apps/desktop/tests/desktop-preferences.mjs`.
- Modify: `apps/desktop/src-tauri/Cargo.toml`, `apps/desktop/src-tauri/Cargo.lock`,
  `apps/desktop/src-tauri/src/lib.rs`, `apps/desktop/src-tauri/src/windows/mod.rs`,
  `apps/desktop/src-tauri/src/windows/contract.yaml`,
  `apps/desktop/src-tauri/src/commands/mod.rs`,
  `apps/desktop/src-tauri/src/commands/companion.rs`,
  `apps/desktop/src-tauri/src/commands/contract.yaml`,
  `apps/desktop/src-tauri/build.rs`, `apps/desktop/src-tauri/permissions/companion.toml`,
  `docs/aylul/codebase/README.md`, `docs/aylul/native/stages.md`.
- Test: the new preferences test module and native driver above;
  `apps/desktop/tests/native-acl.test.mjs`,
  `apps/desktop/src-tauri/src/commands/companion.rs` (caller matrix).
- Read only: both capability JSON files, `apps/desktop/src-tauri/src/app_paths.rs`,
  `apps/desktop/tests/windows-support.ps1`, `apps/desktop/tests/tray-menu.ps1` and
  `apps/desktop/tests/companion-native.mjs` (existing native acceptance patterns).

**Interfaces:**

- `windows::preferences::DesktopPreferences { pub launch_at_login: bool, pub close_to_tray: bool }`
  derives Serialize/Deserialize with `#[serde(rename_all = "camelCase")]`.
  Fresh-install defaults: `{ "launchAtLogin": false, "closeToTray": true }`.
- `windows::preferences::Store` owns serialized preference changes and the close-policy snapshot.
  Initialize it from `AppPaths::root().join("desktop-preferences-v1.json")` before window setup;
  use the existing app-data root and acceptance override, never Core or sidecar storage.
  Store the full camelCase preference object. A missing file gives defaults; malformed data or
  write failures produce fixed errors and must not silently replace an existing file.
  Keep close-to-tray true if loading fails so startup remains usable and preserves today's close.
  Preference commands report the load error until repaired; do not hide it as a successful save.
- `commands::preferences::desktop_preferences(window, app, store)` returns
  `Result<DesktopPreferences, String>` with only injected Tauri parameters and no wire arguments.
  `commands::preferences::set_desktop_preferences` takes the same injected parameters plus
  `launch_at_login` and `close_to_tray`. Both commands are async; their injected types are
  `window: tauri::WebviewWindow`, `app: tauri::AppHandle`, and `store: tauri::State<'_, Store>`.
  The setter returns `Result<DesktopPreferences, String>`; the two caller parameters are
  `Option<bool>`. Tauri command arguments use camelCase. Invoke arguments are the flat object
  `{ launchAtLogin?: boolean, closeToTray?: boolean }`, with no `preferences` nesting.
  Empty input is a read/no-op; a partial update retains the omitted value. Success returns the
  full `{ launchAtLogin: boolean, closeToTray: boolean }` after saving.
- Both commands guard main before accessing OS/storage, join `allowed_caller`, `generate_handler!`,
  `AppManifest::commands` and only `main-actions`. Keep both capability window lists unchanged.
  Do not grant `autostart:*` plugin permissions to either webview: only these native wrappers
  expose the behavior. Unknown and companion callers remain refused.
- Pin `tauri-plugin-autostart = "=2.5.1"` in the independent desktop manifest. This is an
  infrastructure dependency for OS login registration, not product rules or UI validation.
  Its [published manifest](https://docs.rs/crate/tauri-plugin-autostart/2.5.1/source/Cargo.toml)
  requires Tauri `2.8.2` (a compatible range including the pinned `=2.11.5`). Keep the existing
  runtime `=2.11.3`, runtime-wry `=2.11.4` and other compatibility pins; do not upgrade Tauri.
  Regenerate the desktop lockfile with Cargo during implementation and verify those pins remain.
- Register `tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None)` in `lib.rs` before
  setup (`MacosLauncher` is the plugin enum, also required by its cross-platform initializer).
  Use `tauri_plugin_autostart::ManagerExt::autolaunch` and its `is_enabled`, `enable`, `disable`
  methods from Rust. See the [Rust API][autostart-api].
  No JavaScript package is needed. Reads query the actual OS login item; reading preferences
  or starting the app never enables it. Reconcile externally changed OS state in the returned view.
- Serialize read/modify/save against concurrent setters. On setter success, confirm OS state,
  safely replace the saved file, then publish the in-memory close policy. On OS refusal, keep
  close policy/file unchanged; return fixed `desktop_preferences_autostart_failed`. On storage
  failure, return `desktop_preferences_save_failed`, retain the last saved close policy and
  attempt to restore any changed login item. Requery OS rather than claim rollback succeeded.
  Reads fail with `desktop_preferences_load_failed` or `desktop_preferences_autostart_failed`;
  callers can reread the actual values after a failed setter. Never return OS paths/messages.
- `windows/mod.rs` reads the managed close policy for every main `CloseRequested` event:
  prevent close and hide when true; prevent close and call `app.exit(0)` when false, so a
  companion window cannot keep the desktop alive. Tray `quit` still calls `app.exit(0)` directly
  regardless of either preference. Keep existing Exit cleanup for watcher and sidecar.

**Other sides:** F-12 owns web wrappers for `desktop_preferences` and `set_desktop_preferences`,
plus General switches, errors and rereading values after failure. Wrapper paths/names are its
choice.
This task builds no web wrapper, settings UI or U-1 companion parser. F-14's save refresh is N-6.
Dependency added: N-7 enables F-12's two desktop-only controls and adds the Rust infrastructure
plugin above; it has no dependency on N-6, Core preferences, or a new backend endpoint.

- [ ] **Step 1: Write failing tests first.** Keep persistence tests in `preferences_tests.rs`,
  registered by `preferences.rs`; use a temporary root and a fake login-item adapter so unit
  tests cannot register the developer's executable with Windows.
  - `desktop_preferences_fresh_install_preserves_close_to_tray`: missing file and disabled OS
    item yield false/true; no write or enable occurs on read.
  - `desktop_preferences_partial_update_survives_restart`: each boolean can change alone;
    omitted values survive, empty input changes nothing, and a new Store reads the saved values.
    Assert exact camelCase JSON without snake_case keys.
  - `desktop_preferences_reports_external_login_change`: change fake OS state independently;
    reads report the OS value without enabling/disabling it to match an old file.
  - `desktop_preferences_os_refusal_preserves_saved_close_policy`: fake enable/disable fails;
    setter returns the fixed code, saved close flag/file remain unchanged and reread is truthful.
  - `desktop_preferences_corrupt_or_unwritable_store_is_not_overwritten`: corrupted JSON is
    preserved and reported; failed writes leave the prior close policy active. If an OS update
    preceded failure, assert rollback attempt and truthful subsequent OS read, including failed
    rollback. Concurrent partial setters cannot lose the other's accepted change.
  - `desktop_preferences_close_policy_hides_or_exits`: true chooses prevent/hide; false chooses
    prevent/application-exit; tray Quit always chooses application-exit for both values.
  - Extend `companion_callers_are_restricted_for_every_operation` for both commands. Add
    `desktop preferences are granted only to main` to `native-acl.test.mjs`: both are generated
    and main-only; neither surface receives direct autostart plugin permission.
  - In `desktop-preferences.mjs`, add native cases `main can read and patch preferences`,
    `companion cannot invoke preferences`, `close to tray survives restart`,
    `close without tray exits with companion visible`, and `tray Quit ignores close preference`.
    Use test-owned WebView2 and isolated acceptance app data; assert actual visibility/process
    lifetime as well as command responses. Accept `--pid`, `--main-port`, `--companion-port`.
- [ ] **Step 2: Run focused unit/ACL checks below; expect FAIL before implementation.**
- [ ] **Step 3: Implement** the native store and login adapter, compose the plugin/store before
  `windows::setup`, wire commands/ACL and the main close callback. Add code-map rows for new
  modules and tests, and amend windows/commands contracts. Keep window geometry/tray Open intact.
- [ ] **Step 4: Run the focused checks; expect PASS.** From repository root, PowerShell:

  ```powershell
  $nativeManifest = 'apps/desktop/src-tauri/Cargo.toml'
  cargo test --manifest-path $nativeManifest desktop_preferences
  cargo test --manifest-path $nativeManifest companion_callers_are_restricted_for_every_operation
  cargo test --manifest-path $nativeManifest windows::placement
  node --test apps/desktop/tests/native-acl.test.mjs
  cargo fmt --manifest-path $nativeManifest --check
  cargo tree --manifest-path $nativeManifest -p tauri-plugin-autostart
  git diff --check
  ```

  Inspect the lockfile diff and tree for autostart 2.5.1 with the unchanged Tauri/runtime pins.
- [ ] **Step 5: Native acceptance.** Build the debug executable using the existing desktop asset
  assembly workflow. Start it with an isolated existing `--acceptance-root` and
  `--acceptance-debug-port`; the companion uses the following port. With that process running:

  ```powershell
  $driver = 'apps/desktop/tests/desktop-preferences.mjs'
  node $driver --pid <desktop-pid> --main-port <port> --companion-port <next-port>
  ```

  Substitute the actual PID/ports. Do NOT enable or disable the real Windows login item in this
  run: it changes the person's own system settings, so the controller reserves it for Mohammed's
  acceptance (controller's ruling, 2026-10-21). Report real login registration and sign-in
  launch as unverified, for Mohammed. Do not
  infer native behavior from fake adapter tests. Keep logs/results in the implementation report.
- [ ] **Step 6: Done when** focused checks and actual close/tray/IPC acceptance pass, OS login
  registration is left to Mohammed and reported as such, and contracts/code map match the behavior.
  Report any unperformed Windows sign-in journey explicitly. Set N-7's status and commit exact
  files.

[autostart-api]: https://docs.rs/tauri-plugin-autostart/2.5.1/tauri_plugin_autostart/
