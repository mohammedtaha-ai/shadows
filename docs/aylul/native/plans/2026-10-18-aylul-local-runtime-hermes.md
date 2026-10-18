# AYLUL local application core and backend connection plan

> **For agentic workers:** Use `superpowers:executing-plans` for approved inline
> execution. No worktrees or automatic delegation. Product files below are proposed.

**Goal:** AYLUL uses one shadows-backend origin for login and assistant conversation;
the backend owns the internal connection to Hermes.

**Architecture:** Rust `AppCore` composes Shadows only. Proposed `AylulAppCore`
composes the existing Node hook/session/approval owners, local context and backend
client. Tauri retains protected backend account state and local process/window
lifetime. Backend composition supplies the Hermes client; the desktop neither
dials Hermes nor carries its service key.

**Tech stack:** Existing React/Tauri, Node sidecar, backend TypeBox-generated
HTTP/SSE client and installed Hermes v0.22.2 native runs API. No new model loop.

**Authority:** [integration design](../../specs/2026-10-18-aylul-hermes-semantic-context-design.md),
[endpoint inventory](../../../integration/2026-10-18-runtime-endpoints.md),
[backend slice](../../../../../shadows-backend/docs/backend/specs/2026-10-18-hermes-backed-assistant-design.md).

## H-0 — Backend first

The backend owner proves Docker-to-host access without installing Hermes in Docker,
then implements the account-authorized assistant facade and generated client
contract. Follow its [plan](../../../../../shadows-backend/docs/backend/plans/2026-10-18-hermes-backed-assistant.md).
The current installed source, not the different F-drive fork, owns its upstream
API. Transport readiness is separate from actual model/session acceptance.

## H-1 — Compose the local application core with backend status

**Create when first caller exists:** `apps/desktop/sidecar/src/{app.ts,app.test.ts}`,
`sidecar/src/backend/{client.ts,client.test.ts,contract.yaml}`; generated backend
wire artifacts through the accepted backend request. `sidecar/` means
`apps/desktop/sidecar/` below.
**Modify:** `main.ts`, handlers/RPC composition/contract; native protected
account-to-sidecar handoff owner and allowlist; actual code map.

- [ ] Accept the [local boundary](../../shared/requests/2026-10-18-hermes-conversation-boundary.md)
  and generated backend contract before implementing their methods.
- [ ] RED: `backend_offline_keeps_hooks_available`, `no_hermes_credential_in_desktop`,
  `signout_invalidates_pending_remote_results`, `app_stop_leaves_remote_runtime_alive`.
- [ ] Create `AylulAppCore` only with the first backend-status consumer. Move
  existing construction, retaining Store/Approvals/hook owners and tests.
- [ ] Keep account credentials native; pass only the scoped backend credential
  required by the client over private stdio. The existing auth implementation
  currently does not perform this handoff; do not describe it as complete.
- [ ] Return local snapshots/prepared requests within the current 10-second native
  deadline. Network/SSE work remains background; stdout stays reply-by-request-ID.
- [ ] Run sidecar typecheck and focused app/backend/RPC tests, native sidecar tests,
  update contracts/map, and commit locally with exact paths.

## H-2 — One main-window backend conversation

**Create:** `src/platform/assistant.ts`,
`src/features/assistant/{AssistantPage.tsx,contract.yaml}` and focused tests.
`src/` means `apps/desktop/src/`. Extend existing platform/app/features contracts.

- [ ] RED: `assistant_uses_same_backend_origin_as_account`,
  `send_targets_owned_conversation`, `unknown_outcome_never_auto_resends`,
  `companion_cannot_send_or_approve`, `hidden_character_preserves_chat_and_monitoring`.
- [ ] Use account-scoped opaque conversation/turn IDs from the backend, not raw
  Hermes session/run selectors from the UI. Replay backend events and refresh a
  snapshot on a gap. Show prepared/accepted/running/terminal/unknown separately.
- [ ] Preserve the existing account, hook and character surfaces. Auth failure
  disables remote conversation, not local observation or personal approval UI.
- [ ] Verify frontend build, focused assistant/shell/ACL tests and production
  exclusion of the studio; native debug run uses synthetic account/test stores.
- [ ] Exercise one real backend-to-Hermes reply only after the backend's readiness,
  authority and isolated provider gates pass. Record fixture vs actual results.
- [ ] Commit the runnable slice. Existing Shadows workspace loading stays T7;
  there is no eager Core startup or mandatory LSP process for simple chat.

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
