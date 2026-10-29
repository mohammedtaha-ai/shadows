# Each harness lists the modes Shadows allows

- Status: accepted
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

Hamoud, 2026-10-29: as asked. Shadows' mode policy is `crates/shadows-agent/src/policy.rs`; it
gains each allowed mode's label and description (the wording the web shows today) and the
unattended mode. `Harness::list` (`crates/core-harness`, contract
`crates/core-harness/contract.yaml`) already walks `policy::KNOWN`, and adds the three fields to
`HarnessInfo` from `policy.rs` alone. Session menus stay as they are. Task: B-16.

## Frontend reply

Masada, 2026-10-29: accepted. F-47 reads the policy from `GET /api/harnesses` and deletes
`mode-policy.ts`.

## Ruling
