# In-window companion shell embedding

- Status: accepted
- Asked by: Character, 2026-10-16
- Owner: Desktop UI
- Serves: docs/aylul/character/plans/2026-10-16-aylul-in-window-implementation.md, Tasks 5–6

## Need

Replace only the existing unavailable AYLUL section with the Canvas companion.
Keep account state, platform wrappers, tray behavior and native permissions as
they are. Add an explicitly opened development studio, excluded from production.

## Interface

| Call / message | Sends | Gets back |
| --- | --- | --- |
| `Companion({presentation?})` from `src/features/companion/Companion` | Optional controlled presentation; shell supplies none in this slice | Idle/default/no message presence with local fallback |
| Development-only dynamic import of `studio/CompanionStudio` | Explicit Open/Close studio action; Vite DEV guard | Isolated preview; synthetic state never reaches main presence or account state |
| Existing `authAction` / `hideToTray` wrappers | Unchanged existing actions | Existing native projections/errors |

## Errors

| When | The asking side does |
| --- | --- |
| Canvas unavailable or render error | Show companion fallback while account and tray controls remain usable |
| Studio import/render fails | Show local studio error without replacing the shell |
| Audio or PNG export fails | Show local feedback, no false success or account effects |

## Answer

Accepted through the teacher's delegated decision on 2026-10-16 after the user
was presented this concrete agreement and instructed the implementer to choose
the most suitable approach. Preserve the narrow scope above.

## Reply

Character accepts the specified interface and independent failure boundaries.

## Native acceptance driver adjustment

Task 6 updates only `apps/desktop/tests/windows-shell.ps1` to focus the UI
Automation button named `Hide to tray` before pressing Space. Added companion
buttons invalidate its previous first-Tab-stop assumption. Actual native hide
must still be observed; do not invoke a native callback or weaken assertions.

## Ruling

The user delegated selection of the suitable approach. The teacher accepts this
in-window embedding and named-button test adjustment; no native/cloud scope added.
