# NYLRA Windows shell

Imported from NYLRA into Shadows on 2026-10-18. Run the repository-root commands
below from `E:/Globalprojects/shadows`. The native package/identifier stay unchanged
so this relocation does not create a second installed identity or migrate stored
credentials. Hermes and loading the Shadows workspace are separate planned work.

React/TypeScript + Tauri 2 with separate main and desktop companion windows. Close hides the corresponding window. Left tray click,
Open NYLRA and Ctrl+Alt+Shift+F12 restore the main panel. Quit NYLRA exits.
Sign-in consumes backend auth-v1 1.1.0 through native Rust. AYLUL is now a
procedural Canvas companion on the desktop, with motion controls in main and
Show/Hide AYLUL in the tray. It starts idle; no simulated operation is reported as real
work, capture or speech playback.

`npm run dev --prefix apps/desktop` includes an explicitly opened development
studio for expressions, outfits, mock scenes and PNG export. Production excludes
that studio. The renderer uses local source and system fonts, without CDN assets.
Legacy PNG experiments are archived locally outside Git; see
[`assets/aylul/README.md`](../../assets/aylul/README.md).

Focused checks from `apps/desktop/`:

```powershell
npm run test:companion:unit
npm run test:companion:browser
npm run test:companion:production
npm run build
```

The browser checks use the installed Chrome channel through Playwright. They use
synthetic IPC for shell presentation tests, not a real backend session. Real
Windows and remaining physical acceptance are recorded in the
[independent companion evidence](../../docs/evidence/nylra/2026-10-16-independent-companion-acceptance.md).

From the repository root:

```powershell
npm ci --prefix apps/desktop
npm run build --prefix apps/desktop
cargo test --locked --manifest-path apps/desktop/src-tauri/Cargo.toml
cargo build --locked --manifest-path apps/desktop/src-tauri/Cargo.toml
cargo build --locked --manifest-path apps/desktop/hook/Cargo.toml
$env:NYLRA_BACKEND_URL='https://127.0.0.1:8443'
$env:NYLRA_DEV_CA=(Resolve-Path '..\shadows-backend\.local-runtime\tls\ca.pem').Path
$env:AYLUL_HOOK_EXE=(Resolve-Path 'apps/desktop/hook/target/debug/aylul-hook.exe').Path
& apps/desktop/src-tauri/target/debug/nylra-desktop.exe
```

The Windows build requires MSVC/WebView2. The native icon is a synthetic build
resource reproduced by the N0 `prepare-icon.ps1`, deleted with the probes and kept at
`41b8e84` (`docs/evidence/nylra/probes/2026-10-10-n0/prepare-icon.ps1`); it is not AYLUL art.

The commands above run a development build. Browser `npm run dev` previews the
interface; the independent OS window requires the native executable. This debug
workflow embeds the latest `dist`, so rebuild the frontend before rebuilding Rust.
Transparent padding is still part of the rectangular native pointer area.
The public local CA configures only the native client. A system browser must trust
that CA separately; do not bypass certificate verification. Windows CurrentUser
trust does not by itself prove Firefox trust. Start the sibling Docker stack using
its [handoff](../../../shadows-backend/docs/nylra-local-auth-handoff.md). Credentials
remain in Rust and DPAPI under the app-data directory, never the webview. Restart
validates a saved handle before showing account identity. Session/provider failures
preserve enrollment; new enrollment requires the explicit action.

Native library acceptance against synthetic local Keycloak (no browser UI claim):

```powershell
cargo build --locked --manifest-path apps/desktop/src-tauri/Cargo.toml --example auth_state
$env:NYLRA_ACCEPTANCE_RUNTIME=(Resolve-Path '..\shadows-backend\.local-runtime').Path
$env:NYLRA_ACCEPTANCE_CHILD=(Resolve-Path 'apps/desktop/src-tauri/target/debug/examples/auth_state.exe').Path
cargo test --locked --manifest-path apps/desktop/src-tauri/Cargo.toml native_compose -- --ignored
```

This explicit test reads only the synthetic runtime password in memory, uses
verified HTTPS and the real loopback listener, exercises fresh-process validation,
and cleans its encrypted acceptance store. Ordinary tests do not contact Keycloak.
The earlier native_fixture example remains a primitive-only test. See the
[joint evidence](../../docs/evidence/nylra/2026-10-11-joint-native-auth.md) for actual
checks, browser limitations and remaining production gates.
