# Context and memory layers of an AYLUL agent

- Status: asked
- Asked by: Masada, 2026-11-02
- Screen: [docs/frontend/plans/2026-11-02-ai-studio-ui-migration.md](../../frontend/plans/2026-11-02-ai-studio-ui-migration.md), F-74 (AYLUL agents home, "Context & Memory Layers")

## Need

The AYLUL agents home shows which layers of context an agent keeps (the design names Working
Task State, Session Conversation, Project Architecture, Scoped Environment), each marked kept or
not. Today the list is static text inside `Planned`. It is not the token figures of one turn
([context use per operation](2026-11-02-operation-context-use.md)); it is what the agent carries
from turn to turn. The web needs Core's list and whether each layer is on for the agent.

## Fields

| Call | Sends | Gets back |
|---|---|---|
| `GET /api/aylul/agents` (or a new read) | as today | per agent, `context_layers: [{ id: string, label: string, kept: boolean }]` in Core's order |

## Errors

| When | The screen shows |
|---|---|
| `context_layers` absent | the list stays inside `Planned` |
| a layer is `kept: false` | it shows as not kept, with its label |

## Backend answer

<Hamoud: as asked / with a change / not possible, and why. Name the service
and contract it touches.>

## Frontend reply

<Masada: accepted, or what must stay and why.>

## Ruling

<The teacher, only when disputed.>
