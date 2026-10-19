# Product daemon: TypeScript Hermes service

Date: 2026-10-19.
Status: Implemented locally on 2026-10-19 ([evidence](../evidence/aylul/2026-10-19-product-daemon-hermes.md);
the plan was deleted later that day, Git `226d0a3`).

2026-10-19 amendment: the daemon stays AYLUL's host, but AYLUL's brain is now
Claude Code run by Shadows core inside this daemon, owned by the
[brain design](../aylul/specs/2026-10-19-aylul-claude-code-brain-design.md).
The Hermes service and TypeScript client below stay in the code, unused by
default; the sections about them describe existing code, not the direction.

## 1. Approved intent

The human selected a product daemon that hosts several capabilities. Shadows
planning is one capability; AYLUL is an independently available assistant. The
same daemon program can run locally or on a server. A local assistant must not
require the account backend or a running Planner. Keep Hermes integration in
TypeScript: this is an extraction and lifecycle change, not a Rust rewrite.

This file owns the product host and the Hermes extraction boundary. The
[backend assistant design](../../../shadows-backend/docs/backend/specs/2026-10-18-hermes-backed-assistant-design.md)
owns the currently implemented account API and its PostgreSQL authority. Existing
source contracts remain descriptions of existing code until implementation.

## 2. Inspected starting point

- `crates/shadows/src/cli/mod.rs` starts one planning `AppCore`. Its
  `cli/args.rs` requires the Node, ACP adapter and Claude executable paths.
  Consequently the current daemon is not yet an assistant-only product host.
- `apps/desktop/src-tauri/src/lib.rs` owns native window composition and starts
  the Node hook sidecar. App exit currently stops that child.
- Native `src-tauri/src/assistant/mod.rs` maps closed operations to the
  account backend. The Hermes HTTP connection is not implemented in Tauri.
- Backend `src/integrations/hermes/{client,target}.ts` own the reusable native
  HTTP client: capabilities, session creation/lookup, run submission/lookup
  and stop. No account storage is owned by these two files.
- Backend `src/modules/assistant/service.ts` also owns account checks and
  durable conversation/command mappings through PostgreSQL. Copying only the
  HTTP client does not supply local conversation continuity or safe recovery.
- Hermes and the Node hook sidecar retain separate state and responsibilities.
  No new model orchestration loop or language conversion is required.

The inspected source used maps and contracts followed by named files. Shadows
MCP navigation and Rust LSP tools were unavailable in this session. These are
static observations, not new runtime acceptance results.

## 3. Selected structure

```text
Tauri desktop (windows, tray, protected remote login)
    -> selected product daemon
         |-> existing Shadows AppCore (optional planning capability)
         |-> Node assistant service -> existing Hermes native HTTP API
         |-> existing local observation sidecar (later lifecycle transfer)
         `-> independent code intelligence (later integration)
