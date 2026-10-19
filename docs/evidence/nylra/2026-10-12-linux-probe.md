# Linux AYLUL window and streamed chat probe (2026-10-12)

Probe source: the `linux_probe` module, `apps/desktop/src/app/linux-probe.tsx`,
`probe-stream.ts` and `docs/evidence/probes/2026-10-12-linux/` at `f9789af`, tagged
`probe/2026-10-12-linux-integration`. It was not merged; this file keeps its findings.

This branch adds an opt-in Linux preview of the approved AYLUL idle image and
the backend's synthetic MiniMax conversation stream. It does not add product
login, mobile support, animation, or a production connector.

## Reproduce

1. From `apps/desktop`, run `npm ci`, `node --experimental-strip-types
   src/app/probe-stream.test.mjs`, and `VITE_NYLRA_PROBE=1 npm run build`.
2. On a Linux host with Rust 1.94, install Tauri's GTK 3, WebKit2GTK 4.1,
   appindicator, SVG, OpenSSL and `pkg-config` development packages. From the
   repository root run `cargo check --locked --manifest-path
   apps/desktop/src-tauri/Cargo.toml` and `cargo build --locked --manifest-path
   apps/desktop/src-tauri/Cargo.toml`. The sibling Dockerfile (`docs/evidence/probes/2026-10-12-linux/Dockerfile` at `f9789af`)
   provides these dependencies and Xvfb for headless verification. The managed
   cloud environment used a BuildKit `proxy_ca` secret while building it.
3. Start a disposable Shadows daemon and the sibling backend's local probe
   before asking about the selected synthetic project. The Rust window itself
   does not need them to open. Run the Tauri executable in a Linux graphical
   session. The preview is opt-in at Vite build time.

## Observed checks

| Check | Observation |
| --- | --- |
| SSE client tests | Five passed: split delta frames, terminal error, missing terminal event, CRLF/multiline data, and bounded unfinished frames. The failed CRLF test justified replacing the hand parser with eventsource-parser 4.1.2. |
| Vite probe build | Succeeded; bundled the approved `assets/aylul/states/v1/idle.png` (732,444 bytes). |
| Vite local server | Served the preview page and idle PNG with HTTP 200. |
| Native Linux `cargo check` in Rust 1.94 Debian container with GTK/WebKit deps | Succeeded after gating Windows-only modules and supplying a Linux PNG icon. |
| Native Linux `cargo build` in the same container | Succeeded, producing the Tauri executable. |
| Native executable under Xvfb | Opened a visible `NYLRA · Linux probe` webview. The captured window.png (`docs/evidence/probes/2026-10-12-linux/window.png` at `f9789af`) visibly contains approved AYLUL idle artwork and the chat form. `run-headless.sh` exited 0. |
| Native button + SSE fixture under Xvfb | `run-headless.sh <output> --mock-chat` clicked Ask NYLRA against `mock-chat.py` on loopback. The captured window-chat-fixture.png (`docs/evidence/probes/2026-10-12-linux/window-chat-fixture.png` at `f9789af`) shows `Complete` and `Synthetic Lighthouse`. This fixture made no provider or Shadows call. |
| Native window → backend SDK → real Shadows owner with provider fixture | `run-live.sh <fresh-output> --provider-fixture` completed with one owner tool execution, two SDK steps and 20 text characters. Screenshot window-owner-fixture.png (`docs/evidence/probes/2026-10-12-linux/window-owner-fixture.png` at `f9789af`) shows the completed owner-derived reply. Only provider responses were simulated. |
| Native window → backend → real MiniMax | MiniMax explicitly rejected the request with Token Plan usage limit code 2067 / insufficient_balance_error. Native UI reported unavailable/incomplete. |

The initial bare host `cargo check` stopped at missing `glib-2.0` development
metadata; the container supplied the required native packages. The initial
container check then found unconditional Windows-only imports and the missing
Tauri PNG icon. Those are the reasons for the Linux entry-point change, not
provider or frontend failures.

The first headless launch failed because the container's non-root user lacked a
writable home directory. `run-headless.sh` now supplies a private writable home
directory through explicit XDG config/data/cache paths and a private XDG runtime
directory; the repeated run opened and captured the window.

The cross-project runtime runtime.Dockerfile (`docs/evidence/probes/2026-10-12-linux/runtime.Dockerfile` at `f9789af`) uses Node 24
and Debian 13 so it can run the host-built Shadows binary requiring GLIBC_2.39.
Mount the three repositories read-only, the disposable probe output writable,
and the pinned Claude executable read-only at their documented `/workspace`
paths. Run from `/workspace/nylra`. `run-live.sh` starts the backend's retained
`native-live.mjs`, which creates a fresh Shadows database/project and only then
opens the UI. Keep real provider keys and proxy/CA configuration in process
environment only. Do not commit a real provider's raw errors or response text.

The Linux entry point creates one local Tauri webview. It deliberately omits
Windows DPAPI/browser-login commands and tray integration. The Linux-specific
Tauri CSP permits only the local probe URL `http://127.0.0.1:8765` in addition
to Tauri IPC; the shared Tauri config keeps its original CSP. Its streamed UI
marks success only after `done`; `error` or an incomplete
stream is reported as unavailable. Source contracts and the inspected code map
mark these as probe paths.

## Remaining evidence boundary

The captured windows prove the native webview rendered the approved idle asset
and that its button can display a completed SSE reply from a local fixture.
The backend's earlier successful **real MiniMax** round trip was driven by a
local HTTP client. The later native click reached MiniMax but was refused by
its plan limit. The complete native/owner/SDK path was then validated with a
deterministic provider fixture. OpenRouter remains blocked by the current
environment policy; its quota or key validity is unknown.
This preview also does not test persisted conversation, identities, project
grant, offline reconnection or production authority.
