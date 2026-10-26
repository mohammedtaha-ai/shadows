# Inspected source map

Updated: 2026-10-20 (shared dashboard and bound companion). Paths below are relative to the
Shadows repository root. Desktop, hook and sidecar retain independent builds;
no AYLUL module was added to AppCore. The main Windows surface hosts `web/`;
native owns verified Core startup and the independent companion watcher.
Account login, the Claude Code hook and local sidecar remain native functions.
Mobile and semantic project-context integration remain future paths.

| Source | Owned job | Contract |
| --- | --- | --- |
| `.github/workflows/aylul.yml` | Desktop automated gate | Desktop job runs test typechecking (`apps/desktop/tsconfig.test.json`), top-level Node tests, companion unit/source-boundary and production-boundary suites plus native/hook/sidecar checks; separate Windows companion browser job installs Chrome; `RECORD_PURPLE`, `EXPORT_COLORS` and native acceptance scripts stay opt-in; shared-main native parity runs in the root web CI job |
| `apps/desktop/src-tauri/tests/architecture/` | Shrinking native ownership ratchet | `docs/aylul/native/plans/2026-10-25-desktop-architecture.md` D-0; exact baseline, accessor-owner fixtures and guard/grant/wrapper surface parity; scanner behavior in its README |
| `apps/desktop/src-tauri/src/presence/appearance_tests.rs` | Opaque companion appearance regressions | Presence contract |
| `apps/desktop/src-tauri/src/presence/core_client/` | The companion's only Core HTTP client: blocking binding reads and submission writes, async observer reads and SSE | Presence contract; the architecture ratchet refuses `reqwest` elsewhere in the companion |
| `apps/desktop/src-tauri/src/presence/send.rs` | Captured companion submission admission | Presence contract; a newer selection owns binding publication |
| `apps/desktop/src-tauri/src/presence/send_fixture.rs` | Scripted submission HTTP server | Presence contract; test-only |
| `apps/desktop/src-tauri/src/presence/send_tests.rs` | Submission admission regressions | Presence contract |
| `apps/desktop/src-tauri/src/presence/` | Operate the configuration-scoped companion conversation with its owned observer | Colocated `contract.yaml` |
| `apps/desktop/src-tauri/src/presence/workflows.rs` | Coordinate companion actions through the owned observer, injected native surface operations and Navigation | Presence contract; selection/show, retarget/readiness, publication admission and open-main workflows |
| `apps/desktop/src-tauri/src/navigation.rs` | Deliver closed pending main navigation before opening main | `docs/aylul/shared/requests/2026-10-20-companion-binding-presence.md` |
| `apps/desktop/src-tauri/src/app_paths.rs` | Native storage location | Existing identifier; isolated debug acceptance override |
| `apps/desktop/src-tauri/src/core_connection/` | Verified local Core lifecycle | Colocated `contract.yaml` |
| `apps/desktop/src-tauri/src/desktop_preferences/` | Native desktop preference coordination | Colocated `contract.yaml`; private store and injected OS login-item adapter |
| `apps/desktop/src-tauri/src/desktop_preferences/tests.rs` | Desktop preference failure regressions | Desktop preferences contract |
| `apps/desktop/src-tauri/src/commands/preferences.rs` | Main-only desktop preference IPC | Commands contract |
| `apps/desktop/tests/desktop-preferences.mjs` | Native preference lifecycle acceptance | Desktop preferences contract |
| `apps/desktop/src-tauri/src/windows/main_policy.rs` | Main response policy with composition-supplied origin and webview options | Windows contract |
| `web/src/desktop/` | Shared dashboard native boundary | Colocated `contract.yaml` |
| `web/src/app/desktop/` | Native account/activity/character settings | Colocated `contract.yaml` |
| `apps/desktop/src/studio.tsx` | Separate development character studio | Character studio contract; absent from production inputs |
| `apps/desktop/scripts/` | Shared desktop asset assembly | `apps/desktop/tests/desktop-build.test.mjs` |
| `packages/hermes-client/` | Portable native Hermes TypeScript HTTP transport consumed as a packed artifact | Colocated `contract.yaml` |
| `services/assistant/` | Persistent installation-scoped local assistant application | Colocated `contract.yaml` |
| `apps/desktop/src-tauri/src/assistant/` | Retained native assistant service primitives without main IPC registration | Colocated `contract.yaml` |
| `apps/desktop/src/api/generated/` | Backend-generated wire artifacts and input hash | Backend TypeBox owner; `provenance.json` |
| `apps/desktop/src/app/` | Independent companion entry | Colocated `contract.yaml` |
| `apps/desktop/src/platform/` | Typed companion-only invocation wrappers | Colocated `contract.yaml` |
| `apps/desktop/src/platform/companion-state.test.ts` | Companion projection validation regressions | Platform contract; co-located unit test |
| `apps/desktop/src/features/companion/renderer/index.ts` | Public character and scene drawing entry | Companion contract; explicit current-consumer exports |
| `apps/desktop/src/features/companion/animation/index.ts` | Public motion, scheduling, viewport and message entry | Companion contract; explicit current-consumer exports |
| `apps/desktop/src/features/companion/animation/geometry.test.ts` | Co-located viewport geometry regressions | Companion contract |
| `apps/desktop/src/features/companion/animation/messages.test.ts` | Co-located message lifetime regressions | Companion contract |
| `apps/desktop/src/features/companion/animation/model.test.ts` | Co-located motion model regressions | Companion contract |
| `apps/desktop/src/features/companion/animation/scheduler.test.ts` | Co-located frame scheduling regressions | Companion contract |
| `apps/desktop/src/features/companion/animation/fixtures/test-fakes.ts` | Deterministic animation test support | Companion contract; excluded from production reachability, bundle and `tsc` |
| `apps/desktop/src/features/companion/composer.test.ts` | Co-located draft and retry regressions | Companion contract |
| `apps/desktop/src/features/companion/desktop-gesture.test.ts` | Co-located desktop gesture regressions | Companion contract |
| `apps/desktop/tests/companion/fakes.ts` | Native composer browser test support | Companion contract; used by `composer.spec.ts` |
| `apps/desktop/src/features/companion/renderer/palette.test.ts` | Character colour palette regressions | Companion contract; co-located unit test |
| `apps/desktop/tests/companion/boundaries.test.ts` | Parser-backed companion import ratchet | Desktop architecture section 6; exact per-import baseline and adversarial fixtures in `fixtures/boundaries/`; production reachability starts at `src/app/companion.tsx` |
| `apps/desktop/src/features/companion/CompanionComposer.tsx` | Compact scoped text submission | Companion contract; unavailable Retry retains pending identity |
| `apps/desktop/src/features/companion/composer-model.ts` | Draft and retry identity rules | Companion contract |
| `apps/desktop/src/features/companion/native-presentation.ts` | Absolute-expiry native reply presentation | Companion contract |
| `apps/desktop/src/features/companion/` | Compact desktop character/gestures plus reusable presence, procedural Canvas renderer and explicit drawing permission | Colocated `contract.yaml`; excludes separately owned studio |
| `apps/desktop/src/features/companion/studio/` | Development-only scenes, controlled simulations and PNG export | Colocated `contract.yaml`; excluded from parent scope |
| `apps/desktop/src-tauri/src/lib.rs` | Register native lifetime adapters | Composition contract |
| `apps/desktop/src-tauri/src/app.rs` | Assemble the nine native owners with explicit window handles and live close/tray callbacks | `src-tauri/src/contract.yaml` |
| `apps/desktop/src-tauri/src/windows/` | MainWindow construction/opening with injected tray/close callbacks and physical placement | Colocated `contract.yaml` |
| `apps/desktop/src-tauri/src/windows/companion/input.rs` | Explicit editor focus with physical bounds through the supplied window Handle | Companion window contract |
| `apps/desktop/src-tauri/src/windows/companion/surface.rs` | Window-owned presentation DTOs | Companion window contract; producers map outside window code |
| `apps/desktop/src-tauri/src/windows/companion/state.rs` | Revisioned companion surface state | Companion window contract; exact unbound, bound, previewing and failed payloads cover command answers and events |
| `apps/desktop/src-tauri/src/windows/companion/` | Independent native companion lifecycle through a shared opaque Handle, explicit render permission, no-activation show and complete-rectangle fitting | Colocated `contract.yaml`; excluded from parent scope |
| `apps/desktop/src-tauri/src/commands/` | Main-only native action IPC boundary | Colocated `contract.yaml` |
| `apps/desktop/src-tauri/src/native_login/` | Local return/verifier/browser/DPAPI primitives | Colocated `contract.yaml` |
| `apps/desktop/src-tauri/src/auth/` | Protected account backend client with captured-session assistant verification | Colocated `contract.yaml` |
| `apps/desktop/src-tauri/src/sidecar/` | Node sidecar process lifetime and stdio calls | Colocated `contract.yaml` |
| `apps/desktop/src-tauri/examples/auth_state.rs` | Fresh-process acceptance verifier | Auth contract; no credential output |
| `apps/desktop/src-tauri/examples/native_fixture.rs` | Explicit synthetic browser/protected-store process fixture | Test utility, no wire contract |
| `apps/desktop/tests/` | Physical Windows shell and synthetic native acceptance drivers | Results in dated evidence |
| `apps/desktop/hook/` | One Claude Code hook call to the sidecar pipe | Colocated `contract.yaml` |
| `apps/desktop/sidecar/src/hooks/` | Token guarded hook endpoint and pipe server | Colocated `contract.yaml` |
| `apps/desktop/sidecar/src/sessions/` | Persisted session projections and capped steps (Sessions; private sessions/store.ts) | Colocated `contract.yaml` |
| `apps/desktop/sidecar/src/agents/` | Hook installation and permission edits (AgentSettings; private agents/store.ts owns rules and grant_uses) | Colocated `contract.yaml` |
| `apps/desktop/sidecar/src/approvals/` | Approval lifecycle and preview admission (Approvals; private approvals/store.ts owns decisions) | Colocated `contract.yaml` |
| `apps/desktop/sidecar/src/store/` | One SQLite connection with the fixed aylul.db schema; no domain persistence API | Colocated `contract.yaml` |
| `apps/desktop/sidecar/src/architecture.test.ts` | Parser-backed sidecar ownership and RPC inventory ratchet | Sidecar composition contract; exact baseline and adversarial JSON fixtures; TypeScript 7 AST over text inputs |
| `apps/desktop/sidecar/src/persistence.test.ts` | Fixed-schema row and RPC projection pins | Sidecar composition contract |
| `apps/desktop/sidecar/src/` | Sidecar composition (createActivityApp in app.ts), its process entry (main.ts) and service-only stdio adapters | Colocated `contract.yaml` |

No backend source is imported by the client.
