# Visual acceptance left open (F-128)

Date: 2026-11-18. Branch `feature/code-settings-servers-row` at `8c314b6d`, web served by
`vite` on `localhost:5173` against the daemon on `localhost:4318` (`shadows-dev` database),
in the desktop app's browser pane, 800 px wide. Observed by the controller session; Mohammed
confirms.

## The light palette

The light palette was applied in the page (the `dark` class removed and `data-theme="light"`
set, as `PersonTheme` does for the saved theme `light`), without changing the saved theme.

| Screen | Observed |
|---|---|
| Sidebar (projects, conversations, workflows) | Readable; the selected project and row keep their accent outline; status labels ("Approved v1") legible. |
| A conversation (`testing6`, Planner on Codex) | Messages, tool rows and the notice row readable; composer, mode pills and the model and effort menus legible. |
| A plan page (`Setup`) | Header, the `Planned` "Plan progress" block, the phase buttons and the canvas nodes readable; the canvas minimap is visible. |
| Project settings | Instructions, allowed modes, "Reads code from" and the danger zone readable; the danger zone keeps its red border. |

Defects: none seen.

## The image tiles (F-114)

Two PNG images (320 × 200) were pasted into the conversation's composer. Two tiles appeared
above the composer, each with its image and a remove button named for its file
("Remove 7c3aed.png"); both removed cleanly. The tiles look right in the light palette.

Not run: the tiles inside a sent message. Sending would start a real Planner turn in
Mohammed's conversation; left for his own run.

## The desktop login item

Not run: starting with Windows needs the AYLUL desktop app installed and a sign-out or
restart of Windows. It stays Mohammed's acceptance.