```

One daemon is the lifecycle owner and application entry, not a shared database
or one struct containing every domain. Keep the existing `AppCore` name and
planning ownership. Tauri remains the native shell; no Tauri dependency enters
the server daemon. The supervisor uses `shadows-process` for owned child trees,
not a second direct `tokio::process` implementation.

Source placement for implementation, not claims of existing modules:

| Location | One job |
| --- | --- |
| `packages/hermes-client/` | Portable TypeScript native Hermes transport |
| `services/assistant/` | Node assistant application with local persistence |
| `crates/shadows/src/` | Product process composition |
| `apps/desktop/src-tauri/src/` | Native desktop boundary |

The first package has no database, account, UI or Node child-process owner.
The assistant service is separate from the hook sidecar: losing a model
connection must not discard pending local agent approvals. Keep existing hook
ownership until an independently verified lifecycle task transfers it.

## 4. Extract rather than duplicate

Move the transport and its focused HTTP-fixture tests into `hermes-client` with
Git provenance recorded. Retain TypeScript and the native REST protocol. Do not
switch to ACP or Hermes desktop JSON-RPC in this migration.

Both the new assistant service and the existing backend consume the same package.
The backend consumes a versioned local `npm pack` artifact with lockfile integrity,
not imports into a sibling checkout. Keep the artifact in a backend-owned vendor
location so Docker/CI work without the Shadows checkout. Its source, version,
hash and reproduction command are recorded; it is not another editable source
copy. No registry publication is required. Remove superseded transport source
only when the backend consumes that tested artifact.

Retain target validation, explicit network modes, redirect refusal, request and
response bounds, fixed error categories, session/run identity checks and no
automatic write retries. Operator references resolve credentials at runtime;
neither UI nor daemon route payloads accept arbitrary upstream URLs or keys.

## 5. Local assistant authority and persistence

The new Node service owns a separate local assistant SQLite store using Node's
existing SQLite facility. It never reads Core SQLite, hook-sidecar tables or
Hermes SQLite. Hermes remains the conversation/model/tool execution owner.

The local store holds conversation-to-Hermes-session bindings, create command
IDs, normalized send fingerprints, durable turn intents and bounded projections.
Persist an intent before network dispatch. Keep the same upstream command key
after a lost reply; conflicting input refuses. Reads never resubmit a write.
Recovery is explicit and respects the verified upstream idempotency durability
and retention. A stale prepared intent becomes uncertain; it is not a fresh send.
Run lookup verifies the addressed session. A stop acknowledgement means stopping,
not cancelled. Restart retains these identities even without the desktop open.

Reuse backend behavior and regression cases where applicable, while keeping
account authorization and PostgreSQL tables in their existing backend owner.
Do not invent a synthetic cloud account for a local user. Existing backend
history remains associated with its account; local history is a separate source.
No automatic migration, merge or deletion of existing conversations occurs.

## 6. Process and connection boundary

The daemon starts Node with explicit executable/entry paths and bounded stdio
requests. Stdout is protocol-only; logs go to stderr without prompts or secrets.
Reuse a maintained framing library where suitable; do not build a generic RPC
framework. Expose only status/list/create/snapshot/send/stop/recover operations.

Planning configuration is validated only when planning is enabled. Preserve the
existing planning command behavior; add an explicit assistant-only launch mode
without fake Claude paths or eagerly starting an unused `AppCore`.

Desktop-to-local-daemon access uses a protected installation capability and a
main-window native client. Bind the local listener to loopback, authenticate
every assistant operation, reject browser-origin access and enforce input bounds.
Do not reuse a Planner MCP grant or pass a cloud account bearer to local Node.
No unauthenticated localhost execution endpoint or generic Hermes proxy is added.
New wire schemas have one executable owner and generated consumers; fixtures
must not become a second hand-written API definition.

Closing a desktop window or exiting the desktop disconnects its viewer. An
explicit daemon shutdown bounds in-flight calls, closes the owned Node process
and records uncertain outstanding writes. It does not terminate an independently
installed Hermes process. A crashed Node child is unavailable until explicitly
restarted; do not automatically replay its last command.

## 7. Local versus remote routing

Local mode routes through the daemon and requires no cloud login. Remote account
mode retains the current backend path until a separately tested backend-to-daemon
adapter is delivered. Never automatically fail over a send between the two.

Selection, cached history, pending commands and late responses are scoped to
the selected source plus account/installation and conversation. Switching source
cannot reuse a conversation ID, draft or result from the prior source. Native
protected cloud sessions remain owned by the existing login implementation.

The same daemon binary can later run on the server behind our backend. Server
deployment does not expose the loopback pilot endpoint or grant remote callers
local installation authority. Backend account/device authorization must remain in
front of any remote assistant operation. Access to laptop files/tools still needs
the laptop connection; a server cwd is not a laptop project path.

## 8. Delivery and acceptance

Implement in independently committed, verified slices after written-design and
implementation-plan review. Work inline in the current checkout; no worktrees.

1. Extract the TypeScript client with package tests, package provenance and a
   backend Docker regression proving the packed artifact replaces its source.
2. Implement Node local conversation persistence with restart, command conflict,
   lost admission reply, explicit recovery and bounded snapshot tests.
3. Supervise Node from the daemon; prove assistant-only startup, authenticated
   access, bounded child failure/shutdown and unchanged Planner startup.
4. Route the native client and UI to the explicitly selected local or remote
   source; prove source isolation, signed-out local chat and retained remote auth.
5. Prove the composed daemon/Node path against an owned native HTTP fixture and
   restart it. Then perform an isolated installed-Hermes journey under the
   existing operator setup. Record actual provider usage separately; no personal
   hook/config/credential migration or production deployment is implied.

The first completion gate is one usable local conversation through the daemon,
including reopen/recovery, without Docker account services or Claude startup.
Package extraction alone is not a completed move. Browser substitutions are not
physical desktop acceptance. Keep existing backend data and APIs working.

Bot CRUD/settings parity, model selection UI, new character states, code-context
sharing, external agent steering, computer control, public event streaming and
hook-sidecar lifetime transfer are separate deliveries. This migration supplies
their host boundary without implementing them ahead of their first caller.
