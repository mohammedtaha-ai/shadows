# AYLUL conversation through the owning backend

- Status: asked
- Asked by: controller, 2026-10-18
- Owner: native/sidecar; backend owns the public wire schema
- Serves: [local application-core plan](../../native/plans/2026-10-18-aylul-local-runtime-hermes.md)
- Backend contract: [Hermes-backed assistant](../../../../../shadows-backend/docs/shared/requests/2026-10-18-hermes-backed-assistant.md)

## Human ruling

NYLRA uses shadows-backend as its single entry point for account and assistant
services. Hermes is an internal part of that deployment. For local testing, reuse
the installed Windows Hermes from Docker; do not install a second Hermes there.
The earlier direct-desktop-to-Hermes draft is superseded before implementation.

## Proposed local interface

Main-only `aylul_call` remains the native boundary. The following sidecar verbs
are proposed UI conveniences, not upstream Hermes RPC or public HTTP endpoints.

| Call | Input | Result |
|---|---|---|
| `brain.status` | `{}` | Credential-free backend/assistant availability |
| `conversation.open` | `{commandId}` | Backend-owned conversation projection |
| `conversation.send` | `{conversationId,commandId,text}` | Prepared/local request or backend acceptance receipt |
| `conversation.read` | `{conversationId}` | Bounded cached backend snapshot/events with omissions |
| `conversation.stop` | `{conversationId,turnId,commandId}` | Actual backend cancellation request outcome |

Backend origin comes from the native-selected account connection. Backend
authorization resolves account, conversation ownership and Hermes binding.
The UI cannot choose an upstream host/profile/session/run or supply a service key.
No generic RPC proxy. Account credential handoff to the sidecar requires a native
implementation and logout/revocation tests; it does not exist in the imported code.

## Outcomes and lifetime

Prepared locally is distinct from accepted remotely, delivered to Hermes and
completed. Unknown writes retain their command ID; no automatic resend. Scope
cached events to account/connection generation and invalidate on logout or revoke.
Use background network work and bounded polling behind the existing 10-second
native RPC limit. The backend contract owns public payload bounds and replay.

Hermes credentials never enter the desktop. Local rule/request approval keeps
its existing owner and fingerprint checks; neither assistant prose nor a server
tool request calls those operations automatically. Companion/workspace labels
cannot invoke the new main-only actions. Closing AYLUL leaves the remote brain
runtime alive; the existing local sidecar lifecycle remains native-owned.

## Answer and closing test

The human approved this routing direction. Exact local/native and backend payload
contracts still require their owners' review; `asked` is not fabricated acceptance.
Close after main-only IPC, account-revocation invalidation, generated contract
consumption, exact conversation routing, command replay/conflict and secret-free
errors pass with the new backend path. Preserve all existing hook/avatar acceptance.
