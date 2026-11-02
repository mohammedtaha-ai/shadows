# Companion panel, outfit and waiting permissions

- Status: accepted (by the controller on Mohammed's behalf, 2026-11-01)
- Asked by: masada, 2026-11-01
- Owner: native-lead
- Serves: docs/frontend/plans/2026-11-01-aylul-agents-panel-frontend.md (design
  docs/aylul/specs/2026-11-01-aylul-agents-and-panel-design.md §7.6–§7.8)

## Need

The companion UI draws the bound agent in its outfit, opens a panel when the character is
clicked (agents grid, bound agent's card, status pill), selects another agent from it, opens the
main window on a named page, and answers a waiting permission request of the bound agent's
conversation. The renderer makes no HTTP call, so native `Companion` must publish what the panel
shows in the companion state and carry each action to Core through its private client. Native
tasks N-22 to N-25 build it; UI tasks U-5 to U-8 consume it.

## Interface

| Call / message | Sends | Gets back |
|---|---|---|
| `companion-state` event and `companion_state` (existing) | | adds `appearance.outfit: string` (opaque, omitted when absent, as `color`) |
| same | | adds `panel: { open: bool, agents: [{ slug, name, description, color, outfit, bound: bool }] }` (≤ 64 agents; name ≤ 128, description ≤ 200 characters) |
| same | | adds `waiting: { count: u32, bound: [{ threadId, requestId, title, options: [{ optionId, name, kind }] }] }` (the bound agent's conversation only; ≤ 8 requests, ≤ 8 options each, title ≤ 240, name ≤ 80 characters; answered by a button only) |
| `companion_panel` (new, companion only) | `{ open: bool }` | `CompanionState` |
| `select_companion_agent` (existing, now companion as well as main) | `{ agent: string }` | `BindingView`, same rules as from main |
| `open_main_from_companion` (existing) | `{ target?: "bound" \| "agentSettings" \| "newAgent" \| "settings" \| "oldestWaiting" }`, `bound` when absent | `()` |
| `main-navigation` (existing) | | `MainDestination` adds `{ kind: "agentSettings", agent }`, `{ kind: "newAgent" }`, `{ kind: "settings" }` |
| `answer_companion_permission` (new, companion only) | `{ threadId, requestId, optionId, commandId }` | `{ ok: true }` or a fixed error code |

## Errors

| When | The asking side does |
|---|---|
| `answer_companion_permission` → `permission_settled` (Core 409 `PERMISSION_SETTLED`) / `permission_not_found` (404 `PERMISSION_NOT_FOUND`) / `permission_option_not_offered` (422) | Shows "Already answered" / "No longer waiting" / "That option is no longer offered"; the next state drops a settled request |
| `answer_companion_permission` → `companion_core_unavailable` | Keeps the command id; the button retries with it |
| `select_companion_agent` refused | Shows the fixed error line the main surface shows today |
| `open_main_from_companion` with a target that no longer resolves | Native falls back to `companionSettings`, as today |

## Answer

Accepted as asked by the controller on Mohammed's behalf, 2026-11-01 (no native-lead session
ran). HTTP names follow `docs/backend/plans/2026-11-01-aylul-agents-panel-backend.md` §3.

## Reply

Accepted (masada, 2026-11-01).

## Ruling
