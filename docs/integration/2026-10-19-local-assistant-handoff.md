# Local assistant handoff

Date: 2026-10-19. Physical GUI and production acceptance remain separate from
fixture and native checks.

## Windows development

Hermes must already expose its native session/run API. Installing the CLI alone
does not start that listener. The existing backend development helper
`scripts/start-hermes-host.ps1` reuses the Windows installation and separate chat
home; it does not install another Hermes.

From the Shadows checkout, the user can run:

```powershell
pwsh -NoProfile -File scripts/start-local-assistant.ps1 `
  -HermesKeyFile E:\Globalprojects\shadows-backend\.local-runtime\hermes-chat\service-key `
  -OpenDesktop
```

This is the Hermes native service key file, not the MiniMax/provider API key.
The script passes only the file reference; Node resolves its contents internally.
It builds the portable client, assistant and daemon and prepares a private
runtime directory under local AppData. `-OpenDesktop` explicitly builds and opens
the native app for the person; automated acceptance does not launch the GUI.

`-SkipBuild` reuses existing outputs. `-RuntimeDirectory` names a dedicated
runtime directory; the launcher refuses to change unrelated nonempty directories.
Its environment applies to the desktop it starts. Other native launches require
`SHADOWS_DAEMON_DESCRIPTOR` to name the private `connection.json`. Never put the
descriptor capability in frontend configuration.
Exit an existing desktop instance from its tray before relaunching so the new
build and descriptor environment are loaded.

The app defaults to Local when a native installation is available unless the
person already saved a Server preference. Choose Local explicitly in that case.
Closing the desktop does not stop the daemon. Hook-sidecar lifetime still follows
the desktop until its separately verified transfer.

If the daemon is already running, attach the desktop instead of starting another
owner for the same runtime directory:

```powershell
$env:SHADOWS_DAEMON_DESCRIPTOR = Join-Path $env:LOCALAPPDATA 'Shadows/assistant-runtime/connection.json'
& E:/Globalprojects/shadows/apps/desktop/src-tauri/target/debug/nylra-desktop.exe
```

## Direct daemon invocation

All paths below are operator-selected absolute paths. Sensitive runtime files
belong outside tracked source; configuration contains only a key-file reference.

```json
{
  "database": "C:/private-runtime/assistant.sqlite",
  "hermes": {
    "origin": "http://127.0.0.1:8642",
    "network": "development-host",
    "keyFile": "C:/private-runtime/hermes-service-key"
  }
}
```

```text
shadows daemon --node <absolute-node> --entry <absolute-assistant/dist/main.js>
  --config <absolute-assistant.json> --descriptor <private-connection.json>
  --bind 127.0.0.1:4320
```

Optional `--planner-config <absolute-json>` starts existing planning on the same
host. Fields: `db_path`, `node_path`, `adapter_path`, `harness_path`,
`allowed_origins`. Paths are absolute. Omitting it starts no Planner and needs no
Claude/ACP paths. Existing `shadows serve` behavior is retained.

The same runtime can run on a server with matching platform binaries and Node
dependencies. This initial listener remains loopback-only. Remote account use
retains shadows-backend; backend-to-daemon deployment/authorization is a later
adapter. Do not expose the local listener publicly.

## Data and selection

Local uses the Node assistant SQLite file; Server uses existing account-backed
conversation bindings. Hermes owns its history and memory. Switching sources
clears visible history and drafts and ignores late replies. It does not copy
history, change providers or resend a request on the other source. Recover an
uncertain request only on its original source with its original identity.

## Reproducible verification

Run `npm test` in `packages/hermes-client` and `services/assistant`. After building
the Rust binary and Node service, run `node scripts/assistant-daemon-journey.mjs`
from the repository root. It uses an owned native HTTP fixture, makes no provider
call and verifies authenticated requests, restart, recovery and one execution.
Installed-Hermes readiness is separate. Bot management, context sharing, public
streaming and computer control are outside this move.
