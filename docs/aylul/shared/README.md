# Shared requests

Where the sides of this repository agree on what crosses between them, before
either writes code: a Tauri command the UI calls, a sidecar message, a hook the
companion needs from the UI. A request to the cloud API goes instead to
`../shadows-backend/docs/shared/requests/`, answered by Hamoud.

## The sides

| Side | Lead | Rules |
|---|---|---|
| Character (AYLUL) | `aylul-lead` | [docs/aylul/character/AGENTS.md](../character/AGENTS.md) |
| Desktop UI | `masada` | [docs/aylul/ui/AGENTS.md](../ui/AGENTS.md) |
| Native (Tauri, sidecar, hook) | `native-lead` | [docs/aylul/native/AGENTS.md](../native/AGENTS.md) |

Mobile has no lead until `apps/mobile` exists.

## The cycle

1. The **asking lead** writes `requests/<yyyy-mm-dd>-<topic>.md` from
   [`requests/TEMPLATE.md`](requests/TEMPLATE.md). Status: `asked`.
2. They tell the **owning lead** (a message to its session).
3. The owning lead answers in "Answer": as asked, with a change, or not
   possible, and why. Status: `answered`.
4. The asking lead accepts, or replies under it. Status: `accepted` or `disputed`.
5. A `disputed` request goes to the **teacher**, which writes the "Ruling".
   Its ruling is final. Status: `accepted`.
6. Only an `accepted` request may be built. The owner builds first; the asker's
   dependent tasks stay `blocked` until then.

A message between sessions is only a notice. What counts is written here.
