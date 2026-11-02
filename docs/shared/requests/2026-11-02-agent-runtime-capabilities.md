# Runtime capabilities of an AYLUL agent

- Status: asked
- Asked by: Masada, 2026-11-02
- Screen: [docs/frontend/plans/2026-11-02-ai-studio-ui-migration.md](../../frontend/plans/2026-11-02-ai-studio-ui-migration.md), F-74 (AYLUL agents home, "Agent Runtime Capabilities")

## Need

The AYLUL agents home shows what the agents can do (the design names File System & Diffs,
Terminal Execution, Multi-Model Handoff, Subagent Delegation), each marked available or not.
Today that list is static text inside `Planned`; the web cannot claim a capability Core does not
report. The web needs Core's list for the runtime an agent runs on, and whether each is on. This
is not the agent's extensions (plugins, skills, MCP tools), which
[the agents panel request](2026-11-01-aylul-agents-panel.md) already serves.

## Fields

| Call | Sends | Gets back |
|---|---|---|
| `GET /api/aylul/agents` (or a new read) | as today | per agent, or once for the runtime, `capabilities: [{ id: string, label: string, available: boolean }]` in Core's order |

## Errors

| When | The screen shows |
|---|---|
| `capabilities` absent | the list stays inside `Planned` |
| an entry is `available: false` | it shows as unavailable, with its label |

## Backend answer

<Hamoud: as asked / with a change / not possible, and why. Name the service
and contract it touches.>

## Frontend reply

<Masada: accepted, or what must stay and why.>

## Ruling

<The teacher, only when disputed.>
