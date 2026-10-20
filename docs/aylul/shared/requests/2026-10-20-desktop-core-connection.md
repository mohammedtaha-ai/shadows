# Shared dashboard native connection

- Status: accepted
- Asked by: Desktop UI scope, 2026-10-20
- Owner: Native scope
- Serves: docs/aylul/specs/2026-10-20-aylul-desktop-web-companion-design.md

## Need

Load the existing web dashboard only after a saved local Core has been verified.
Native owns launch, process identity, persistence and local-resource response policy.

## Interface

| Call / message | Sends | Gets back |
|---|---|---|
| desktop_connection | Nothing; main only | ConnectionView |
| desktop_connection_config | Nothing; main only | LaunchConfig references or null |
| save_desktop_connection | config; main only | Whether normalized identity changed |
| retry_desktop_connection | Nothing; main only | ConnectionView after explicit retry |
| desktop-connection | Native to main | Revisioned ConnectionView |
| open_external | Validated HTTP(S) URL; main only | OS launch outcome |

ConnectionView uses camelCase: revision, state, configurationId, origin, error.
Only ready exposes origin. LaunchConfig uses configurationId, executable, db, node,
adapter, harness, aylulConfig and origin. No receipt, credential or token is returned.
Changed configuration invalidates old startup work and reloads main at Settings.
Native lifetime survives main hiding/reloading; Core survives desktop exit.
Companion receives none of these commands or the Core endpoint in CSP.

## Errors

| When | The asking side does |
|---|---|
| Invalid/missing paths or nonliteral loopback origin | Show fixed core_path_invalid/core_origin_invalid |
| Busy endpoint or identity mismatch | Remain failed; do not attach or create a replacement DB |
| Startup timeout | Remain failed until explicit Retry; retain pending process identity |

## Answer

The controller records Mohammed's approved specification and instruction to execute
the plan as acceptance of this interface across Native and UI scopes. This is not
a claim of separate lead messages. The native owner is implemented first in Task 4.

## Reply

The shared web bootstrap consumes the implemented owner, subscribes before snapshot,
then freezes the verified origin before dynamically importing API consumers.
