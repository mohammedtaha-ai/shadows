# Inspected source map

Updated: 2026-10-19 (normal conversation). Paths below are relative to the
Shadows repository root. Desktop, hook and sidecar retain independent builds;
no AYLUL module was added to AppCore. Main and independent companion Windows surfaces with a native
backend login/conversation consumer, Claude Code hook and local sidecar. Mobile
and semantic project-context integration remain future paths.

| Source | Owned job | Contract |
| --- | --- | --- |
| `apps/desktop/src/features/assistant/` | Account conversation hook, sidebar, view, bot list and read-only details with visible-panel snapshot polling | Colocated `contract.yaml` |
| `apps/desktop/src-tauri/src/assistant/` | Closed native assistant operation mapping | Colocated `contract.yaml` |
| `apps/desktop/src/api/generated/` | Backend-generated wire artifacts and input hash | Backend TypeBox owner; `provenance.json` |
| `apps/desktop/src/app/` | Main React presentation: workspace navigation shell, its inline icons and settings grouping, account panel and local hide error | Colocated `contract.yaml` |
| `apps/desktop/src/platform/` | Typed native invocation wrappers | Colocated `contract.yaml` |
| `apps/desktop/src/features/` | Local sessions and requests presentation with person actions | Colocated `contract.yaml`; excludes separately owned assistant, companion and studio |
| `apps/desktop/src/data/aylul-state.ts` | Refresh local sidecar state while the panel is visible | Features contract; presentation polling |
| `apps/desktop/src/features/companion/` | Compact desktop character/gestures plus reusable presence, procedural Canvas renderer and explicit drawing permission | Colocated `contract.yaml`; excludes separately owned studio |
| `apps/desktop/src/features/companion/studio/` | Development-only scenes, controlled simulations and PNG export | Colocated `contract.yaml`; excluded from parent scope |
| `apps/desktop/src-tauri/src/lib.rs` | Compose native companion and sidecar lifetime | Composition only; behavior owned below |
| `apps/desktop/src-tauri/src/windows/` | Main/tray lifecycle and physical placement | Colocated `contract.yaml` |
| `apps/desktop/src-tauri/src/windows/companion/` | Independent native companion lifecycle, explicit render permission, no-activation show and complete-rectangle fitting | Colocated `contract.yaml`; excluded from parent scope |
| `apps/desktop/src-tauri/src/commands/` | Main-only native action IPC boundary | Colocated `contract.yaml` |
| `apps/desktop/src-tauri/src/native_login/` | Local return/verifier/browser/DPAPI primitives | Colocated `contract.yaml` |
| `apps/desktop/src-tauri/src/auth/` | Protected account backend client | Colocated `contract.yaml` |
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
