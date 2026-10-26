# AYLUL desktop architecture

Owner of the desktop's structure: the native top object, its owners, the allowed dependency
directions, the sidecar shape and the companion TypeScript entries. The checks that hold it are
the native ratchet (`apps/desktop/src-tauri/tests/architecture/`, its README holds the owner
table) and the companion boundary test (`apps/desktop/tests/companion/boundaries.test.ts`).

Decided 2026-10-25 (D-0). It was built by tasks N-8 to N-21, U-4 and C-7; the task plan and
its inventory are in `docs/aylul/native/plans/2026-10-25-desktop-architecture.md` at
`ca618f89`, and the gate run in `docs/evidence/2026-10-26-desktop-architecture-gate.md`.

## Decision

- **One crate, module boundaries first.** The Tauri crate stays one crate. No module becomes a
  crate until a second consumer or a compiler-held boundary needs it, because most native code
  is tied to one Tauri application and UI thread, and extracting early would harden today's
  DTOs.
- **One top object, `DesktopApp`** (`src-tauri/src/app.rs`). It is built once, is the only value
  in Tauri managed state, owns assembly and shutdown, and exposes typed owner accessors over
  private fields. Each owner receives only its declared dependencies.
- **Commands translate only.** A command may take `tauri::State<Arc<DesktopApp>>`. Multi-owner
  workflows are methods on the owner (`Companion` for companion workflows). `AppHandle` is for
  window and event work, never a service lookup. Tray callbacks are wired with explicit
  actions, not found through a registry.
- **Window code holds no conversation state.** `CompanionWindow` owns its surface DTO and
  receives what it shows; it does not import the conversation producer.
- **One private Core client inside `Companion`** for binding reads, observation and submission.
  `CoreConnection` keeps its own readiness handshake. The shared web client is owned
  separately.
- **Behaviour and stored identity do not change with structure:** captured retries, newer
  binding precedence, watcher replacement, publication acknowledgement, foreground restoration,
  `app.nylra.desktop`, the DPAPI location and sidecar storage.

## Owners

| Owner | One job |
|---|---|
| `CoreConnection` | Establish a verified local runtime connection |
| `Companion` | Operate the bound conversation companion (owns its watcher) |
| `CompanionWindow` | Control the native companion surface |
| `MainWindow` | Control the native main surface |
| `Navigation` | Deliver the latest closed main destination |
| `DesktopPreferences` | Coordinate machine-specific desktop preferences |
| `Account` | Coordinate protected account access |
| `Activity` | Expose the owned hook sidecar |
| `AppPaths` | Supply immutable native locations |

## Allowed dependency directions

```text
Tauri commands / lifecycle adapters → DesktopApp
DesktopApp → Companion, CoreConnection, CompanionWindow, MainWindow, Navigation,
             DesktopPreferences, Account, Activity
Companion → CoreConnection, CompanionWindow, Navigation, its private Core client (HTTP / SSE)
Navigation → MainWindow → DesktopPreferences
CoreConnection → process, receipts, readiness
Account → private native-login primitives
Activity → Node sidecar → hook protocol
```

## Sidecar

One composition (`createActivityApp`) builds `Sessions`, `Approvals` (pending lifecycle plus
preview and approval admission), `AgentSettings` (hook installation and permission-rule edits)
and `HookEndpoint` (authenticated pipe lifetime). One SQLite connection may stay; each domain
store receives it during composition, and adapters receive service operations, never the shared
store.

## Companion TypeScript entries

| Module | Entry |
|---|---|
| Native companion start-up | `src/app/companion.tsx` |
| Native companion IPC | `src/platform/companion.ts` |
| Native companion feature | `features/companion/DesktopCompanion.tsx` |
| Character drawing | `features/companion/renderer/index.ts` |
| Motion model | `features/companion/animation/index.ts` |
| Development studio | `features/companion/studio/CompanionStudio.tsx` |

Only platform code imports Tauri APIs. Renderer and animation import no platform or studio
code. Production companion code imports no studio or test fixture. The renderer makes no HTTP
call. Studio uses the renderer and animation entries. Main screens stay in `web/`.

## Checks

Each boundary above is held by a parser-backed check with an exact baseline that only shrinks:
new violations and stale entries both fail, and adversarial fixtures prove each rule catches
its bypass. These checks do not replace behaviour tests: Windows focus, tray behaviour, settings
races and provider acceptance stay with their own tests and acceptance.

## Open

> **OPEN: distribution ownership.** `bundle.active` is false, sidecar start-up uses development
> paths, and Node and hook installation are unresolved. Shipping managed binaries versus keeping
> explicit existing paths affects lifecycle, updates and trust. Closes when Mohammed decides how
> the desktop is distributed.

> **OPEN: activity retention.** Sidecar records and backups have no retention lifecycle; inline
> shell commands can hold sensitive text, and pending refusals disappear on restart. Closes when
> retention, restoration and deletion behaviour are decided.

> **OPEN: native acceptance.** Real DPI transitions, monitor removal, suspend and resume,
> optimised resource measurements and launch at OS sign-in are not provable by tests. C-6 in
> `docs/aylul/character/stages.md` holds the companion part. Closes when each is accepted on a
> real machine.

> **OPEN: stored installation identity.** Consolidating `app.nylra.desktop`, the DPAPI location,
> Core DB references, sidecar storage and profile homes, or renaming the installation, is a
> migration decision. Closes when Mohammed asks for that migration.

> **OPEN: retained assistant APIs.** Account-assistant primitives and generated declarations stay
> in source without the retired main chat IPC. Closes when their owner decides to keep or remove
> them.
