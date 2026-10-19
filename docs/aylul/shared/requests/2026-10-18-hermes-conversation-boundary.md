# AYLUL conversation through the owning backend

- Status: accepted (controller implementing both sides under the human's normal-chat request)
- Asked by: controller, 2026-10-18
- Owner: native/sidecar; backend owns the public wire schema
- Serves: [local application-core plan](../../native/plans/2026-10-18-aylul-local-runtime-hermes.md)
- Backend contract: [Hermes-backed assistant](../../../../../shadows-backend/docs/shared/requests/2026-10-18-hermes-backed-assistant.md)

## Human ruling

NYLRA uses shadows-backend as its single entry point for account and assistant
services. Hermes is an internal part of that deployment. For local testing, reuse
the installed Windows Hermes from Docker; do not install a second Hermes there.
The earlier direct-desktop-to-Hermes draft is superseded before implementation.

## Implemented local interface

Main-only `assistant_call` is a closed native authenticated HTTP boundary. It
reuses the account store/origin. The earlier proposed sidecar verbs and bearer
handoff are superseded; `aylul_call` still owns existing local hook actions.

| Call | Input | Result |
|---|---|---|
| `status` / `list` | `{action}` | Backend availability / account conversations |
| `create` | `{action,commandId}` | Backend-owned conversation snapshot |
| `send` | `{action,conversationId,commandId,text}` | Snapshot after durable backend admission |
| `snapshot` | `{action,conversationId}` | Bounded latest snapshot with omissions |
| `stop` / `recover` | `{action,conversationId,turnId}` | Snapshot after the addressed idempotent control |

Backend origin comes from the native-selected account connection. Backend
authorization resolves account, conversation ownership and Hermes binding.
The UI cannot choose an upstream host/profile/session/run or supply a service key.
No generic RPC proxy. No account credential handoff to the sidecar is needed.
The backend TypeBox contract 0.5.0 owns wire payloads; Rust maps only fixed paths.

## Outcomes and lifetime

Prepared locally is distinct from accepted remotely, delivered to Hermes and
completed. Unknown writes retain their command ID; no automatic resend. Scope
cached events to account/connection generation and invalidate on logout or revoke.
Network calls run through Tauri's blocking worker with the native HTTP 15-second
timeout. They bypass the sidecar's 10-second stdio boundary. Poll only snapshots;
the backend contract owns public payload bounds and replay. No public SSE yet.

Hermes credentials never enter the desktop. Local rule/request approval keeps
its existing owner and fingerprint checks; neither assistant prose nor a server
tool request calls those operations automatically. Companion/workspace labels
cannot invoke the new main-only actions. Closing AYLUL leaves the remote brain
runtime alive; the existing local sidecar lifecycle remains native-owned.

## Answer and closing test

The human authorized implementing this path now. The controller accepted the
specific native mapping and backend-generated payloads while implementing both
sides; no independent lead review is claimed. Native allowlist/ACL, real native HTTP,
backend account isolation/replay and browser conversation continuity are tested.
Physical desktop launch remains open after automatic review rejection. Public SSE,
Core/MCP and code intelligence are separate slices. Existing hook/avatar owners stay.
