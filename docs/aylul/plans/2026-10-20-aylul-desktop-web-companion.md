# AYLUL Desktop Web and Companion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Run the existing web dashboard as the desktop main interface, start its local Core automatically, and connect the independent character to a changeable AYLUL agent.

**Architecture:** Package the `web/` build beside the existing companion entry. Native code owns daemon startup, a persisted agent/thread binding and one background presence subscription; the existing Core owns every conversation and turn. The character consumes a bounded native presentation and opens the normal chat.

**Tech Stack:** Existing React/TanStack/Vite, Tauri 2.11.5, Rust, axum/utoipa, reqwest; add `eventsource-stream = "=0.2.3"` as SSE infrastructure in the native workspace.

**Spec:** [Approved design](../specs/2026-10-20-aylul-desktop-web-companion-design.md), especially §§5, 7–9. Mohammed approved inline execution and a final same-model review on 2026-10-20. Execution is in progress.

## Global Constraints

- Preserve `.mcp.json`. Use `codex/aylul-desktop-web-companion`, inline execution with `superpowers:executing-plans`, one completed-task commit, one final fresh-context reviewer on this session's model (latest user instruction), and no push/merge/deployment.
- `web/` is the sole main UI source; companion source remains under `apps/desktop/`. No iframe, duplicated dashboard or second assistant execution path.
- Use `shadows serve` with explicit absolute executable/configuration paths. No PATH discovery, shell command construction, Hermes startup, automatic DB creation or source/port fallback.
- Core survives desktop Quit. Existing harness children retain their kill-on-owner-close guarantee. Hook-sidecar lifetime stays unchanged.
- Keep `aylul.json` as `{ "home_root": ... }`. Native launch settings and binding preferences contain references/IDs, never provider keys or stored conversation text.
- One fixed selected agent, changeable in Settings; viewing a conversation never changes it. No microphone, TTS or mini-chat composer.
- Preview: at most 240 Unicode characters, two visible lines, eight-second lifetime, no duplicate-ID extension. Success expression: two seconds. Native hide/pause/render permission wins.
- Core API changes are Rust-owned and generated through OpenAPI; never hand-edit `api/openapi.json` or `web/src/api/schema.d.ts`. Native IPC has parity tests.
- Read maps/contracts before code. Update changed contracts/maps in the implementation commit. Source files crossing 300 lines name their one job; split at 500 by responsibility.
- Reuse existing default Cargo caches. No alternate target directories, worktree build, clean, release, profile or RUSTFLAGS changes. Do not stop an unrelated running application to unlock its executable.
- Run touched checks per task; full gates once in Task 9. Probe output is evidence, not acceptance of a real provider or physical Windows behavior.
- All task paths are repository-relative; run commands from the repository root unless stated otherwise. Every task's Files list is its Touch list. A native test filter below means `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml <filter>`; native and root workspaces retain their existing default caches.
- If actual source requires another file, amend this plan's Touch list with the reason before editing. Routine caller fixes within the named modules do not authorize unrelated cleanup.

## Review Focus

1. Executable/config paths containing spaces, Arabic or shell metacharacters must arrive as unchanged arguments; an inherited provider token must not reach the daemon (Tasks 2, 4).
2. A stale receipt, reused PID, changed configuration or occupied port must never attach to the wrong process/DB or allow an older launch result to replace a newer one (Tasks 3, 4).
3. Agent A finishing after selection changes to B, including main reload and old navigation acknowledgement, must not affect B or clear its pending navigation (Tasks 6, 7).
4. Forked entries retain source operation IDs; copied replies and initial history must not become a fresh character reply (Task 7).
5. Terminal status arriving before its final entry, Arabic/emoji previews and expiry while hidden must not produce a partial, duplicated or revived bubble (Tasks 7, 8).

## File structure and cross-owner agreements

| Location | One job |
|---|---|
| `crates/shadows-process/src/persistent.rs` | Explicit persistent process launch |
| `crates/shadows/src/runtime_record.rs` | Private atomic runtime-record writing |
| `crates/shadows/src/desktop_runtime.rs` | Core desktop launch receipt |
| `crates/shadows-http/src/runtime.rs` | Public readiness projection |
| `apps/desktop/src-tauri/src/core_connection/` | Native Core connection lifetime |
| `apps/desktop/src-tauri/src/presence/` | Agent-bound conversation presence |
| `apps/desktop/src-tauri/src/navigation.rs` | Pending closed main destination |
| `web/src/desktop/` | Typed native integration boundary |
| `web/src/app/desktop/` | Desktop-specific dashboard sections |
| `apps/desktop/scripts/assemble-desktop.mjs` | Assembly of generated main/companion assets |
| Existing companion feature | Character drawing from the native presentation |

Before the corresponding producer task starts, the controller records the reviewed interfaces below as accepted agreements:
- Core/native: `docs/shared/requests/2026-10-20-desktop-runtime-identity.md`, Task 3's request/receipt/readiness shapes.
- Native/UI/character: `docs/aylul/shared/requests/2026-10-20-desktop-main-presence.md`, Tasks 4, 6 and 7's IPC, generation and navigation shapes.

