# Runtime connections and endpoint disposition

Date: 2026-10-18. Status: inspected source plus backend-first routing. Docker host
network/client checks are separate from live Hermes/model acceptance. Design authority remains the
[AYLUL integration owner](../aylul/specs/2026-10-18-aylul-hermes-semantic-context-design.md).

## Application cores

| Application core | Composition | Owned state | Other runtimes call |
|---|---|---|---|
| Shadows | Existing `crates/shadows-core/src/app.rs::AppCore` | Ten services, Core SQLite, managed ACP sessions | Existing HTTP/SSE/MCP; future scoped companion operations |
| AYLUL local | Proposed `apps/desktop/sidecar/src/app.ts::AylulAppCore`; Tauri owns platform startup | Existing hook/session/approval store, local context, backend client projections | Main-only Tauri `aylul_call`, translated to allowlisted stdio calls |
| Backend | Existing `shadows-backend/src/app/compose.ts`; its own application composition | Accounts/devices, external conversation ownership and internal Hermes bindings | One authenticated public origin for NYLRA |
| Code intelligence | Independent package/entry point selected by CI-3 | Registered project roots and LSP state | Selected package query protocol/MCP |
| Hermes | Installed upstream API-server runtime; internal service within backend deployment | Its profile, sessions, runs and memory | Private native REST runs/session API, never direct desktop access |

Multiple application cores do not require renaming the existing Rust symbol or
splitting its ten services. They require separate startup/state/shutdown owners.
There is no umbrella core, shared database, universal proxy or cloud connection
inside Shadows `AppCore`. `aylul-hook` remains a short-lived client of the local
named pipe; the avatar renderer and native windows keep their current owners.

## Inspected sources

| Snapshot | Protocol/source owners |
|---|---|
| Shadows `8973877` | `api/openapi.json`; `crates/shadows-core/src/app.rs`; HTTP `conversation.rs`, `queue.rs`, `sse.rs`; desktop native auth and sidecar contracts |
| shadows-backend `cbacd2a` | `src/transports/http/{auth-routes.ts,nylra-paths.ts,nylra-routes.ts}`; `api/{auth-v1,nylra-v1}.openapi.json`; `src/transports/laptop-link/contract.yaml`; conversations `reply-port.ts`; app `compose.ts` |
| Hermes `ccc695635094d7615373db267b77f894d978b1a0` | `apps/shared/src/gateway-contract.openrpc.json`; `tui_gateway/contracts/{canonical.py,canonical_sessions.py,prompt_voice.py,sessions.py}`; `gateway/{session_contract.py,run_api.py}`; `tui_gateway/ws.py`; `apps/desktop/docs/canonical-gateway-protocol.md` |
| Installed Hermes v0.22.2 `d923a5eb487a17c0a68801bba9ef0f56c81040a8` | Installed `gateway/platforms/{api_server.py,api_server_runs.py,api_server_run_idempotency.py}`; owns the selected host API |

The F-drive checkout is a different reference version, not the installed target.
The installed source is the selected local transport baseline; canonical WS
findings below remain comparison evidence. No
Shadows MCP navigation or Rust LSP was connected; maps/contracts preceded scoped
file reads. A source operation and a planned operation have different labels below.

## Desktop to existing shadows-backend

These native calls already exist in `apps/desktop/src-tauri/src/auth/service.rs`.
Retain their implementation and tests. New assistant operations use the same
backend origin/identity with their own server-side authorization. Configuration
currently uses `NYLRA_BACKEND_URL`.

| HTTP operation | Current caller / purpose | Pilot disposition |
|---|---|---|
| `POST /v1/auth/login` | Native browser-bound login start | Retain |
| `POST /v1/auth/return` | Native ticket redemption/enrollment | Retain |
| `GET /v1/auth/session` | Native persisted account validation | Retain |
| `POST /v1/auth/logout`, `/v1/auth/logout-all` | Explicit native surrender | Retain |
| `DELETE /v1/devices/{deviceId}` | Native revoke current device | Retain |
| `GET /v1/auth/provider-logout` and callback/login callback routes | Browser/provider flow owned by backend | Retain existing auth journey |

Backend sign-in is required for its assistant facade. The internal Hermes key is
different from the desktop account bearer and stays server-side. A failed Hermes
connection does not disable login or local monitoring. No account/data migration
or deletion follows automatically from selecting a different brain.

