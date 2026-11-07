# Codex Planner through the existing conversation API

- Status: accepted
- Asked by: Controller for the Core web client, 2026-11-07
- Screen: Existing new/open conversation; owner spec §12.14 in
  `docs/core/specs/2026-09-24-harness-controls-design.md`.
- Basis: Mohammed chose direct app-server integration and a fixed harness per conversation.

## Need

Select Codex for a new project conversation and use the existing Planner controls,
streamed text, Shadows tools, refusals, context and account-limit display. The
frontend continues to call Core; Core owns availability, modes and permissions.

## Fields

| Existing call | Required behavior |
|---|---|
| `GET /api/harnesses` | Keep `kind: codex`; use existing availability/reason, modes, initial mode, remembered settings and limits fields. No model names are hard-coded in web. |
| Existing thread creation/harness selection | Send `codex`; preserve Core's first-operation harness lock and source-harness fork rule. |
| Existing session/model/effort/mode calls | Return the existing `SessionChoices` shape, including per-model efforts and actual current settings. |
| Existing turn/Stop/Send now calls | Preserve command IDs, terminal status semantics, busy/refusal errors and event order. |
| Existing SSE stream | Text, tool/refusal entries, choices and usage use the existing event vocabulary. History loaded on resume is not new output. |

## Errors

Missing executable configuration is an actionable `reason` on an unavailable
harness. Missing sign-in, process/setup failure, unusable models and unsupported
choices use existing Core failure surfaces with public-safe messages. No silent
switch to Claude. A project mode refusal comes from Core, not menu filtering.

## Backend answer

Accepted for the existing shapes. AppCore/Harness own routing and lifecycle;
Agent owns protocol translation. B-85 must settle effective modes before enabling
Codex. No new endpoint or public field is required by this agreement. Any later
capability field required to disable an unsupported visible action must be added
to this request and generated from source before its backend/frontend tasks run.

## Frontend reply

Accepted by the controller for the planned Core web work. F-97 waits for B-87's
implemented behavior. Use the server's available modes, reasons and capabilities;
do not add local permission rules. Keep the current conversation layout.