Shared documentation Touch paths for the named implementation tasks: `docs/codebase/README.md`, `docs/frontend/codebase/README.md`, `docs/aylul/codebase/README.md`, `apps/desktop/src-tauri/src/commands/contract.yaml`, `apps/desktop/src-tauri/src/windows/contract.yaml`, `apps/desktop/src-tauri/src/windows/companion/contract.yaml`, `apps/desktop/src/app/contract.yaml`, `apps/desktop/src/platform/contract.yaml`, `apps/desktop/src/features/contract.yaml`, `web/src/desktop/contract.yaml`, `web/src/app/desktop/contract.yaml`, `web/src/api/contract.yaml`, `web/src/app/settings/contract.yaml`, `web/src/app/sidebar/contract.yaml`, `web/src/app/conversation/contract.yaml`. Update only the ownership/behavior actually changed by the task.

Use each directory's request template. Approval of this plan is the basis for the controller's ruling; do not invent an approval by another lead or send an unrequested message. Add task links to `docs/aylul/{ui,native,character}/stages.md` and relevant Core stages before dispatching a domain builder. This plan preserves inline execution, so no builder dispatch is required.

---

### Task 1: Measure packaged WebView2 access and close the OPEN

**Files:**
- Create: `docs/evidence/aylul/2026-10-20-desktop-webview-origin.md`, `docs/evidence/aylul/2026-10-20-desktop-webview-origin.json`.
- Modify: spec §5's OPEN block.
- Temporary probe source: `apps/desktop/src-tauri/examples/desktop_origin_probe.rs`, `apps/desktop/src-tauri/examples/desktop_origin_probe.js`, `apps/desktop/tests/desktop-origin-probe.{ps1,mjs}`; generated assets/results under `.superpowers/sdd/2026-10-20-aylul-desktop-web-companion/origin-probe/`. Temporarily link the existing resource.lib to examples in `apps/desktop/src-tauri/build.rs`; restore that probe-only hook with source cleanup. Remove probe source after recording its Git provenance.

**Interfaces:** Produces evidence values `main_origin: string`, `companion_origin: string`, GET/JSON/SSE results, effective CSP and per-window native-command results. Task 3 consumes the observed main origin; Task 4 consumes the tested CSP/build boundary.

- [x] **Step 1:** Build a temporary native example with `cargo build --locked --manifest-path apps/desktop/src-tauri/Cargo.toml --example desktop_origin_probe`. Its PowerShell driver reads an explicit `-Config <absolute fixture JSON path>`, assembles the real web build and existing companion as generated local assets, launches an isolated Core/fake-ACP fixture, and collects HTTP Origin and native command results from the two owned windows. Use the existing capability files and local WebviewWindowBuilder/response hook; no extra Cargo workspace or alternate target directory. Commit the short-lived probe source as `test(desktop): add packaged-origin probe`, then run `powershell -NoProfile -File apps/desktop/tests/desktop-origin-probe.ps1 -Config <fixture config>`. Capture these assertions:

~~~ts
expect(result.get).toBe(200)
expect(result.jsonMutation).toBe(200)
expect(result.sseCaughtUp).toBe(true)
expect(result.mainAllowedCommand).toBe(true)
expect(result.companionMainCommand).toBe(false)
expect(result.foreignOriginRejected).toBe(true)
expect(result.nativeOrigin).not.toBe('null')
~~~

Record packaged reload/missing-asset behavior and dashboard layout/font/inline-style needs under CSP. Task 4 implements and asserts the final hash-route/missing-asset policy; those screens need not exist in this probe. Measure actual request Origin; do not infer it from the displayed URL.

- [x] **Step 2:** Exercise selected-endpoint CSP using Tauri's local-resource response hook. Preserve generated script nonces; allow neither script unsafe-inline nor unsafe-eval. A required inline style attribute may be permitted specifically as style-src-attr, with its result recorded.
- [x] **Step 3:** PASS requires all assertions and a local packaged main retaining its native ACL. Record raw credential-free results, versions and commands. If it fails, stop dependent tasks and amend the spec for review; never widen origins to wildcard/null.
- [x] **Step 4:** Remove temporary source, name the probe-source commit and exact invocation in the evidence, and close the spec OPEN with measured values. If a locked personal executable prevents the probe, report the lock rather than building elsewhere or closing that app.
- [x] **Step 5:** Commit `docs: measure packaged desktop origin and Core access`.

### Task 2: Launch a persistent daemon without weakening managed children

**Files:**
- Create: `crates/shadows-process/src/persistent.rs`, `crates/shadows-process/tests/persistent.rs`.
- Modify: `crates/shadows-process/src/lib.rs`, `crates/shadows-process/src/bin/tree_probe.rs`, `crates/shadows-process/Cargo.toml`, `Cargo.lock`, `docs/codebase/README.md`.
- Amend in place: `docs/superpowers/specs/2026-09-21-architecture-design.md` §1.5's managed-process containment scope, pointing to the approved desktop-daemon lifetime owner.

**Interfaces:**
- Existing `spawn(ProcessSpec) -> io::Result<ProcessHandle>` is unchanged.
- Produce `spawn_persistent(spec: ProcessSpec) -> io::Result<ProcessIdentity>` and `inspect_process(pid: u32) -> io::Result<Option<ProcessIdentity>>`.
- `ProcessIdentity { pid: u32, started_at: u64, executable: PathBuf }`; creation time and image come from the OS, not the caller. `is_current(&self) -> io::Result<bool>` centralizes instance comparison for the native receipt consumer.
- Persistent launch requires both pipe flags false, null stdio, explicit environment, hidden Windows startup and no kill-on-drop containment. It has no turn or harness policy.