The following backend capabilities are implemented **on the server**, but the
imported sidecar has no client binding to them. Do not describe them as a working
desktop path or rename them to Hermes methods.

| Backend family | Canonical operations | Pilot disposition |
|---|---|---|
| Laptop relay | Pairing-code/claim/list/unlink under `/v1/nylra/laptops`; WebSocket `GET /v1/nylra/laptop-link` | Preserve, defer desktop relay integration |
| Session/account observation | `/v1/nylra/sessions`, per-session `/events`, account `/v1/nylra/events` SSE | Preserve, not required for local hook observation |
| Account text conversations | `/v1/nylra/topics`, per-topic `/events`, `/turns`, `/turns/{turnId}/cancel`, `/connection` | Preserve existing implementation; Hermes facade is separately specified before cutover |
| BYOK provider settings | `/v1/nylra/connections`, `/default`, `/{connectionId}/test` | Preserve; do not copy encrypted provider credentials into Hermes |
| Approvals, grant revocation, push | Corresponding generated `nylra-v1` operations | Registry says planned; no real implementation claimed |

The backend `ReplyPort` accepts text history/message and returns text/finish/error.
It cannot represent the entire Hermes run/session protocol. Extend the backend
through the [assistant owner](../../../shadows-backend/docs/backend/specs/2026-10-18-hermes-backed-assistant-design.md),
with account-to-Hermes bindings and an allowlisted public facade, rather than
pretending a text-only model adapter is a complete proxy. Existing provider chat
is retained; one turn uses exactly one brain path.

## Backend to installed Hermes — selected path

```text
NYLRA -> https://our-backend -> account authorization -> Hermes adapter
                                                   -> installed native API
Local Docker: http://host.docker.internal:8642
Server:       http://hermes:8642 on its private service network (or reviewed TLS)
```

Docker is a deployment boundary, not another installation of Hermes for this
local test. The server packages backend and Hermes as one product while each
keeps its own process, storage and lifecycle. The client-facing origin does not
change with the internal service address. Do not publish a raw Hermes listener.

| Installed native operation | Backend use |
|---|---|
| `GET /health` | Reachability/identity only, no model call |
| `GET /v1/capabilities` | Authenticated native feature and idempotency-mode negotiation |
| `POST /api/sessions`, `GET /api/sessions/{session_id}` and `/messages` | Explicit server-owned binding and snapshot/history |
| `POST /v1/runs` with `Idempotency-Key` | Durable backend command mapped to an upstream run |
| `GET /v1/runs/{run_id}` and `/events` | Run status and SSE; upstream positions are not backend cursors |
| `POST /v1/runs/{run_id}/stop`, `/steer`, `/approval` | Separate allowed controls with exact account/run ownership; approval needs explicit person authority |

The installed capability response declares tools execute on the API-server host
(`server_agent`, `split_runtime:false`). It cannot infer a laptop code root when
deployed on the server. Use the local evidence bridge later. Verify explicit
session continuity and actual run recovery rather than assuming stateless
OpenAI-compatible requests provide persistent assistant memory.

The [Docker readiness tool](../../../shadows-backend/docs/hermes-host-handoff.md)
checks health/auth/native features with time/byte limits and redirect refusal.
The actual default host API was not listening initially. A synthetic listener
proved network plumbing, then an API-only instance from the installed Hermes
runtime passed Docker health/auth/capabilities using a disposable home/key. No
model or conversation ran; no personal config or key was read or modified. Both
listeners were ephemeral; persistent operator-selected service setup remains open.

Later B-H1 checkpoint: backend `8fbb068` added the internal native client and
proved a real configured MiniMax-M3 session journey. Lost-ACK identical replay,
completed-result recovery after API restart, session continuity, conflict refusal
and native stop/SSE cursor replay were observed. See the
[dated journey](../../../shadows-backend/docs/evidence/2026-10-18-hermes-session-journey.md).
The account facade and PostgreSQL command intents were subsequently implemented
in backend `02aca92`, contract 0.5.0. The desktop uses native authenticated HTTP
for status/list/create/snapshot/send/stop/recover under `/v1/nylra/assistant`.
The first public transport is snapshot polling; public assistant SSE remains
deferred. The acceptance receipt file is not product persistence. No local
code/context bridge or Core mutation is activated by normal chat.

## Different F-drive Hermes WS reference — not selected for this slice

