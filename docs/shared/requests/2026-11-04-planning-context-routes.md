# Planning anchor and carried context

- Status: accepted
- Asked by: Claude for Mohammed, 2026-11-04. The screen comes later (Planner context slice 3);
  slice 1's acceptance reads these routes directly.
- Screen: `docs/backend/specs/2026-11-04-planner-context-design.md` §4.1 (the anchor) and §4.4
  (inspection); plan `docs/backend/plans/2026-11-04-planner-context-slice-1.md`, B-51.

## Need

From a part's or a stage's page, Mohammed anchors a Planner conversation there, and the
conversation's header shows the anchor and can change or clear it. For any Planner turn, he can
open what context it carried: the text, each piece's source and version, the sizes, what was
cut, and whether the session heard it.

## Fields

| Call | Sends | Gets back |
|---|---|---|
| `PUT /api/threads/{id}/anchor` | `{ command_id: string, anchor: { kind: "part" \| "outcome", id: string } \| null }` | `{ anchor: { kind, id } \| null }` |
| `GET /api/threads/{id}/anchor` | | `{ anchor: { kind, id } \| null }` |
| `GET /api/operations/{id}/context` | | `{ carried: { digest: string, text: string, sources: [{ kind: string, id: string, version: number }], index_tokens: number, detail_tokens: number, over_tokens: number, cuts: string[] }, heard: boolean }` |

## Errors

| When | The screen shows |
|---|---|
| The part or outcome is not in the conversation's project (404) | "That part is not in this project." |
| The conversation is an AYLUL agent's (422 `INVALID_COMMAND`) | The anchor control is not offered for AYLUL conversations. |
| The turn has no context record (404 `CONTEXT_NOT_CARRIED`) | "No planning context record is available for this turn." |
| A replay with the same `command_id` and other params (409 `COMMAND_CONFLICT`) | The usual conflict line. |

## Backend answer

Accepted with a neutral missing-record message. `Design` owns the anchor and `Harness`
owns the prepared context record and its heard state.

## Frontend reply

Accepted by Mohammed's instruction to adjust the reviewed details and implement the plan,
2026-11-04. The anchor replies use the `{ anchor }` envelope in this request.

## Ruling

<The teacher, only when disputed.>