- [x] **Step 1:** Add process-fixture tests with these assertions:

~~~rust
assert_eq!(reported_args, ["C:\\Work Space\\عربي", "& literal"]);
assert!(!reported_environment.contains_key("ANTHROPIC_AUTH_TOKEN"));
assert!(inspect_process(identity.pid)?.is_some()); // after launcher exits
assert_ne!(identity.started_at, 0);
assert_ne!(identity.started_at, reused_pid_creation_time);
~~~

Tests: `persistent_child_survives_launcher_exit`, `persistent_launch_keeps_argument_boundaries`, `persistent_launch_has_explicit_environment`, `persistent_launch_rejects_pipes`, `process_identity_rejects_reused_pid`. Use the existing test-only tree_probe, reporting arguments/allowlisted environment through an owned JSON fixture file rather than piped stdio; explicitly terminate only the fixture PID after assertions.

- [x] **Step 2:** Run `cargo test -p shadows-process --test persistent`; expected RED for the absent API.
- [x] **Step 3:** Implement the persistent path separately from `spawn`; use existing OS/process infrastructure. Add the narrow top-level daemon exception to the indexed §1.5 owner; no harness may call it.
- [x] **Step 4:** Run `cargo test -p shadows-process --test persistent --test containment` and `cargo test -p shadows --test codemap`; expected PASS, including existing kill-on-owner-close tests.
- [x] **Step 5:** Commit `feat(process): explicit persistent local daemon launch`.

### Task 3: Core runtime identity, private receipt and exact native origin

**Files:**
- Create: `crates/shadows/src/runtime_record.rs`, `crates/shadows/src/desktop_runtime.rs`, `crates/shadows-http/src/runtime.rs`, `crates/shadows/tests/desktop_runtime.rs`.
- Modify: `crates/shadows/src/{lib.rs,config.rs}`, `crates/shadows/src/cli/{args.rs,mod.rs}`, `crates/shadows/src/daemon/descriptor.rs`, `crates/shadows/src/daemon/contract.yaml`.
- Modify: `crates/shadows-http/src/{lib.rs,openapi.rs}`, `crates/shadows/tests/openapi.rs`, `crates/shadows/tests/openapi/read_only.rs`, `crates/shadows/Cargo.toml`, `Cargo.lock`.
- Generate: `api/openapi.json`, `web/src/api/schema.d.ts`.
- Update: Core/native request named above, `docs/codebase/README.md`, `web/src/api/contract.yaml`.

**Interfaces:**
- Add paired optional Serve flags `--desktop-record <absolute path>`, `--desktop-configuration <UUID>`; either both or neither. Normal Serve remains valid.
- `DesktopLaunch { record_path: PathBuf, configuration_id: String }` becomes `Config.desktop_launch: Option<DesktopLaunch>`.
- HTTP `RuntimeIdentity { runtime_id: String, configuration_id: String }`, JSON snake_case, GET `/api/runtime`: 200 when desktop launch metadata is installed, otherwise empty 404.
- Install identity using a router Extension, not another field on every AppState fixture or a new AppCore service.
- Private `RuntimeReceipt` adds `origin: String`, `process: ProcessIdentity` to that identity. The binary generates runtime_id, claims a lifetime receipt-owner lock, binds/starts Core, then writes the receipt atomically.
- Extract private atomic JSON writing from the existing descriptor implementation for its second caller; preserve the old Descriptor wire format and DACL behavior.

- [ ] **Step 1:** Add `desktop_flags_are_paired`, `runtime_identity_is_installed_after_core_ready`, `receipt_matches_bound_endpoint_and_process`, `runtime_identity_does_not_expose_configuration`, `runtime_receipt_owner_is_exclusive`, `desktop_metadata_rejects_non_loopback_bind`, `native_origin_is_added_without_replacing_browser_origins`.

~~~rust
assert_eq!(response.status(), StatusCode::OK);
assert_eq!(keys(&body), ["configuration_id", "runtime_id"]);
assert_eq!(receipt.process, inspect_process(receipt.process.pid)?.unwrap());
assert_eq!(foreign_origin_status, StatusCode::FORBIDDEN);
assert!(browser_origin_still_allowed);
~~~

Check absence of executable/DB paths, args and tokens in the public projection, and preserve private-descriptor replacement tests. Use Task 1's exact native origin; reject null and unrelated loopback web-page origins.

- [ ] **Step 2:** Run `cargo test -p shadows --test desktop_runtime`; expected RED.
- [ ] **Step 3:** Implement metadata and receipt lifecycle, validated UUID/path flags, desktop-only loopback binding and exact origin union. Remove a receipt only if it still belongs to this runtime. Add the guarded route through the canonical HTTP route table; keep metadata rule ownership in the binary.
- [ ] **Step 4:** Run `cargo test -p shadows --test desktop_runtime --test assistant_daemon`, `cargo test -p shadows --lib cli::args::tests`, `cargo test -p shadows-http guard::` and `cargo test -p shadows --test codemap`. Generate with PowerShell: `$env:UPDATE_OPENAPI='1'; cargo test -p shadows --test openapi; Remove-Item Env:UPDATE_OPENAPI`, then `npm --prefix web run gen:api`. Inject identity into the read-only OpenAPI fixture so it exercises the 200 route. Expected PASS and canonical artifacts.
- [ ] **Step 5:** Commit `feat(runtime): identify desktop Core launches and publish readiness`.

