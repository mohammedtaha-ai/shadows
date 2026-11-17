# Themes an agent edits

Status: approved by Mohammed (2026-11-16); built after slice 6. Owner of the look, the scenes and how a theme
is drawn. Core's side (storage, checks, the MCP tool) is asked for in a shared request and is
decided in Hamoud's owner spec once he answers.

Idea: [vision §12](../../core/vision.md#12-clear-navigation-and-focused-work), line "Themes an agent
edits" in [future](../../core/future.md).

## 1. What the person gets

Mohammed says in any conversation "make it a deep teal night with slow waves", and the app
changes at once, without a reload. He can keep several themes, pick one in Settings, or turn
custom themes off and return to the built-in light, dark or system look.

## 2. Decided (Mohammed, 2026-11-16)

1. **Any conversation changes the theme, through MCP.** There is no dedicated appearance agent.
2. **Our own system, not Claude's.** The Claude desktop mods (`~/.claude/themes/`) are the model
   for the idea only. Their colour generator is Anthropic's generated code and their API runs only
   inside Claude's app; nothing of it is copied.
3. **The app's data, the app's folder.** Themes are kept by the local Core with the rest of the
   person's settings, in the product's data folder, like `.claude` and `.codex` keep theirs.
4. **The desktop app.** Its main window is the shared `web/` build, so the theme is drawn by
   `web/` code. The same Core serves the browser, which shows the same theme; nothing extra is
   built for it. The companion window and the character are not themed.
5. **Open colour keys.** The palette token list has one owner, `web/`. Core accepts any
   well-formed key and checks only what a colour must be, so a new token needs no change in Core
   (dynamic lists, not backend enums).
6. **Text stays readable over any scene.** Contrast is checked against the surface text really
   sits on: the background, the card and, when a scene is drawn, the glass tint (§4).

## 3. Where each part lives

| Part | Owner | Why |
|---|---|---|
| Themes, the active one, their checks | Core, the person service (`core-person`) | It already owns `theme` (`dark`/`light`/`system`); one owner for the person's look |
| `theme_*` MCP tools and HTTP routes | `shadows-mcp`, `shadows-http`, calling the same methods | An adapter translates; HTTP and MCP give the same answer |
| Telling open windows a theme changed | Core's event stream | The live update with no reload |
| Turning a theme into CSS, the scenes, the picker | `web/` (Masada) | Drawing only; it shows Core's answer |

No native (`src-tauri`) work is needed.

## 4. A theme

A theme is data, never code. Core stores it and checks every field; `web/` draws it.

| Field | Meaning | Core checks |
|---|---|---|
| `id` | Chosen by Core | — |
| `name` | Shown in the picker, such as "Night tide" | 1–40 characters, unique per person |
| `base` | `dark` or `light`: which built-in palette fills what the theme leaves out | one of the two |
| `colors` | Values for the palette tokens of `web/src/index.css`, by name without `--palette-` (today `background`, `sidebar`, `card`, `border`, `input`, `text`, `text-secondary`, `text-muted`, `accent`, `accent-line`, `accent-soft`, `accent-softer`, `accent-text`, `warning`, `danger`, `danger-border`, `danger-text`, `success`, `overlay`) | any key of lowercase letters, digits and `-`, up to 40 characters, at most 64 keys; each value `#rrggbb` or `#rrggbbaa`; `text`, `background` and `card` are required; `text` on `background` and on `card` at least 4.5:1 |
| `scene` | The animated background's id, such as `tide`, `stars`, `none` | any short id: the list grows in `web/`, so Core keeps it opaque, and `web/` draws `none` for an id it does not know |
| `scene_colors` | Up to 4 colours the scene uses | `#rrggbb` |
| `motion` | Speed of the scene, 0 (still) to 2 | the range |
| `glass` | Panels over the scene: `blur` 0–24, `tint` colour, `opacity` 0–1 | the ranges; when `scene` is not `none`: `opacity` at least 0.75 and `text` on `tint` at least 4.5:1 |

A colour the theme leaves out keeps the base palette's value, so an agent can give five colours
and get a whole theme. The list of tokens is the web's alone: a key `web/` does not know is kept
and ignored, and a new token needs no change in Core. Core cannot know the base palette's values,
so the three colours it measures (`text`, `background`, `card`) are required.

Over a scene, text sits on glass, not on the background. A glass at least 0.75 opaque lets at most
a quarter of the scene through, so the tint decides what text is read against; Core checks text
on the tint as well.

The person keeps any number of themes and one **active theme, or none**. None means today's
built-in `dark`/`light`/`system`, which stays as it is.

## 5. What an agent can do (MCP)

| Tool | Does |
|---|---|
| `theme_list` | The person's themes and which one is active |
| `theme_save` | Creates a theme, or replaces one by id; answers the theme or every refused field with its reason |
| `theme_use` | Makes a theme active, or none |
| `theme_delete` | Removes a theme; deleting the active one returns to none |

A refused field comes back as a plain reason the agent can fix and resend ("text on background is
3.1:1, needs 4.5:1"), not as a failure of the conversation. The tool descriptions name the scene
ids `web/` has today and say that an unknown id draws nothing.

The same four operations are HTTP routes, used by the Settings picker.

## 6. Drawing (web)

- The active theme is read through one hook in `web/src/api/`, and its `colors` are set as the
  `--palette-*` variables on the document root; `base` sets the `dark` class. With no active theme
  the person's `dark`/`light`/`system` choice applies as today (`PersonTheme`).
- A colour key `web/` does not know is ignored; the tool descriptions list today's keys.
- When Core's stream says the theme changed, the hook refetches and the page repaints: no reload.
- A scene is an SVG drawn once behind the app, with CSS animation: waves, stars and so on, written
  by us. `motion` scales the animation's duration; `0` stops it. The OS "reduce motion" setting
  always stops it.
- `glass` puts a blurred, tinted layer under panels so text stays readable over the scene.
- Settings › Appearance lists the themes with a small preview, a "Use" button and "None". Making
  or editing a theme is the agents' job; the picker has no colour editor.
- First scenes: `none`, `tide` (slow waves along the foot, a moon), `stars` (a slow star field).

## 7. Not now

- A colour editor in Settings.
- Themes for the companion window or the character.
- Sharing or importing themes as files.
- Scenes an agent writes as code.

## 8. Order of building

1. **Core:** themes as data with their checks, the active theme, HTTP routes, the change event.
   (Hamoud, from the shared request.)
2. **Web:** read the active theme and paint the palette, the Settings picker, the live repaint.
3. **Core:** the four MCP tools.
4. **Web:** the scenes, `motion`, `glass` and reduce-motion.

Steps 1 and 2 already give a theme Mohammed picks; step 3 lets any conversation change it; step 4
adds the animated background.

## 9. Acceptance

In the desktop app, a conversation asked for "a deep teal night with slow waves" saves and uses
a theme; the open window changes with no reload; a refused colour comes back with its reason and
the agent fixes it; "None" in Settings returns the built-in look; with reduce motion on, the waves
stand still.
