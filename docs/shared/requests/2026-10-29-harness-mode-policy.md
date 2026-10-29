# Each harness lists the modes Shadows allows

- Status: asked
- Asked by: Masada, 2026-10-29
- Screen: docs/frontend/plans/2026-10-25-web-architecture.md (W-11 packet 3 `mode-policy`)

## Need

The project-modes editor, the new-conversation draft, the composer's mode line and
the refused-permission line need each harness's allowed modes, with a label and a
plain description, before any session is open. Today `web/src/app/mode-policy.ts`
copies Shadows' policy from `shadows-agent/src/policy.rs`: Claude Code and AYLUL
allow `acceptEdits` and `auto`, start in `acceptEdits`, and `auto` is the mode
that asks nothing. The harness list must carry that policy, so the web copy can go.

## Fields

| Call | Sends | Gets back |
|---|---|---|
| `GET /api/harnesses` | nothing | each `HarnessInfo` adds `modes: [{ id: string, label: string, description: string }]` in listed order, `initial_mode: string \| null` and `unattended_mode: string \| null`; a harness with no decided modes sends `[]`, `null`, `null` |

## Errors

| When | The screen shows |
|---|---|
| The harness list cannot be read | the existing harness-list error; no mode is offered |
| A mode the policy does not list appears in a thread | its id, as today |

## Backend answer

## Frontend reply

## Ruling