### Task 4: Auto-start Core and package the shared dashboard

**Files:**
- Create native: `apps/desktop/src-tauri/src/core_connection/{mod.rs,config.rs,launch.rs,contract.yaml}`, `apps/desktop/src-tauri/src/commands/desktop.rs`, `apps/desktop/src-tauri/src/windows/main_policy.rs`, `apps/desktop/src-tauri/src/app_paths.rs`.
- Modify native: `apps/desktop/src-tauri/src/{lib.rs,commands/mod.rs,windows/mod.rs}`, `apps/desktop/src-tauri/{Cargo.toml,Cargo.lock,build.rs,tauri.conf.json}`, `apps/desktop/src-tauri/permissions/companion.toml`, `apps/desktop/src-tauri/capabilities/main.json`.
- Create web: `web/src/desktop/{runtime.ts,native.ts,bootstrap.tsx,contract.yaml}`, `web/src/api/daemon-origin.ts`, `web/src/app/desktop/connection-settings.tsx`.
- Create web entry: `web/src/app-root.tsx`; modify `web/src/{main.tsx,router.tsx}`, `web/src/api/client.ts`, `web/vite.config.ts`, `web/{package.json,package-lock.json}`.
- Create build/tests: `apps/desktop/scripts/assemble-desktop.mjs`, `apps/desktop/tests/desktop-build.test.mjs`, `apps/desktop/tests/desktop-bootstrap.spec.ts`, `web/src/desktop/bootstrap.test.tsx`.
- Modify: `apps/desktop/{package.json,package-lock.json,vite.config.ts,playwright.config.ts}`.
- Update: native agreements, changed native contracts, `web/src/api/contract.yaml`, both source maps, `apps/desktop/README.md`.

**Interfaces:**
- Native `ReadyConnection { origin: String, configuration_id: String, runtime_id: String }`, obtainable only after process/receipt/readiness checks. Native `LaunchConfig { configuration_id, executable, db, node, adapter, harness, aylul_config, origin }`; paths absolute, origin HTTP literal loopback with an explicit nonzero port and no credentials/path/query/fragment.
- Save at app data `desktop-core.json`. Missing DB/config/executable is an error; the user supplies existing paths. A changed normalized config gets a new configuration_id.
- Main-only IPC: `desktop_connection()`, `desktop_connection_config()`, `save_desktop_connection(config)`, `retry_desktop_connection()`; emit revisioned `desktop-connection`. Put native unit tests under `mod connection_tests` so the named command selects them.
- `ConnectionView = { revision, state: 'unconfigured'|'starting'|'ready'|'failed', configurationId: string|null, origin: string|null, error: string|null }`. Only ready supplies origin. Separate main-only settings read may return configured path references, never a runtime receipt.
- JS `prepareDesktop(): Promise<void>` gates API consumers; `setDaemonOrigin(origin: string): void` freezes the verified origin until a full main reload. Move QueryClient/Router mounting to `mountApplication(root: HTMLElement): void` in app-root.tsx and import it dynamically only after bootstrap readiness, so client.ts cannot capture the old origin during module evaluation. Browser defaults remain unchanged.
- Add `npm --prefix apps/desktop run build:desktop`: build web, build companion only, assemble web assets at `dist/index.html` with companion assets at `dist/companion/`. Keep existing companion URL synchronized.
- Main uses TanStack hash history in native mode; browser uses its existing history. Paths inside the router remain the existing paths.

- [ ] **Step 1:** Add native `connection_tests` for two concurrent starts launching exactly once; malformed/relative/missing paths; stale/reused PID; foreign occupied port; configuration change during startup; a startup exceeding 30 seconds; desktop Quit preserving Core. Validate process image and creation time against Task 3's receipt before readiness is trusted.
- [ ] **Step 2:** Add bootstrap/build tests:

~~~ts
expect(apiRequestsBeforeReady).toEqual([])
expect(browserCallsToNative).toBe(0)
expect(oldConfigurationResultWasApplied).toBe(false)
expect(packagedMainSources).toContain('web/src/main.tsx')
expect(packagedMainSources).not.toContain('apps/desktop/src/app/main.tsx')
expect(unknownAssetResponseIsHtml).toBe(false)
~~~

Also assert one failed launch remains failed until Retry; Arabic/spaced paths reach the executable unchanged; native reload restores a thread route; companion entry/ACL still work.

- [ ] **Step 3:** Run `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml connection_tests`, `npm --prefix web test -- desktop/bootstrap`, and `node --test apps/desktop/tests/desktop-build.test.mjs`; expected RED.
- [ ] **Step 4:** Implement a private startup lock, receipt verification and Task 2's persistent launcher. `AppPaths` defaults to the existing app-data location/identifier. A debug-only `--acceptance-root <absolute existing directory>` selects isolated native preference/auth paths and per-window webview data directories; release code has no such override. Fixture drivers must not install hooks or change personal sidecar state, and must clean only their owned processes after lifetime assertions. Allowlist required OS/home/PATH environment values explicitly; never inherit provider-token variables. Do not add Core/AppCore as a native dependency.
- [ ] **Step 5:** Implement bootstrap/connection screen, build assembly, hash history and main policy from the measured probe. Use Tauri's local-response CSP hook for the exact selected endpoint, keep native IPC sources, block remote navigation/new windows, and open deliberate validated HTTP(S) external links through a main-only `open_external` command. Saving a changed connection detaches old work and fully reloads main at Settings; old query results/drafts cannot enter the new DB context.
- [ ] **Step 6:** Run focused tests above, `npm --prefix web run typecheck`, `npm --prefix web run lint`, `npm --prefix apps/desktop run build:desktop`, and native ACL tests. Start the assembled debug application against an isolated real Core/fake-ACP fixture; create an AYLUL conversation, send once, see streamed text and reload the same thread. Record this as synthetic adapter acceptance.
- [ ] **Step 7:** Commit `feat(desktop): launch Core and load the shared dashboard`.

