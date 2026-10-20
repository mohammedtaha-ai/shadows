# Desktop Core runtime identity

- Status: accepted
- Asked by: Controller for the desktop UI/native integration, 2026-10-20
- Screen: docs/aylul/plans/2026-10-20-aylul-desktop-web-companion.md#task-3-core-runtime-identity-private-receipt-and-exact-native-origin

## Need

The desktop must identify its configured local Core before mounting API consumers.
Mohammed approved this plan for inline execution on 2026-10-20. The controller
records that approval here; no independent lead response or message is claimed.

## Fields

| Call | Sends | Gets back |
|---|---|---|
| GET /api/runtime | no body | RuntimeIdentity: runtime_id and configuration_id strings, or 404 ErrorBody with RUNTIME_NOT_CONFIGURED without desktop metadata |
| shadows serve | paired --desktop-record absolute path and --desktop-configuration UUID | private RuntimeReceipt after Core readiness |

RuntimeReceipt flattens RuntimeIdentity and includes the actual canonical database path, origin and ProcessIdentity
(pid, started_at, executable). It stays native-only. The native reader additionally
verifies the OS process instance. This does not add authentication or a new Core
business service. The binary owns metadata; the HTTP adapter translates it.

## Errors

Invalid pairing, UUID, relative receipt or non-loopback desktop bind refuses
startup. Null/foreign HTTP origins remain refused. Missing or mismatched native
receipt is unavailable; it never selects another DB or kills an existing listener.

## Backend answer

Controller implementation follows the approved design and plan. Runtime metadata
uses a router Extension; AppState and AppCore services are unchanged. Canonical
OpenAPI and TS generation follow the route table.

## Frontend reply

Controller accepts these exact fields for the native startup consumer.
Browser API configuration remains unchanged.

## Ruling

Direct human plan/execution approval is the authority for this cross-owner
agreement. This is not an assertion that Hamoud or Masada sent a separate reply.
