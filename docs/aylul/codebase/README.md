# Inspected source map

Updated: 2026-10-20 (shared dashboard and bound companion). Paths below are relative to the
Shadows repository root. Desktop, hook and sidecar retain independent builds;
no AYLUL module was added to AppCore. The main Windows surface hosts `web/`;
native owns verified Core startup and the independent companion watcher.
Account login, the Claude Code hook and local sidecar remain native functions.
Mobile and semantic project-context integration remain future paths.

| Source | Owned job | Contract |
| --- | --- | --- |
| `apps/desktop/src-tauri/src/presence/` | Configuration-scoped companion presence | Colocated `contract.yaml` |
| `apps/desktop/src-tauri/src/navigation.rs` | Closed pending main navigation | `docs/aylul/shared/requests/2026-10-20-companion-binding-presence.md` |
| `apps/desktop/src-tauri/src/app_paths.rs` | Native storage location | Existing identifier; isolated debug acceptance override |
| `apps/desktop/src-tauri/src/core_connection/` | Verified local Core lifecycle | Colocated `contract.yaml` |
| `apps/desktop/src-tauri/src/windows/main_policy.rs` | Shared main response policy | Windows contract |
| `web/src/desktop/` | Shared dashboard native boundary | Colocated `contract.yaml` |
| `web/src/app/desktop/` | Native account/activity/character settings | Colocated `contract.yaml` |
| `apps/desktop/src/studio.tsx` | Separate development character studio | Character studio contract; absent from production inputs |
| `apps/desktop/scripts/` | Shared desktop asset assembly | `apps/desktop/tests/desktop-build.test.mjs` |
| `packages/hermes-client/` | Portable native Hermes TypeScript HTTP transport consumed as a packed artifact | Colocated `contract.yaml` |
| `services/assistant/` | Persistent installation-scoped local assistant application | Colocated `contract.yaml` |
| `apps/desktop/src-tauri/src/assistant/` | Retained native assistant service primitives without main IPC registration | Colocated `contract.yaml` |
| `apps/desktop/src/api/generated/` | Backend-generated wire artifacts and input hash | Backend TypeBox owner; `provenance.json` |
| `apps/desktop/src/app/` | Independent companion entry | Colocated `contract.yaml` |
| `apps/desktop/src/platform/` | Typed companion invocation wrappers | Colocated `contract.yaml` |
| `apps/desktop/src/features/companion/native-presentation.ts` | Absolute-expiry native reply presentation | Companion contract |
| `apps/desktop/src/features/companion/` | Compact desktop character/gestures plus reusable presence, procedural Canvas renderer and explicit drawing permission | Colocated `contract.yaml`; excludes separately owned studio |
| `apps/desktop/src/features/companion/studio/` | Development-only scenes, controlled simulations and PNG export | Colocated `contract.yaml`; excluded from parent scope |
| `apps/desktop/src-tauri/src/lib.rs` | Compose native companion and sidecar lifetime | Composition only; behavior owned below |
| `apps/desktop/src-tauri/src/windows/` | Main/tray lifecycle and physical placement | Colocated `contract.yaml` |
| `apps/desktop/src-tauri/src/windows/companion/` | Independent native companion lifecycle, explicit render permission, no-activation show and complete-rectangle fitting | Colocated `contract.yaml`; excluded from parent scope |
| `apps/desktop/src-tauri/src/commands/` | Main-only native action IPC boundary | Colocated `contract.yaml` |
| `apps/desktop/src-tauri/src/native_login/` | Local return/verifier/browser/DPAPI primitives | Colocated `contract.yaml` |
| `apps/desktop/src-tauri/src/auth/` | Protected account backend client with captured-session assistant verification | Colocated `contract.yaml` |
| `apps/desktop/src-tauri/src/sidecar/` | Node sidecar process lifetime and stdio calls | Colocated `contract.yaml` |
| `apps/desktop/src-tauri/examples/auth_state.rs` | Fresh-process acceptance verifier | Auth contract; no credential output |
| `apps/desktop/src-tauri/examples/native_fixture.rs` | Explicit synthetic browser/protected-store process fixture | Test utility, no wire contract |
| `apps/desktop/tests/` | Physical Windows shell and synthetic native acceptance drivers | Results in dated evidence |
| `apps/desktop/hook/` | One Claude Code hook call to the sidecar pipe | Colocated `contract.yaml` |
| `apps/desktop/sidecar/src/hooks/` | Token guarded hook endpoint and pipe server | Colocated `contract.yaml` |
| `apps/desktop/sidecar/src/sessions/` | Session state and trimmed steps from hook events | Colocated `contract.yaml` |
| `apps/desktop/sidecar/src/agents/` | Narrow permission rules and safe settings edits | Colocated `contract.yaml` |
| `apps/desktop/sidecar/src/approvals/` | Held requests and refusals with person supplied outcomes | Colocated `contract.yaml` |
| `apps/desktop/sidecar/src/store/` | Local SQLite sessions, steps, decisions and rule records | Colocated `contract.yaml` |
| `apps/desktop/sidecar/src/` | Sidecar composition and stdio RPC for the desktop window | Colocated `contract.yaml` |

No backend source is imported by the client.