### Task 5: Transfer retained native screens, then delete the old main UI

**Files:**
- Move/rework: `apps/desktop/src/app/AccountPanel.tsx` to `web/src/app/desktop/account-panel.tsx`; `apps/desktop/src/platform/{auth.ts,aylul.ts}` to `web/src/desktop/{auth.ts,activity-api.ts}`; `apps/desktop/src/data/aylul-state.ts` to `web/src/app/desktop/use-activity.ts`; `apps/desktop/src/features/{sessions/SessionsPanel.tsx,approvals/ApprovalsPanel.tsx,approvals/rule-preview.ts}` to `web/src/app/desktop/{sessions-panel.tsx,approvals-panel.tsx,rule-preview.ts}`.
- Create: `web/src/app/desktop/{activity.tsx,native-settings.tsx,companion-controls.tsx,contract.yaml,native-sections.test.tsx,rule-preview.test.ts}`.
- Modify: `web/src/app/settings/settings-page.tsx`, `web/src/app/sidebar/sidebar.tsx`, `web/src/app/shell.tsx`, `web/src/router.tsx`, `web/src/desktop/native.ts`.
- Remove old main closure: `apps/desktop/index.html`, `apps/desktop/src/app/{main.tsx,WorkspaceShell.tsx,SettingsPanel.tsx,AssistantLocation.tsx,CompanionControls.tsx,icons.tsx,shell.css,workspace.css}`, `apps/desktop/src/features/assistant/`, `apps/desktop/src/platform/assistant.ts`.
- Modify native removal: `apps/desktop/src-tauri/src/commands/mod.rs`, `apps/desktop/src-tauri/{build.rs,permissions/companion.toml}`.
- Update/rehome or remove with their exclusive retired source: `apps/desktop/tests/{approvals-ui.test.mjs,rule-preview.test.ts,assistant-acceptance.mjs,assistant-fixture.tsx,assistant-state.mjs,assistant.html,assistant.vite.config.ts}`, `apps/desktop/tests/companion/{fixture.tsx,shell.spec.ts,workspace.spec.ts,workspace-conversations.spec.ts,assistant-source.spec.ts,production.test.ts}`.
- Update: package manifests/locks if dependencies become unused, existing app/platform/features contracts, settings/sidebar contracts and both source maps.

**Interfaces:** Native Settings renders the existing auth actions and independent companion controls. Native-only route `/aylul/activity` consumes the preserved sidecar wrappers. Main hide action uses existing `hide_main`. No cloud/local chat source selector remains.

- [ ] **Step 1:** Write regression assertions using the moved real wrappers:

~~~ts
expect(localChatNeedsLogin).toBe(false)
expect(previewedRuleSentToApprove).toEqual(capturedPreview.rule)
expect(latePreviewAfterEditIsUsable).toBe(false)
expect(companionFailureDisablesAccount).toBe(false)
expect(accountSignOutCalledChatSourceSwitch).toBe(false)
~~~

Cases: confirmed enroll/sign-out-all/revoke, cancellable sign-in, account error independent of tray/presence, activity session/project context, missing sidecar, settings_changed/preview_mismatch, and browser absence of native-only navigation.

- [ ] **Step 2:** Run `npm --prefix web test -- native-sections rule-preview`; expected RED.
- [ ] **Step 3:** Move functional screens/wrappers using provenance-preserving moves; adapt them to shared web primitives. Keep preview-bound approvals and account confirmation behavior. Link the studio only from a separate development entry; production main must not load it.
- [ ] **Step 4:** Use Serena references/build graph to confirm the old-main removal closure before deleting it. Remove replaced assistant IPC registrations/permissions. Retain native auth internals, companion entry `apps/desktop/src/app/companion.tsx`, companion wrappers, sidecar and externally consumed Hermes packages/services. Rehome useful assertions; remove only tests/fixtures exclusive to the retired chat. Record precise deletions in the commit.
- [ ] **Step 5:** Run web targeted tests/typecheck/lint, `npm --prefix apps/desktop run build:desktop`, `node --test apps/desktop/tests/native-acl.test.mjs`, companion production tests and focused native auth/command tests. Verify generated module inventory has no old main/Hermes chat source. Expected PASS.
- [ ] **Step 6:** Commit `refactor(desktop): keep native functions in the shared main UI`.

### Task 6: Persist a changeable agent binding and closed navigation

