# NYLRA source organization

Date: 2026-10-10. Updated: 2026-10-11, and for AYLUL on 2026-10-13.
Status: layout design; the implemented
main-only Windows slice is mapped in [the inspected code map](codebase/README.md).
This file owns client source placement. [Vision](vision.md) owns experience;
the [shared architecture](../../../shadows-backend/docs/architecture.md) owns system
boundaries. Listed paths are future destinations, not claims of existing code.

2026-10-18 runtime amendment: AYLUL remains the primary UI and opens Shadows on
request. The sidecar is a local integration runtime; the selected personal-pilot
brain is Hermes behind shadows-backend, the one external account/assistant entry.
Local Docker reuses Windows Hermes rather than installing it. Code intelligence is independently packaged and consumed
as a client capability. The [transition plan](plans/2026-10-18-nylra-transition.md)
records the 2026-10-18 tracked-source import into Shadows. The actual desktop
keeps `apps/desktop/` here, with its own npm lockfiles and separate Rust workspaces.
The inspected code map is rebased to this checkout. Existing `crates/` and `web/`
remain Core-only; no runtime or database ownership was merged. Mobile has no
implemented source in this snapshot. Historical evidence is in
`docs/evidence/nylra/`; the original ignored art archives remain at NYLRA.

[Delivery spec](specs/2026-10-10-nylra-delivery-design.md) and
[initial plan](history/plans/2026-10-10-nylra-initial-plan.md) define the
Windows-first experiments and phased acceptance. This file remains the owner of
source placement; the plans do not create parallel ownership maps.

## Selected approach

Keep desktop and mobile applications in one repository, each with its normal
toolchain. Share protocol definitions, fixtures and asset specifications across
languages; do not force Dart to reuse TypeScript application logic. Neither client
owns project rules or cloud NYLRA orchestration.

A single cross-platform UI would contradict the chosen React/Tauri and Flutter
direction. Separate repositories for these two clients would add coordination
without removing their shared product/protocol changes. This organization keeps
each build independent while allowing one review to update both consumers.

## Intended layout

```text
apps/
  desktop/
    src/
      app/                       composition, navigation and application lifecycle
      features/
        chat/                    conversation with AYLUL
        sessions/                live agent sessions and their steps
        approvals/               pending agent requests and their decisions
        grants/                  creating, listing and revoking grants
        connections/             phone pairing code and linked state
        preferences/             user settings presentation
        companion/               AYLUL presence
      api/generated/             generated cloud wire client/types
      data/                      scoped client data access and cache
      platform/                  typed Tauri invocation wrappers
      ui/                        reusable presentation primitives
    src-tauri/
      src/
        lib.rs                   native composition
        main.rs                  desktop entry point
        commands/                typed native operation boundary
        auth/                    backend session for this installation
        native_login/            system-browser login primitives
        sidecar/                 starts, watches and stops the Node sidecar
        windows/                 window lifecycle
        notifications/           native notification integration
        audio/                   native capture/playback integration (later)
      capabilities/              minimal permissions per window
    sidecar/                     AYLUL's laptop process, Node and TypeScript
      src/
        main.ts                  entry: started by Tauri, stopped when AYLUL quits
        backend/                 client for the owning backend's assistant API
        link/                    outbound WebSocket to shadows-backend
        agents/                  writes agent settings: hook entries and permission rules
        hooks/                   named-pipe server receiving aylul-hook events
        sessions/                agent sessions and steps built from hook events
        approvals/               holds a request or refusal until answered or timed out
        grants/                  rules AYLUL wrote, their expiry and revocation
        store/                   sessions, requests, answers and rules in node:sqlite
        shadows/                 client for the local Shadows API's projection
      tests/                     sidecar unit and pipe tests
    hook/                        aylul-hook, the command an agent's hook runs
    tests/                       desktop journeys
  mobile/
    lib/
      main.dart                  mobile entry point
      app/                       dependency composition and routing
      ui/
        chat/                    conversation views and view models
        sessions/                live agent sessions and their steps
        approvals/               pending requests and the person's decision
        connections/             laptop pairing presentation
        preferences/             user settings presentation
        core/                    reusable visual primitives
      data/
        repositories/            scoped client view/cache coordination
        services/                remote API access
        generated/               generated cloud wire client/types
      platform/                  mobile notification/audio integration
    test/                        unit and widget checks
    integration_test/            mobile journeys
assets/                          canonical character/media source assets
protocol-fixtures/               versioned consumer conformance examples
docs/
  codebase/README.md             actual ownership map when source exists
  evidence/                     dated desktop/mobile observations
```

Only create paths required by an implemented journey. Files inside a feature are
named for the work they do, not forced into a fixed number of layers. Native
Android/iOS and build configuration follow the tool-generated Flutter layout.
Each app keeps its own dependencies and lockfile. Asset packaging copies required
formats into build inputs; one asset manifest records origin, version and license.

## AYLUL desktop's sidecar

