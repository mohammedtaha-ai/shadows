# AYLUL Windows desktop

The main window packages the existing Shadows dashboard from `web/`. The
independent transparent companion uses its own entry and native lifecycle.
The application identifier and existing native account storage remain unchanged.

Run these commands from `E:/Globalprojects/shadows`:

```powershell
npm --prefix apps/desktop run build:desktop
cargo build --locked --manifest-path apps/desktop/src-tauri/Cargo.toml
& apps/desktop/src-tauri/target/debug/nylra-desktop.exe
```

Build assets before Rust: the debug executable embeds the assembled `dist/`.
The build requires MSVC and WebView2. `build:desktop` builds the shared dashboard,
builds the companion, then assembles `dist/index.html` and
`dist/companion/companion.html`. `npm --prefix apps/desktop run build` is the same
shared assembly. Use `build:companion` for the character alone.

## Existing local runtime

On first launch, Local Core asks for existing absolute paths to the Shadows
executable, database, Node, ACP adapter, Claude Code executable and AYLUL
configuration, plus a literal local HTTP address. Save starts Core automatically.
It verifies the process instance, canonical database and runtime/configuration
identities before attaching. Retry never silently replaces a live process.
Changing the configuration reloads main at Settings. Quitting desktop leaves
Core running; the next launch verifies and reuses that runtime.

Use the actual existing dashboard database when continuing an installation.
Neither the legacy Hermes `assistant.sqlite` nor the hook sidecar `aylul.db` is
that Core database. Connection preferences contain file references, not provider
keys. Configure providers and agents through the existing dashboard Settings.

Native account and local activity controls remain in shared Settings and Local
activity. Account login is independent of local AYLUL chat. The old desktop
Hermes chat/source selector and its main IPC registration have been removed.
`packages/hermes-client` and `services/assistant` were removed on 2026-10-29.
Historical account chat evidence (`docs/evidence/aylul/2026-10-19-normal-chat.md` at `0187365`)
records that earlier UI; it does not describe the current composer.

For retained account/hook functions, the existing launcher may set these public
runtime references before starting the same executable:

```powershell
$env:NYLRA_BACKEND_URL='https://127.0.0.1:8443'
$env:NYLRA_DEV_CA=(Resolve-Path '../shadows-backend/.local-runtime/tls/ca.pem').Path
$env:AYLUL_HOOK_EXE=(Resolve-Path 'apps/desktop/hook/target/debug/aylul-hook.exe').Path
```

Start the sibling backend using its
[handoff](../../../shadows-backend/docs/nylra-local-auth-handoff.md) when those
account actions need it. The public development CA configures the native client;
do not bypass certificate verification. Account credentials remain in native
DPAPI storage. No credential migration or personal hook installation is implied.

## Bound companion

Choose an AYLUL agent in Settings. You can change it later; viewing another chat
does not change the selection. Send from that agent's chat, or choose Use with
companion, to establish its target. The native watcher observes that one chat
while main is hidden or reloaded. It does not send another turn.

The character shows actual progress and a short plain-text completion preview.
Previews can be disabled in Settings. Click the character or press Enter/Space
to open its exact chat. Dragging consumes the click. Hiding or pausing stays in
effect across replies; no microphone or voice feature is active.

Closing a window hides it. The tray, left tray click and Ctrl+Alt+Shift+F12 retain
main restoration; Quit NYLRA exits desktop. Companion failure remains separate
from the main window, account, sidecar and Core runtime.

## Checks and acceptance

```powershell
npm --prefix web run typecheck
npm --prefix web run lint
npm --prefix web run test -- --run
npm --prefix apps/desktop test
npm --prefix apps/desktop run test:companion:unit
npm --prefix apps/desktop run test:companion:browser
npm --prefix apps/desktop run test:companion:production
cargo test --locked --manifest-path apps/desktop/src-tauri/Cargo.toml
```

Browser checks use installed Chrome with synthetic IPC; they do not establish
native focus, tray behavior or a real provider response. The development server
exposes `studio.html` separately; production excludes the studio.

The explicit automated native regression
`AYLUL_RUN_NATIVE=1 node --test apps/desktop/tests/desktop-bootstrap.spec.ts`
uses a separate temporary profile and fake ACP. Debug flags
`--acceptance-root <existing absolute directory>` and
`--acceptance-debug-port <port>` isolate that automated fixture's storage and
WebViews; release has no such override. These are not the actual-program trial.

Mohammed requested that the current hands-on trial use the actual executable,
existing profile/database and real assistant without a copy or synthetic turn.
Current desktop evidence (`docs/evidence/aylul/2026-10-20-desktop-web-companion.md` at `0187365`)
separates passed automated gates and actual-profile readiness from that pending
human journey. For a read-only readiness check, run:

```powershell
pwsh -NoProfile -File apps/desktop/tests/desktop-web-companion.ps1 -VerifyOnly
```

Claude Windows packaging can redirect the launcher's LocalAppData path into its
package LocalCache directory. Set the existing physical database path in Local
Core Settings; do not create a replacement database when metadata appears empty.
The original Windows profiles migration 29 may have CRLF bytes. Core accepts
only that exact SQL variant through SQLx validation and preserves its stored
checksum; other mismatches remain errors.

Earlier physical
DPI, native hit testing and optimized resource measurements remain open in the
independent companion evidence (`docs/evidence/nylra/2026-10-16-independent-companion-acceptance.md`
at `0187365`).