That reference has authenticated WebSocket **`/api/ws`**. A method name below
is JSON-RPC on that connection, not a separate REST URL. The pinned generated
OpenRPC contains both ordinary `methods` and **`x-canonical-methods`**. Canonical
overrides must win; their dependencies stay referenced to upstream schemas.

| Canonical method | Required integration behavior |
|---|---|
| `runtime.describe` | Verify owner `instance_id`, `profile_id`, `authority_epoch` and capabilities before opening a conversation |
| `session.create` | Retain one `request_id` for one create intent; use source `gui`; store returned session/snapshot/subscription identity |
| `session.resume` | Use the stored exact session, not most recent/title matching; adopt snapshot revision, generation and replay cursor |
| `session.list` | Optional recovery picker, not a hidden fallback to another session |
| `prompt.submit` | Send exact text with retained `submission_id`/`input_id`; result is `AdmissionStatus`, not legacy `{status:"streaming"}` |
| `prompt.receipt` | Resolve `admission_id`; retain `ref`, status/outcome and generation; `unknown` is not permission to resend |
| `session.events.since` | Canonical `last_sequence` + `replay_epoch`; gap/epoch change/snapshot_required triggers snapshot refresh |
| `session.interrupt` | Include the captured `execution_generation`; applies only to this Hermes run |
| `session.detach` | Drop this subscription; do not stop upstream runtime when AYLUL exits |

Canonical snapshot includes `stored_session_id`, `subscription_id`,
`authority_epoch`, `revision`, `execution_generation`, `replay_epoch`,
`last_sequence`, pending admissions and prompts. Admission states are
`queued | started | unknown | terminal`; a terminal outcome must be inspected.
Local `commandId`, submitted input identity and upstream `admission_id` remain
different fields. Hermes session IDs never replace observed Claude/Core IDs.

The legacy `session.history`/resume shapes are not automatically valid canonical
history APIs. Use the canonical resume snapshot first. The notification envelope
and actual terminal/cancellation behavior require a live protocol probe; source
schemas alone do not establish delivery, durability or cancellation completion.

**Authentication boundary:** native local Hermes uses `hermes-gateway-v1` plus
a fresh single-use ticket subprotocol minted by the owner's private control
surface. The inspected upstream document explicitly keeps remote SSH/URL/OAuth
on their existing paths. A tunnel alone does not establish canonical access;
neither a backend bearer nor a generic `/api/auth/ws-ticket` is assumed to mint
the required grant. Select and prove one remote route at G7; no credential values
belong in URLs, generated artifacts, webviews or evidence. Fail visibly on an
unsupported protocol instead of switching dialect or starting a local brain.

Generated client artifacts must record the pinned commit, selected canonical
schemas, transitive references, generator input hashes and upstream MIT notice.
The private `@hermes/shared` workspace is not an installed public dependency.
Use a small protocol adapter; do not vendor the Electron application or hand-copy
its full connection-management system.

## AYLUL to Shadows Core

Keep `web/` on its current API. Sidecar is a **separate scoped caller**, not a way
for Hermes to acquire the web client's local-person privileges.

| Core operation | Current meaning | New assistant usage |
|---|---|---|
| `GET /api/subscribe`, `/api/projects/{id}/events` | Existing durable replay/live plus documented live signals | Consume committed notices after caller/sharing contract |
| `GET /api/threads/{id}/entries`, `/operations` | Existing conversation/operation reads | Read only through the agreed bounded projection; unrestricted routes are not a disclosure policy |
| `POST /api/threads/{id}/turns` | Existing local-person start, 202 operation receipt | Preserve for existing UI; remote-origin assistant writes need §24.6 caller identity |
| `POST /api/operations/{id}/stop`; queue/send-now routes | Existing local-person control | Same separate authority gate; never automatic from model text |
| Core `/mcp` | Existing project/thread grants and exposed tools | Not the read-only assessment bridge credential |
| `Projects::set_nylra_sharing`, `Projects::nylra_context` | Proposed T3 operations, not existing endpoints | Separate accepted Core contract before routes/schema generation |

## Next vertical slice

The [local-runtime/Hermes plan](../aylul/native/plans/2026-10-18-aylul-local-runtime-hermes.md)
first proves the backend-to-installed-Hermes route, then wires AYLUL's local core
to the authenticated backend facade with Core absent. Existing hook and
avatar checks stay intact. Semantic retrieval and scoped Core context follow as
their own tracks; raw code disclosure and external agent control are not implied
by successful chat transport.