**Files:**
- Create: `apps/desktop/src-tauri/src/presence/{mod.rs,binding.rs,preferences.rs,contract.yaml}`, `apps/desktop/src-tauri/src/navigation.rs`.
- Create: `web/src/desktop/{presence.ts,navigation.ts,native-parity.test.ts}`, `web/src/app/desktop/{companion-settings.tsx,companion-binding.test.tsx}`; generate `web/src/test/native-presence.json` from native serialization tests.
- Modify native: `apps/desktop/src-tauri/src/{lib.rs,commands/desktop.rs,commands/companion.rs,windows/mod.rs,windows/companion/mod.rs,core_connection/mod.rs}`, `apps/desktop/src-tauri/{build.rs,permissions/companion.toml}`.
- Modify web: `web/src/app/desktop/native-settings.tsx`, `web/src/main.tsx`, `web/src/desktop/bootstrap.tsx`.
- Update: presence agreement, changed contracts/maps, `apps/desktop/tests/native-acl.test.mjs`.

**Interfaces:**
- Native `BindingView { revision: u64, generation: u64, configuration_id: Option<String>, agent: Option<String>, target: Option<ConversationTarget>, previews: bool }`.
- `ConversationTarget { project_id: String, thread_id: String }`; persist with the chosen agent in `desktop-presence.json`, scoped to configuration_id. Default unbound, previews true.
- Main-only `companion_binding()`, `select_companion_agent(agent: Option<String>)`, `target_companion_thread(thread_id: String, expected_generation: u64)`, `set_companion_previews(enabled: bool)`. Native validation reads the existing `GET /api/aylul/agents` workspace response and `GET /api/threads/{id}`; there is no individual-agent GET route to invent.
- Closed `MainDestination = CompanionSettings | AgentDraft { agent } | Conversation { project_id, thread_id }`.
- `PendingNavigation { revision: u64, destination: MainDestination }`; native `Navigation::request(destination) -> PendingNavigation`, `snapshot() -> Option<PendingNavigation>`, `ack(revision: u64)`; main-only snapshot/ack IPC. Ack must name the current revision. Companion only invokes existing open_main_from_companion.
- Native settings/read failures return fixed error codes; no raw HTTP bodies. serde uses camelCase for IPC. A native `write_presence_fixture` test regenerates `web/src/test/native-presence.json` only with `UPDATE_NATIVE_FIXTURES=1`; JS parity tests consume that artifact, never an independently hand-written shape.

- [ ] **Step 1:** Write `binding_tests` and `navigation_tests`:

~~~rust
assert_eq!(after.agent.as_deref(), Some("agent-b"));
assert_eq!(after.target, None);
assert!(after.generation > before.generation);
assert_eq!(after_old_reply, after);
navigation.ack(old_revision);
assert_eq!(navigation.snapshot().unwrap().destination, new_destination);
~~~

Also cover invalid/deleted agent; wrong-harness or other-agent thread; DB/configuration change; corrupt preferences; selecting same agent as a no-op; unbound click to Settings; bound/no-thread click to draft; target click to conversation without creating it; concurrent latest retarget wins.

- [ ] **Step 2:** Add Settings/parity tests: provider/model labels from generated API, selection survives app restart, changing viewed conversation does nothing, visibility/pause works independently of agent queries, stale response ignored, companion denied selection/target/settings-path access.
- [ ] **Step 3:** Run native `binding_tests`/`navigation_tests` and `npm --prefix web test -- companion-binding`; expected RED.
- [ ] **Step 4:** Implement atomic preferences, native validation through only fixed read endpoints, generation/revision changes and pending navigation delivered after router readiness. Selecting an agent shows the character through the existing visibility owner; later explicit hide wins. Configuration change invalidates binding before a new connection can become active.
- [ ] **Step 5:** Run those tests, native ACL tests, web typecheck/lint and native serialization parity checks; expected PASS. No provider key, prompt or reply may exist in either preference file.
- [ ] **Step 6:** Commit `feat(companion): select an agent and retain its conversation target`.

### Task 7: Native background presence with replay-safe completion

**Files:**
- Create: `apps/desktop/src-tauri/src/presence/{client.rs,watcher.rs,projection.rs,preview.rs,tests.rs}`.
- Modify: `apps/desktop/src-tauri/src/presence/{mod.rs,binding.rs,contract.yaml}`, `apps/desktop/src-tauri/src/windows/companion/{mod.rs,state.rs,contract.yaml}`, `apps/desktop/src-tauri/src/lib.rs`, `apps/desktop/src-tauri/{Cargo.toml,Cargo.lock}`.
- Modify wire consumers: `apps/desktop/src/platform/companion-state.ts`, `web/src/desktop/presence.ts`, `web/src/desktop/native-parity.test.ts`; regenerate `web/src/test/native-presence.json`.
- Update: presence agreement and desktop source map.

**Interfaces:**
- Native `PresenceWatcher::replace(binding: BindingView, connection: ReadyConnection)`, `PresenceWatcher::stop()`; replacement cancels/detaches old reads and advances generation. ReadyConnection comes from Task 4 and includes configuration identity.
- `PresenceProjection` carries expression/status plus optional `Preview { id, text, expires_at_ms }`; publish it in revisioned CompanionState. The presentation never changes renderAllowed.
- Pure `Projection::apply(input: PresenceInput, now_ms: u64) -> PresenceProjection` and `reply_preview(markdown: &str) -> Option<String>`. Define PresenceInput as the decoded observation variants exercised by the state table, with generation/configuration/thread/operation identity on each applicable observation.
- Consume existing `GET /api/threads/{id}`, `GET /api/threads/{id}/operations`, `GET /api/threads/{id}/entries`, `GET /api/aylul/agents`, `GET /api/aylul/providers` (key_set metadata only) and `/api/subscribe?thread_id=...&after=...`; no write endpoint is available to this client.
- Bound reads: 10-second snapshot timeout, 16 MiB per JSON response, 1 MiB per SSE event; reject oversized/malformed input with a fixed unavailable status. Reconnect delay 500 ms doubling to 10 seconds, at most ten failures without caught-up, then explicit Retry.
- reqwest async/stream, no proxy or redirects; use per-request snapshot timeouts, no total timeout on a healthy SSE connection, and eventsource-stream for framing. Business interpretation and preview sanitization remain local product code.

