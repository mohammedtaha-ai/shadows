# AYLUL local application core and backend connection plan

> **For agentic workers:** Use `superpowers:executing-plans` for approved inline
> execution. No worktrees or automatic delegation. The normal-chat slice is implemented;
> semantic context and local application-core composition remain later work.

**Goal:** AYLUL uses one shadows-backend origin for login and assistant conversation;
the backend owns the internal connection to Hermes.

**Architecture:** Rust `AppCore` composes Shadows only. Normal conversation reuses
Tauri's authenticated backend client and protected account store. The existing Node
hook/session/approval composition is preserved. A future `AylulAppCore` is deferred
until local context needs it. Backend composition supplies the Hermes client; the
desktop neither dials Hermes nor carries its service key.

**Tech stack:** Existing React/Tauri, Node sidecar, backend TypeBox-generated
HTTP snapshot client and installed Hermes v0.22.2 native runs API. No new model loop.

**Authority:** [integration design](../../specs/2026-10-18-aylul-hermes-semantic-context-design.md),
[endpoint inventory](../../../integration/2026-10-18-runtime-endpoints.md),
[backend slice](../../../../../shadows-backend/docs/backend/specs/2026-10-18-hermes-backed-assistant-design.md).

## H-0 — Backend first

The backend owner proves Docker-to-host access without installing Hermes in Docker,
then implements the account-authorized assistant facade and generated client
contract. Follow its [plan](../../../../../shadows-backend/docs/backend/plans/2026-10-18-hermes-backed-assistant.md).
The current installed source, not the different F-drive fork, owns its upstream
API. Transport readiness is separate from actual model/session acceptance.

The backend internal client and actual MiniMax-M3 session/recovery journey passed
in `8fbb068`. The public account facade and generated 0.5.0 contract are implemented
in `02aca92`. Its first client uses polling, with no public assistant SSE endpoint.

## H-1 — Compose the local application core with backend status

**Controller amendment, 2026-10-19:** the human authorized normal chat now and
deferred Core/MCP. Reuse native authenticated HTTP instead of handing a bearer to
Node or adding an unused local core. No separate lead approval is represented.

**Implemented files:** `apps/desktop/src-tauri/src/assistant/{mod.rs,contract.yaml}`,
the existing `auth/{http.rs,service.rs,mod.rs,policy_tests.rs,contract.yaml}`,
`commands/{mod.rs,companion.rs,contract.yaml}`, `lib.rs`, `build.rs`, native permission
source and regenerated ACL schemas. No sidecar or hook behavior changes.

- [x] Accept the [local boundary](../../shared/requests/2026-10-18-hermes-conversation-boundary.md).
- [x] Closed main-only `assistant_call`; fixed method/path mapping, strict fields,
  UUID validation and 4096-byte input bound. No arbitrary upstream selector.
- [x] Keep the account bearer in the protected native store; cap responses at
  128 KiB, use the existing HTTPS origin and refuse a reply after the handle changes.
- [x] Native mapping tests pass after RED, including the extra-field refusal fix.
- [x] Separately run real authenticated native HTTP against the local backend using
  a synthetic DPAPI store; status/list pass without a credential in their projection.
- [ ] Physical Tauri window acceptance: automatic execution review rejected both
  launch attempts with `blocked by policy`. A user-operated launcher is supplied;
  browser substitution and native HTTP tests do not close this gate.

## H-2 — One main-window backend conversation

**Create:** `src/platform/assistant.ts`,
`src/features/assistant/{AssistantPage.tsx,contract.yaml}` and focused tests.
`src/` means `apps/desktop/src/`. Extend existing platform/app/features contracts.

- [x] Consume generated backend types through `src/api/generated/` with input hash
  and `scripts/sync-backend-api.mjs`; no hand-written wire definitions.
- [x] Present account conversation selection/creation, bounded visible-panel
  polling, send/stop/explicit recovery and distinct run outcomes.
- [x] Retain one send command ID after a lost reply; reads never repeat a POST.
  Account/session changes invalidate responses and clear conversation/draft state.
- [x] Preserve account, hooks, approvals and independent character owners. Main-only
  ACL tests deny assistant IPC to companion; production excludes the studio.
- [x] Actual local backend + MiniMax two-message UI journey and reload pass with an
  explicit browser substitute for native IPC. Separately verify actual native HTTP.
- [ ] Physical native GUI acceptance remains open as recorded above. Source-level
  implementation and its checks are complete; no native-window success is claimed.

Existing Shadows workspace loading stays T7. There is no eager Core startup or
mandatory LSP process for simple chat. Dated results belong in
`docs/evidence/aylul/2026-10-19-normal-chat.md`.

## Following slices

T1 keeps local session/root/attention continuity. The independently packaged code
service remains its own application core; T2 consumes it locally. T3 adds Core-owned
sharing/projection; the backend/Hermes obtains bounded laptop evidence through a
separately authenticated bridge, never by guessing a laptop path on the server.
T5 explains actual interruptions; T6 sends the person's direction through the
proven owning CLI/Core adapter. No general command proxy is introduced by chat.

Keep native credentials, server service credentials, local approval evidence and
Core authority distinct even though the external product has one backend origin.
Local commits are part of task completion; publishing remains separate.
