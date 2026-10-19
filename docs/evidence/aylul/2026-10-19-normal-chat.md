# Normal conversation through the account backend

Recorded 2026-10-19, Asia/Aden. Implementation and paid browser/native HTTP
observations began on 2026-10-18. This is evidence, not design authority.

## Scope

The human requested usable conversation in the existing desktop, deferring MCP,
Shadows context and code intelligence. Backend `02aca92` implements the account
facade and canonical generated contract 0.5.0. This desktop change consumes it
through native protected authenticated HTTP. Existing sidecar, hook, local
approvals and character ownership remain unchanged. No model key enters Node,
the renderer, evidence or generated source.

## Actual model and account path

The actual installed Hermes v0.22.2 used its configured `minimax` / `MiniMax-M3`
provider with a separate `.local-runtime/hermes-chat` home. Its agent reported
zero enabled toolsets; personal memory/context and background review were disabled.
The backend reached it from Docker through the host route and its separate service
key file. The public client used the ordinary account facade, not raw Hermes IDs.

`tests/assistant-acceptance.mjs` rendered the real `AssistantPage` in Chrome. An
explicit test-only native IPC substitute forwarded requests to the actual HTTPS
backend using an owned synthetic Keycloak account. This makes two real model calls:

1. Send `Please reply with exactly UI_CHAT_OK.` and observe `UI_CHAT_OK`.
2. Reload the page; the first reply returns with no second send.
3. Ask `What exact marker did you reply with before? Reply only with it.` in the
   same conversation; observe a second `UI_CHAT_OK`.

Observed result:

```json
{"liveBackend":true,"liveMiniMax":true,"browserNativeBoundary":"substitute","messages":2,"reloadRecovered":true,"duplicateSubmissions":0}
```

The [screenshot](2026-10-19-normal-chat/conversation.png) was visually inspected.
It contains synthetic text only. The browser substitute is excluded from the
production graph. It does not prove physical Tauri behavior.

Separately, the opt-in native Rust test
`assistant_local_backend_uses_protected_account_without_exposing_handle` passed
against the real backend using a temporary DPAPI-protected synthetic store.
It verified real native status/list calls and credential-free output. The
ordinary native suite intentionally ignores this and the separate Compose login
journey. Neither native test is a physical window or sign-in-browser claim.

## Local checks

- Frontend TypeScript/Vite production build passed (54 modules).
- Main Node tests passed: 7/7, including assistant IPC main-only ACL assertions.
- Native formatting passed. Native tests passed: 31 passed, 2 opt-in ignored.
- Native Clippy with all targets and warnings denied passed.
- Final native debug executable build passed after the frontend rebuild.
- Seven affected YAML contracts parsed. Generated TypeScript bytes match the
  backend artifact and recorded SHA-256 provenance.
- The deterministic `tests/assistant-state.mjs` browser run passed: unknown reads
  never POST, explicit recovery, addressed stop, sign-out clears history and account
  switch clears the unsent draft. Backend and native IPC are substitutes here.
- Production graph checks verify no assistant operation in companion, no test
  fixture in shipped bundles and no development studio in production.

The first old shell-browser run timed out inside `page.goto` before its first
assertion (6 other tests passed). The final complete retry passed 7/7 in 13.7 seconds.
No product change or increased timeout was made to hide that failure.

Backend Docker typecheck, canonical auth/NYLRA schema checks (12/12), documentation
links and secret checks passed. Four focused assistant tests passed before the
implementation commit. The broader suite was started once; captured output
included a failure in the existing
`enrollment_fresh_process_and_parallel_independent_reauthentication` test.
Final totals/diagnostic tail were lost during interruption. That full suite is
**not recorded as passing**; focused checks are not substituted for its result.
After resuming the local Docker runtime, the entire focused enrollment file passed
3/3 in 15.4 seconds with no source modification. This does not recover the missing
full-suite totals or prove a root cause for its earlier failure.

## Remaining acceptance and startup

Automatic execution review rejected actual desktop launch twice, including a
normal launch without a debug listener. Both returned `blocked by policy` without
a more specific reason. No alternate execution route was used to bypass that
rejection. Physical account sign-in and conversation in the Tauri window remain
open. The person can use `start-chat.cmd` after starting the documented backend
and installed-Hermes chat runtime; the launcher is not executed by this evidence.

See [desktop startup](../../../apps/desktop/README.md#normal-account-chat) and the
backend host handoff. The launcher opens only the desktop. Closing it leaves the
backend and Hermes alive. This does not certify production, CI, server deployment,
multi-user isolation, full history/deletion, public assistant SSE, provider-side
cancellation billing, Core/MCP, arbitrary CLI control or code disclosure.

Shadows MCP index and Rust LSP navigation were unavailable in this session;
inspection used the actual AYLUL map/contracts and their named source files.