Decided 2026-10-13 in the
[delivery spec](specs/2026-10-10-nylra-delivery-design.md#independent-companion).
AYLUL desktop is three processes on the laptop, each with one job:

| Process | Owns |
| --- | --- |
| Tauri (Rust) | Windows, tray, login, the backend session, and the sidecar's lifetime |
| Sidecar (Node) | Local brain connection, agent sessions, context, approvals and grants |
| `aylul-hook` | One agent hook call: send the event over the pipe, return the answer |

- **The UI never reaches the sidecar directly.** It calls Tauri commands, and
  `src-tauri/src/sidecar/` relays them over the sidecar's standard input and
  output. The sidecar opens no local port.
- **The sidecar keeps no credential on disk.** Tauri's `auth` module holds the
  backend session. Its scoped handoff to the backend client is planned, not present
  in the imported sidecar startup. Hermes service credentials stay backend-side.
- **The pipe is restricted to the current Windows user.** `aylul-hook` refuses
  to run when the pipe is missing or owned by another user.
- **Failure is not consent.** If the sidecar is off, the pipe is missing or a
  request times out, `aylul-hook` returns control to the agent's own prompt.
  It never answers "allow" by itself.
- **The agent enforces grants.** A grant is a permission rule the person
  approved, written by `agents/` into the project's agent settings; Claude Code
  matches it, not AYLUL ([step 2(a)](specs/2026-10-13-aylul-hook-laptop-design.md#5-rules)).
  Model output, file content and session text never create one. Only the laptop
  creates or widens a grant; a revocation from the phone arrives through `link/`.
- **The selected brain is an internal backend runtime.** `backend/` consumes our
  authenticated assistant facade; it does not dial Hermes or copy a model loop. Existing
  backend `aylul-brain` code is retained until its own migration decision.
  Code intelligence has its own process/manifest and is not owned by sidecar startup.
- **`agents/` changes another program's settings.** It backs up the agent's
  hook configuration and shows the diff before writing it.

**`aylul-hook` is a Rust executable** (decided 2026-10-13). On Windows it
started in about 30 ms against about 200 ms for a Node script, needs no path to
`node.exe`, and is the real `.exe` that Claude Code's exec form requires
([probe](../evidence/nylra/2026-10-13-aylul-hook-probe.md)). It is its own crate in
`apps/desktop/hook/`, not part of the Tauri crate, so an agent's hook never
starts the desktop app.

> **OPEN: how the installed app ships Node for the sidecar.** Closes at the
> first installer build. Development runs the sidecar with the local Node.

## Client boundaries

Desktop components call feature state/actions, which use the data layer or narrow
platform wrappers. Sessions, approvals and grants come from the sidecar through
those wrappers, not from the cloud. They do not import a database client, model SDK or raw Tauri
commands throughout the UI. Tauri Rust owns native operations, not a second NYLRA
engine. Local Shadows navigation cannot become an alternate approval route.
Remote content is displayed as data, never privileged application code. Scope
capabilities to the window that needs them; an animated companion window should
not inherit every permission of the confirmation surface.

Mobile views observe view models; view models use repositories, which coordinate
API services and client caches. A repository is authoritative for the displayed
client projection only, never for accepting a project decision. Add a separate
use-case/domain layer only when actual client logic warrants it. Do not duplicate
server authorization or require a specific state-management package in this design.

Both clients distinguish draft, queued, accepted and completed outcomes from the
owner. Offline caches retain source scope/version/freshness. Account/workspace
switches clear or partition caches; revocation invalidates derived views. Navigation
to a new workspace cannot leave the previous workspace's sources in the composer.

## Protocols, contracts and tests

The backend owns its wire schema. Clients pin a versioned generated artifact and
must not reach into sibling TypeScript source. Desktop local Shadows integration
uses Shadows' own protocol, not the cloud client's types. Generated DTOs do not
become shared domain classes; map to presentation state only where useful.

Use the [source contract convention](../../../shadows-backend/docs/contracts/README.md)
for behavior-bearing modules: conversation state, confirmation interaction, cache
invalidation and native command boundaries are examples. Pure rendering widgets
are covered by their owning feature contract when they have no independent rule.
Keep contracts beside actual source; do not fill them from this directory plan.

Shared conformance fixtures describe observable wire behavior, not shared UI code.
Both clients should exercise source attribution, expired proposals, identical retry
outcomes, workspace switching and reconnect against those fixtures. End-to-end tests
must also reach the real owner; fake replies cannot prove approval enforcement.
Voice/background delivery and cross-device continuity require real-device evidence.

## Research basis

Flutter recommends UI/data separation, views/view models and repositories/services;
an extra domain layer is conditional. This design adopts those responsibilities
without deciding a state-management library.
[Flutter recommendations](https://docs.flutter.dev/app-architecture/recommendations).

Tauri's conventional layout separates frontend source from `src-tauri`, and its
capability system scopes native access by window/webview. We retain that split.
[Tauri layout](https://v2.tauri.app/start/project-structure/),
[Tauri capabilities](https://v2.tauri.app/security/capabilities/).
Checked 2026-10-10. The feature names above are our product-specific choices.
