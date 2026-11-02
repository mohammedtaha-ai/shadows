# AYLUL agents you can shape, and the panel

- Status: accepted (by the controller on Mohammed's behalf, 2026-11-01)
- Asked by: Masada, 2026-11-01
- Screen: docs/frontend/plans/2026-11-01-aylul-agents-panel-frontend.md (design
  docs/aylul/specs/2026-11-01-aylul-agents-and-panel-design.md §7.1–§7.8)

## Need

Mohammed shapes an AYLUL agent in its form (outfit, description, instructions prefilled from the
bundled template, MCP tools with secret values, capabilities switches), and answers an AYLUL
agent's permission request by a button, in the conversation or on the desktop panel, whose pill
counts what waits. The web client and the desktop companion's private Core client must read and
write all of it through the HTTP API, and follow waiting requests without polling.

## Fields

The shapes are exactly those of `docs/backend/plans/2026-11-01-aylul-agents-panel-backend.md`
§3 (at `e4001500`): §3.1 agents, §3.2 MCP tools, §3.3 permission requests. They are not copied
here, so there is one owner. The calls the frontend uses:

| Call | Sends | Gets back |
|---|---|---|
| `GET /api/aylul/agents` | | `AgentWorkspace`; `Agent` gains `description`, `instructions`, `outfit` |
| `POST /api/aylul/agents`, `PUT /api/aylul/agents/{slug}` | `AgentInput` with optional `description`, `instructions`, `outfit` | `Agent` |
| `GET /api/aylul/agent-template` | | `{ instructions }` |
| `GET /api/aylul/extensions`, `GET /api/aylul/agents/{slug}/extensions` | | gain `mcp`; `Item` gains `bundled` |
| `POST /api/aylul/extensions/mcp`, `PUT /api/aylul/extensions/mcp/{id}` | `{ command_id, name, description?, transport }` | `McpItem` |
| `PUT /api/aylul/agents/{slug}/extensions/mcp/{id}`, `DELETE /api/aylul/extensions/mcp/{id}` | as for plugins and skills | as for plugins and skills |
| `GET /api/aylul/permissions` | | `WaitingPermission[]`, oldest first |
| `POST /api/threads/{id}/session/permissions/{request_id}` | `{ command_id, option_id }` | `{ thread_id, request_id, option_id, kind, answered_at }` |
| `GET /api/projects/{project_id}/events` (AYLUL project) | | durable `PermissionsChanged`, payload `{ request_id, state }` |

## Errors

| When | The screen shows |
|---|---|
| 400 `PROFILE_INVALID`, `SECRET_NAME_INVALID`, `SECRET_VALUE_INVALID` | Core's message under the form |
| 409 `SECRET_NAME_RESERVED`, `EXTENSION_NAME_CONFLICT` | Core's message under the form |
| 503 `SECRETS_UNAVAILABLE` | Core's message; the typed values stay in the form for a retry |
| 404 `PERMISSION_NOT_FOUND`, 409 `PERMISSION_SETTLED` | "No longer waiting" / "Already answered"; the waiting list is refetched |
| 422 `PERMISSION_OPTION_NOT_OFFERED` | Core's message; the list is refetched |

## Backend answer

As planned in `docs/backend/plans/2026-11-01-aylul-agents-panel-backend.md` §3 (Hamoud,
2026-11-01).

## Frontend reply

Accepted (Masada, 2026-11-01). Accepted for both sides by the controller on Mohammed's behalf,
2026-11-01. Approvals are buttons only (design §7.8).

## Ruling
