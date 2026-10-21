# Settings dialog: character colour and the General switches

- Status: accepted
- Asked by: masada (Desktop UI and web), 2026-10-21
- Owner: Native scope
- Serves: docs/frontend/specs/2026-10-20-settings-dialog-design.md (Character colour; General),
  docs/aylul/ui/plans/2026-10-21-character-colour.md (U-1, U-3), docs/frontend/stages.md (F-12, F-14)

## Need

The settings dialog lets each agent pick AYLUL's character colour (Core stores it on the agent as
`character_color`, an opaque string, default `"purple"`), and its General section offers two
desktop-only switches. The native side must (1) pass the companion agent's colour to the
companion window, unread, (2) republish it at once when the main window saves that agent, and
(3) own the two switches: start AYLUL with Windows, and closing the main window hides it to the
tray instead of quitting.

## Interface

| Call / event | Sends | Gets back |
|---|---|---|
| `companion-state` (existing event, native to companion) | adds `appearance: { color: string }`, the bound agent's `character_color` copied unread; omitted when no agent is bound or it is unavailable | — |
| `refresh_companion_appearance` | nothing; main only | `{ ok: true }` once the bound agent was reread from Core and the state republished (no bound agent: republishes as is) |
| `desktop_preferences` | nothing; main only | `{ launchAtLogin: boolean, closeToTray: boolean }` |
| `set_desktop_preferences` | `{ launchAtLogin?: boolean, closeToTray?: boolean }`; main only | the full `{ launchAtLogin, closeToTray }` after saving |

- The colour is read where the presence already reads the agent (on selection and on the
  10-second check while bound); native keeps no list of colours and never validates the id.
- Defaults on a fresh install: `launchAtLogin: false`, `closeToTray: true` (today closing the
  main window already hides it: `windows/mod.rs` prevents the close and hides).
  With `closeToTray` on, closing the main window hides it (today's behaviour); with it off, closing
  the main window quits the desktop. The tray's Quit always quits. Both are stored by native on this machine, not in Core.

## Errors

| When | The asking side does |
|---|---|
| `appearance` missing, or `color` not in the table | The renderer draws purple |
| `refresh_companion_appearance` fails (Core unreachable) | The save still succeeds; the colour shows on the next 10-second check; no error is shown |
| `set_desktop_preferences` fails (the OS refuses the login item) | The switch returns to the value native reports, and the section shows "Could not change this setting." |
| The commands are called from the companion window | Refused by native; the UI never calls them there |

## Answer

With one change, by the teacher for the native scope (no native lead is running), 2026-10-21:
`closeToTray` defaults to **true**, because closing the main window already hides it today
(the asked default `false` would have changed today's behaviour). Everything else as asked:

- `appearance.color`: the presence check in `presence/client.rs` already reads
  `/api/aylul/agents` for the bound agent; it also reads `character_color` (absent stays absent,
  never validated), and the companion state carries `appearance: { color }` only while the
  agent is bound and available.
- `refresh_companion_appearance`: main only; reruns that check once and republishes.
- `desktop_preferences` / `set_desktop_preferences`: main only. `launchAtLogin` uses
  `tauri-plugin-autostart` (infrastructure: the OS login item); `closeToTray` is stored by native
  in the desktop's app data and read by the main window's close handler.
- Main only means the companion window's capability does not list them.

## Reply

Accepted, 2026-10-21 (the teacher, for the asking side): the `closeToTray: true` default keeps
today's behaviour, which is what General must show first.

## Ruling

<The teacher, only when disputed.>