- [ ] **Step 1:** Add reducer fixtures for all spec §8 state-table rows plus:

~~~rust
assert_eq!(replay_completion.preview, None);
assert_eq!(copied_fork_entry.preview, None);
assert_eq!(old_generation_update, current_projection);
assert_eq!(preview.text.chars().count(), 240);
assert_eq!(repeat.expires_at_ms, first.expires_at_ms);
assert_eq!(after_eight_seconds.preview, None);
~~~

Tests: `terminal_before_final_entry_waits`, `terminal_is_monotonic`, `new_turn_replaces_success_timer`, `fork_provenance_is_not_a_new_reply`, `unicode_preview_is_bounded`, `hidden_expiry_does_not_revive`, `deleted_agent_clears_preview`, `provider_without_key_is_blocked`, `unknown_status_is_not_success`.

- [ ] **Step 2:** Add an HTTP/SSE fixture test for subscribe-before-snapshot reconciliation, caught-up/replay suppression, duplicate seq, lagged/fatal, disconnect during a turn, rapid A-to-B retarget, active binding deletion, oversized history/frame, and native watcher continuing while main is hidden/reloaded. Assert fixture requests are reads only and there is one active target subscription.
- [ ] **Step 3:** Run `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml presence::`; expected RED.
- [ ] **Step 4:** Implement the watcher and pure mapping from actual operation status, durable ToolCall/Subagent progress and delta presence. Revalidate agent/target on reconnect and a bounded 10-second read-only liveness check while bound; deletion must be discovered even without a deletion event on that thread. The liveness check validates only the selected agent/provider from these existing listing APIs; no history scan, watch per agent or database access is added.
- [ ] **Step 5:** Resolve preview only after a newly live Completed operation and a reconciled durable `AgentMessage` with matching thread_id/operation_id; choose greatest ordinal. Ignore inherited fork provenance, tool cards, user messages and old initial completions. Wait for a late final entry through bounded snapshot recovery; missing content is no preview.
- [ ] **Step 6:** Implement the small preview sanitizer: omit fenced code and images/HTML tags, keep link labels/inline code text, remove Markdown decoration, collapse whitespace, truncate by Unicode scalar count. Add hand-authored Arabic/emoji/link/code/HTML fixtures; malformed unsupported structure returns no preview rather than raw markup. Preview text is in memory only and is never logged.
- [ ] **Step 7:** Run presence tests plus existing native companion state tests and IPC parity tests; expected PASS. Dependency commit classifies eventsource-stream as SSE infrastructure; no general-purpose product-rule dependency is added.
- [ ] **Step 8:** Commit `feat(companion): observe its bound conversation in the background`.

### Task 8: Connect normal sends and render the character's live reactions

**Files:**
- Modify: `web/src/app/conversation/{draft.tsx,composer.tsx,conversation.tsx,contract.yaml}`, `web/src/desktop/presence.ts`, `web/src/app/desktop/companion-settings.tsx`.
- Create: `web/src/desktop/companion-send.ts`, `web/src/app/conversation/companion-send.test.tsx`, `apps/desktop/src/features/companion/native-presentation.ts`, `apps/desktop/tests/companion/live-presence.spec.ts`.
- Modify: `apps/desktop/src/features/companion/{DesktopCompanion.tsx,SpeechBubble.tsx,desktop.css,contract.yaml}`, `apps/desktop/src/platform/{companion.ts,companion-state.ts,contract.yaml}`, `apps/desktop/tests/companion/{native-state.test.ts,desktop-gesture.test.ts,desktop-window.spec.ts}`.
- Update: web desktop contract and both source maps.

**Interfaces:**
- `noteCompanionSend(threadId: string): Promise<void>` attempts a native retarget for the selected agent, checks the captured generation, reports binding failure independently and never rejects the ordinary send.
- Invoke after successful draft thread creation and before its turn attempt; invoke for explicit Send/Queue in an existing selected-agent conversation. Queue acceptance alone never sets thinking; only Core admission/status does. Automatic queued-turn execution needs no new retarget.
- `Use with companion` explicitly retargets an existing selected-agent thread. Other agents cannot silently replace the selected agent.
- DesktopCompanion consumes CompanionState's projection and existing native render permission; remove the constant Idle presentation.

- [ ] **Step 1:** Add send-path tests:

~~~ts
expect(threadCreateBodies[0].profile).toBe('agent-a')
expect(startTurnCalls).toHaveLength(1)
expect(queueAcceptanceMadeCharacterRunning).toBe(false)
expect(bindingFailurePreventedSend).toBe(false)
expect(navigationOnlyChangedBinding).toBe(false)
~~~

Pin the first-send model selection/replay path, existing-thread send, queue, admission failure, unselected agent and a successfully created thread whose subsequent session/turn fails.

