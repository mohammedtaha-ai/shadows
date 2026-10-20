# Packaged desktop Origin and Core access

Recorded 2026-10-20 on Windows. This is measured evidence, not design authority.

## Result

PASS. Both owned local webviews sent the actual HTTP Origin
`http://tauri.localhost`. The packaged main used the existing web production
build and a real local Core on an isolated database/project directory:

- GET /api/projects: 200.
- JSON project creation/replay: 200.
- EventSource /api/subscribe: received caught-up.
- Main's hide_main IPC: allowed; the companion-labelled surface: refused.
- An unrelated Origin https://foreign.invalid: 403.
- Web fonts loaded, the main root measured 1000 CSS pixels, no CSP violation
  was reported by the probe pages.

The companion measurement used a minimal local probe page with the real
companion capability label. This does not establish character drawing or the
retained auth/hook IPC behavior; those have their own acceptance gates.

Raw credential-free measurements: [result JSON](2026-10-20-desktop-webview-origin.json).

## Boundary tested

Main connect-src kept the native IPC sources and added only the two exact
owned test endpoints, http://127.0.0.1:43381 (Core) and
http://127.0.0.1:43382 (collector). Companion connect-src added only the collector,
not Core. Native resource-response policy kept the generated script policy and
allowed style-src-attr 'unsafe-inline' for dashboard positioning styles.
No script unsafe-inline, unsafe-eval, wildcard or null origin was introduced.

This closes the desktop design's packaged Windows Origin/fetch/EventSource OPEN.
Task 4 still owns final native bootstrap, hash-route reloads, missing-asset
behavior, endpoint selection and production assembly; those are not claimed here.

## Reproduction and provenance

Temporary probe source is retained at Git commit
`26464f6c5da398783e6361cee8279d17df9d4eba`:

- apps/desktop/src-tauri/examples/desktop_origin_probe.rs
- apps/desktop/src-tauri/examples/desktop_origin_probe.js
- apps/desktop/tests/desktop-origin-probe.mjs
- apps/desktop/tests/desktop-origin-probe.ps1
- The temporary examples resource hook in apps/desktop/src-tauri/build.rs

Builds used existing default caches, Tauri 2.11.5 and Node v24.15.0:

1. npm --prefix apps/desktop run build
2. Set VITE_SHADOWS_URL=http://127.0.0.1:43381 for npm --prefix web run build.
3. Assemble those generated assets under apps/desktop/dist, retaining the
   companion entry and adding the minimal measurement page.
4. cargo build --locked --manifest-path apps/desktop/src-tauri/Cargo.toml --example desktop_origin_probe
5. node apps/desktop/tests/desktop-origin-probe.mjs <absolute owned fixture config>

The fixture config names the existing debug Shadows executable, the owned
scratch DB/project directory, explicit Node/fake-acp paths, test endpoints and
the native probe executable. The harness paths are unused: no session or model
turn was opened. There was no provider use or credential access.

Windows PowerShell 5.1 refused the unsigned wrapper. The same Node driver ran
directly; no execution policy was changed.

The first native example exited with 0xC0000139 before Rust main. The generated
build output linked resource.lib to bins only, while the example imported
Common Controls v6 functions. Linking that same resource into the example
resolved the loader failure; the successful probe was rerun without warnings.
The probe-only hook and sources are removed after this evidence.

Owned Core/native test processes were stopped after each run. No personal Core
database, provider configuration, account action or hook installation was used.
The probe used the normal Tauri webview cache; final acceptance must additionally
isolate webview user data through its explicit acceptance root. The normal web
build retains its existing large-chunk warning.
