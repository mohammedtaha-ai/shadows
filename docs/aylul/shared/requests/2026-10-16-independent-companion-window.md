# Independent companion window integration

- Status: accepted
- Asked by: Character and Desktop UI, 2026-10-16
- Owner: Native
- Serves: docs/aylul/native/specs/2026-10-16-independent-companion-design.md

## Need

Host the existing procedural renderer in a transparent native companion window,
separate from main, without granting account authority to the companion. Native
owns visibility, motion pause, placement and lifetime. Character consumes explicit
render permission; Desktop UI exposes controls and retains account functionality.

## Interface

Proposed names below are design names, not assertions that commands exist.

| Call / message | Sends | Gets back |
| --- | --- | --- |
| `companion_ready` (companion only) | No caller-supplied label or visibility | Native state snapshot; initial presentation readiness enables the pending initial show |
| `companion_failed` (companion only) | No error text, identity, label or native coordinates | Native render denial and stable retryable rendering error; main/tray retain recovery |
| `companion_state` (main or companion) | No arguments | `{ revision: number, visible: boolean, renderAllowed: boolean, paused: boolean, error: string or null }`, with no account material |
| `set_companion_visible` (main only) | `{ visible: boolean }` | Reconciled native snapshot or stable error; tray calls the same native owner |
| `set_companion_paused` (main only) | `{ paused: boolean }` | Native snapshot or stable error |
| `drag_companion` (companion only) | No native window identifier or coordinates | Native drag outcome or stable error |
| `open_main_from_companion` (companion only) | No arguments; deliberate local click | Explicit main open outcome or stable error |
| `companion-state` (native to each allowed local window) | The same versioned snapshot | Typed subscription; latest revision wins |

Native state starts with render permission denied. The surface subscribes before
requesting state/readiness and ignores stale snapshots. Permission is separate
from observed window visibility during native transitions. The frontend does not
author visibility events or supply trusted caller identities.

## Errors

| When | The asking side does |
| --- | --- |
| State subscription or synchronization fails | Keep drawing stopped, show a local failure and retain main/tray recovery |
| Show/hide/pause/drag fails | Report a stable error; do not optimistically claim success |
| Companion initialization fails | Keep account and main/tray controls usable; explicit retry may reconstruct only the companion |
| Unexpected caller or account request from companion | Native rejects; no identity or window state mutation |

## Answer

Native accepts this interface following the user's written specification approval
on 2026-10-16. It retains separate render permission, real native visibility and
main-only account authority. Its implementation belongs to Task 1 of the native
plan; dependent presentation tasks consume the implemented owner.

## Reply

Character and Desktop UI accept the versioned state and narrow commands. They
must not implement native visibility or authority rules in the renderer.

## Ruling

The teacher records the user's subsequent written specification approval with
the instruction to begin. This accepts the cross-side interface. The implementation
plan remains a separate review boundary before product changes.

Implementation ruling, 2026-10-17: accept the narrow companion-only failure
report above. Final review reproduced that errors after readiness otherwise left
native believing the surface was healthy and prevented Retry. This extends the
same recovery owner, grants no new account action and accepts no model text.