- [ ] **Step 2:** Add browser character fixtures: success lasts two seconds, preview expires after eight seconds, repeated ID cannot extend it, hiding through native state stops draws, showing cannot revive an expired preview, paused/reduced motion retains readable status, two-line preview fits compact bounds, Enter/Space opens main, drag does not, and a broken Canvas leaves account/tray controls usable.
- [ ] **Step 3:** Run `npm --prefix web test -- companion-send first-send` and, from apps/desktop, `npx playwright test tests/companion/live-presence.spec.ts`; expected RED.
- [ ] **Step 4:** Connect sends and independent binding-error UI, adapt projection/timers by the existing message-slot/scheduler owners, render plain text only and preserve no-activation/drag behavior. No duplicated transcript or new composer is added to the character.
- [ ] **Step 5:** Run the focused tests, web typecheck/lint, `npm --prefix apps/desktop run build:desktop`, companion unit tests and the desktop-window/native-visibility browser regressions. Expected PASS.
- [ ] **Step 6:** Commit `feat(companion): react to AYLUL replies and open the bound chat`.

### Task 9: One full gate and an actual Windows journey

**Files:**
- Create: `apps/desktop/tests/desktop-web-companion.ps1`, `docs/evidence/aylul/2026-10-20-desktop-web-companion.md`.
- Modify: `apps/desktop/tests/{windows-shell.ps1,windows-companion.ps1,companion-native.mjs,native-acl.test.mjs}` only for the new main entry/navigation and independent presence assertions.
- Update: `apps/desktop/README.md`, `docs/aylul/{README.md,codebase/README.md,ui/stages.md,native/stages.md,character/stages.md}`, implementation status in the spec and this plan's checkboxes.

- [ ] **Step 1:** Build the debug desktop in place and use an isolated DB/profile copy plus a saved native launch config. The driver must assert the eight real Windows cases in spec §9; target owned windows/processes only. Keep account/device actions synthetic unless their real action is specifically authorized.
- [ ] **Step 2:** Prove background completion with main actually hidden to tray, character show not stealing focus, clicking it restoring the exact conversation, binding switch during an old turn, hide/pause holding during a new reply, native ACL denial, and desktop Quit/relaunch attaching to the same daemon without a repeated send. Record actual request Origin/CSP/IPC outcomes; browser fixtures are insufficient.
- [ ] **Step 3:** Perform one real AYLUL provider turn using the person's authorized runtime credential setup; never retrieve a key into tool output or change personal config implicitly. If credentials/human interaction are unavailable, record that acceptance as pending; do not label provider/native journey complete.
- [ ] **Step 4:** Run the root full gate once:
  - `cargo fmt --all --check`
  - `cargo clippy --workspace --all-targets --features fake-acp/test-support -- -D warnings`
  - `cargo test --workspace`
  - `cargo clippy --workspace -- -D warnings`
  - `cargo tree -e features,no-dev --workspace`: filtering for test-support must yield no line.
  - Regenerate OpenAPI canonically and require `git diff --exit-code -- api/`; regenerate TS and require no schema drift.
  - Run the CI width check in Bash, expected exit 0 with no violations:

~~~bash
find crates -name '*.rs' -not -path '*/target/*' -print0 \
  | LC_ALL=C xargs -0 awk '
      { s = $0; sub(/\r$/, "", s); gsub(/[\200-\277]/, "", s) }
      length(s) > 100 {
        printf "::error file=%s,line=%d::%d characters; the limit is 100\n",
          FILENAME, FNR, length(s)
        bad = 1
      }
      END { exit bad }'
~~~

- [ ] **Step 5:** Run full relevant client/native gates: web gen:api/typecheck/lint/test/build; `npm --prefix apps/desktop run build:desktop`, retained desktop Node tests, companion unit/browser/production tests; native `cargo fmt --manifest-path apps/desktop/src-tauri/Cargo.toml -- --check`, `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml`. Sidecar/hook source is unchanged; report that separately rather than inventing a new migration gate.
- [ ] **Step 6:** Record commands, counts, versions, actual/synthetic/native/provider distinctions, failures and outstanding hardware gaps. Name any source files over 300 lines with their one responsibility. Re-run a failed/newly affected check only after a relevant fix; no second routine whole-branch run; after completion dispatch the final same-model reviewer requested by Mohammed.
- [ ] **Step 7:** Commit `test(desktop): verify shared dashboard and bound companion journey`. Verify only the preserved `.mcp.json` change remains, report task commits and any pending acceptance, and stop without pushing.

## Execution notes and technical references

Execution was approved on 2026-10-20. Its first task intentionally supplies a measured origin rather than a guessed constant. If that changes the selected architecture, return to the spec; otherwise continue inline in task order.

- [Tauri 2.11.5 local-resource response policy](https://docs.rs/tauri/2.11.5/tauri/webview/struct.WebviewWindowBuilder.html#method.on_web_resource_request): supports modifying local application responses; the probe must establish Windows behavior.
- [TanStack history types](https://tanstack.com/router/latest/docs/framework/react/guide/history-types): use hash history for the packaged main without changing browser routes.
- [eventsource-stream 0.2.3](https://docs.rs/eventsource-stream/0.2.3/eventsource_stream/): framing over reqwest byte streams; our reducer remains the application owner.

No product code or acceptance test has been run by writing this plan.
