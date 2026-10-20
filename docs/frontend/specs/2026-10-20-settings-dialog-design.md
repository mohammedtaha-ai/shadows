# Settings dialog

- Status: proposed with Mohammed on 2026-10-20 (mockups in the brainstorm session; not kept)
- Owner: Masada (frontend). Backend needs: [settings dialog request](../../shared/requests/2026-10-20-settings-dialog.md)
- Replaces: the single scrolling Settings page (`web/src/app/settings/settings-page.tsx` with
  `web/src/app/desktop/native-settings.tsx` under it)

## Why

The Settings page today is one long column. The Account section shows six buttons at once
(Sign in, Check session, Register as a new device, Sign out, Sign out all sessions, Revoke this
device) whatever the person's state, so nothing tells them which one applies. Providers, agents,
the character and Local Core are spread over separate pages and one scroll. Mohammed wants one
place, split into sections, that shows only what applies now.

## Goals

- Settings opens as a **modal dialog over the app, with a dimmed backdrop**, and a **sidebar of
  sections** on its left. The app stays visible behind it; closing returns to where the person was.
- Each section shows only the controls that apply to the current state.
- Everything about providers, agents, the character, Local Core, plugins and skills, and usage is
  reached from this dialog.

## Non-goals

- No backend change in this work. What the screens need from the daemon or the account server is
  written in the request above; those parts are built after it is accepted and implemented.
- No redesign of the conversation, the sidebar of conversations, or the companion window.

## Anti-goals

- A section that shows a control the person cannot use in their state (the six-button Account).
- An API key ever shown again after it is saved.
- A second dialog opened on top of the settings dialog. Sub-steps (adding a provider, editing an
  agent) replace the section's content, with a back arrow.

## The dialog

- Opened by the Settings button in the app sidebar (today a link to `/settings`).
- Centered, about 900 by 590 px on the desktop window, rounded, over a backdrop that dims and
  blocks the app. It closes with ✕, Escape, or a click on the backdrop. Focus is trapped inside
  while it is open and returns to the Settings button when it closes.
- Left sidebar, three groups, in this order:
  - **You:** Account, General
  - **AI:** Providers, Agents, Plugins & Skills, Usage
  - **This machine:** AYLUL character, Local Core
- The right side shows the chosen section with its title at the top.
- My call: the open section is kept in the URL (`?settings=<section>`), so reload and the native
  bootstrap's jump to Settings (`web/src/desktop/bootstrap.tsx`, which opens Local Core when the
  Core is unreachable) land on the right section. `/settings` keeps working and opens the dialog.
- Mohammed's ruling (2026-10-21): the **Providers button leaves the app sidebar**, and the
  separate Providers page (`/aylul/providers`) goes. Providers live only in this dialog; the AYLUL
  home's "add a provider" button opens the dialog on Providers.
- Mohammed's ruling (2026-10-21): an agent's **Agent settings** in the app sidebar opens this
  dialog on Agents, with that agent's form open. The separate agent pages (`/aylul/agents/...`)
  go. My call, following it: **+ New agent** in the app sidebar opens the dialog on Agents with
  an empty form.
- On the browser (not the desktop), sections that are desktop-only (AYLUL character, Local Core,
  and the account actions that need the native shell) are not listed, as today.

## Sections

### Account

The person's own data comes first, then the account state, then only the actions that state allows.

- **You:** their name, and **what AYLUL calls them** (the name AYLUL uses when it speaks to them).
  Both are kept on this machine and work without an account. *(needs backend)*
- **State and actions**, from today's account states (`account-panel.tsx`):

  | State | Shown | Actions |
  |---|---|---|
  | Not signed in, local only | "You are not signed in. Everything works on this machine." | Sign in |
  | Signed in | Account email/id, "Signed in" | Sign out; Sign out all sessions (confirm); Revoke this device (confirm, at the bottom, in red) |
  | Backend unavailable | "The account server is unavailable. Your device stays registered." | Check again |
  | Session not verified | "Your session could not be verified." | Sign in again; Check again |
  | Device revoked | "This device was revoked." | Register this device again |
  | Signing in | "Finish signing in in your browser." | Cancel sign-in |

  "Register as a new device" is not a separate button: it is what Sign in does when there is no
  registration, and the only action when the device was revoked.
- **Devices on your account** (signed in only): each device with its name, platform and last
  activity; "this device" is marked; each other device has **Revoke** (confirm). Not signed in, the
  card says "Sign in to see the devices linked to your account." *(needs backend)*
- "Check session" is not a button: the state is checked each time the section opens.
- Until the "You" and "Devices" endpoints exist, those two cards are not drawn; the state and its
  actions work today through the native `authAction`.

### General

- **Active projects** (moved here from today's page, unchanged behaviour).
- Language and theme, start AYLUL with Windows, closing the window hides it to the tray.
  *(needs backend / native: none of these is stored today)*. The Arabic interface itself (its
  translations and right-to-left layout) is its own piece of work; this section only offers the
  choice once it exists.

### Providers

- A list: each provider by **its provider name, then the person's label in parentheses**, for
  example "MiniMax (Work key)", with its number of models and a connected / not set tag, and Edit.
- **+ Add provider** replaces the list with two steps (back arrow returns):
  1. **Choose:** a search box over cards of known providers (MiniMax, Anthropic, DeepSeek,
     OpenRouter, Moonshot (Kimi)) plus **Custom**, which always stays visible. Search filters the
     cards by name.
  2. **Set up:** the provider is fixed (its name is the primary name and cannot be typed over).
     - **Label** (optional): tells two keys of the same provider apart; shown in parentheses
       everywhere, in Providers and in Usage. *(needs backend)*
     - **API key**, write-only, with **Test connection** beside it. *(needs backend)*
     - After a successful test, **the models the key can use are listed at once**, each with a
       checkbox; the person ticks the ones to add and marks one as default. A model missing from
       the list can still be **added by hand** by its id. *(listing needs backend)*
     - **URL is not shown** for a known provider: the preset supplies it. It is shown, and
       required, only for Custom.
     - The context window comes with each model instead of one number typed for the provider.
       *(needs backend)*
     - Save says how many models it adds ("Add 2 models").
- Edit opens the same set-up step for an existing provider; removing the key and deleting the
  provider keep today's confirmations (`delete-zone.tsx`).
- Until the backend parts exist, the set-up step keeps today's fields: name (prefilled from the
  preset), URL (prefilled, hidden for presets), key, models typed one per line, context window.

### Agents

- A list of agents: name, provider (with label) and default model, Edit and Delete.
  **+ New agent** and Edit replace the list with today's agent form (`agent-settings.tsx`) inside
  the dialog, including its environment variables (`agent-env.tsx`).
- **Character colour per agent:** each agent picks the colour of AYLUL's character when it is the
  companion agent: blue, red, green, purple, orange, pink (a row of swatches; purple is today's
  look). *(needs backend: the colour is stored with the agent)*

#### Character colour

**Where the character is drawn today** (studied 2026-10-21):

| Place | How it is drawn | Its colours |
|---|---|---|
| Companion window (`apps/desktop/src/app/companion.tsx` → `DesktopCompanion`) | Procedural Canvas 2D: `renderCharacter` in `apps/desktop/src/features/companion/renderer/` draws every frame from code; no image, sprite or video | Literal `rgba(...)`/hex strings in `character.ts`, `eyes.ts`, `crystals.ts`, `hands.ts`, `effects.ts`, `outfits.ts`, `scene.ts`: a violet/fuchsia family (`#a855f7`, `#e879f9`, `#d946ef`, `#c084fc`, `#d8b4fe`, `#9333ea`) and violet-tinted dark cloth (`#43325c` … `#08050d`); gold metal (`#fde047`, `#d97706`); the `blocked` state already swaps the accents to rose (`#f43f5e`), `success` adds gold |
| In-window presence (`Companion.tsx`) | The same renderer | The same; it is mounted only by the development studio and tests, not by a product window |
| Development studio (`studio/`) | The same renderer, plus PNG export | The same |
| Web sidebar (`web/src/app/sidebar/agent-list.tsx`) | **No character.** Each agent row shows the lucide `Bot` icon, tinted `text-accent-line` when it is the open agent | Theme tokens |
| `assets/aylul/` | No runtime asset: the historical PNGs are archived and the renderer does not load them | – |

No CSS variable reaches the drawing: Canvas takes colours as strings at draw time.

**The approach: a palette passed to the renderer.** The renderer gets one `CharacterColor` per frame
and reads every accent and cloth tint from a palette for it, instead of literals. Purple's palette
is today's literals exactly (same pixels). Each other palette is the same tone family in its hue
(its 500 tone is the glow and the swatch: blue `#3b82f6`, red `#ef4444`, green `#22c55e`, purple
`#a855f7`, orange `#f97316`, pink `#ec4899`), and the dark cloth tinted toward it. What does not
change with the palette: gold metal (stud, hoop, `special` outfit), white sparkles, outfit
accessories' own colours, and the state colours (`blocked` rose, `success` gold), since they are
signals.
- Why not the others: a hue filter (`filter: hue-rotate()` on the canvas, or `ctx.filter`) is free
  to build and cheap on the GPU, but it turns the gold metal and the state colours too (a blue
  AYLUL's `blocked` would stop being red) and shifts the dark cloth's brightness unevenly per hue.
  Recoloured assets do not apply: there are no assets. CSS tokens cannot reach a canvas.
- Cost: no asset work; one palette object per colour in the renderer; no draw call is added, so
  frame cost is unchanged. The work is a careful pass over the renderer's literals, owned by the
  character side, with a pixel test that purple is unchanged.

**The picker** (in the agent form, after Default model):
- A radio group named **Character colour**: six round swatches in the order above, each with its
  name as its accessible label and as a tooltip; the chosen one has a ring. A new agent starts on
  Purple. The colour is saved with the rest of the form by **Save agent**, like the other fields.
- Under it, one line says when the colour shows: "AYLUL wears this colour now." when this agent is
  the companion agent on this desktop; "AYLUL wears this colour when this agent is the companion
  agent." when it is not; in the browser, "AYLUL wears this colour on the desktop when this agent
  is the companion agent." The picker is the same in all three cases: the colour belongs to the
  agent, not to the companion.
- The Agents list shows each agent's colour as a small dot before its name, and a **Companion** tag
  on the companion agent (desktop only).
- The AYLUL character section shows, under the companion agent picker, "Character colour: <name>"
  with its dot, for the chosen agent.
- My call: **the preview** is a small still of the character in the chosen colour beside the
  swatches, from six PNGs the character studio exports (one per colour). The web client cannot run
  the desktop renderer (a separate package and owner), so a live preview would mean copying it.
  Until the six stills exist, no preview is drawn; the live companion is the preview once saved.

**How the colour reaches the companion window.** Core stores the colour on the agent; the web
client never sends it to the native shell. The native presence already reads the agent from
Core's agent list when the companion agent is selected and checks it every 10 seconds while bound;
it reads `character_color` there too and publishes it in the companion state it already sends the
window (`companion-state`), as `appearance.color`. No bound agent, an unavailable agent, a missing
field or an unknown value all draw purple. Changing the companion agent repaints at once (selection
already republishes the state). Saving the companion agent's form calls a main-only native
command that rereads the agent and republishes, so an edit shows at once rather than after the
next 10-second check.

**Who builds what:** the web picker, list dot, character-section line and the save-time refresh
call are this side's (F-9, F-14; the preview F-13); the companion state's parsing in
`apps/desktop/src/platform/` is the desktop UI's (U-1); the native read and command, and the
renderer palette with the six stills, belong to the native and character sides and are asked for
by AYLUL shared requests.

> **OPEN — red and the blocked state.** The `blocked` state turns the accents rose (`#f43f5e`); a
> red AYLUL's `blocked` then differs from its normal look only by its shake and warning mark.
> Closed by Mohammed's answer: keep red as is, use another state colour for a red character, or
> drop red from the six.

### Plugins & Skills

- Two tabs, Plugins and Skills. Each item has its name, a one-line description and an on/off
  switch; **+ Add** installs one. *(needs backend: nothing lists or toggles them today)*

### Usage

- A period picker (last 7 days, last 30 days, this month).
- **By provider:** one row per provider key, named "Provider (label)", with requests, tokens and
  cost, and a bar comparing them.
- **By model:** one row per model with its provider, requests, tokens and cost.
- *(needs backend: nothing records usage today)*

### AYLUL character

Today's controls, as switches instead of buttons: show AYLUL on the desktop, motion, short reply
preview. Below them the companion agent picker, its connection tag and **Retry connection**.

### Local Core

The Core's state first (address, running / not running). The executable, database and Node paths
of today's `connection-settings.tsx` move under a closed **Advanced** part, since they rarely change.

## What is left to the builder

- Component split, styling within the existing tokens, dialog primitive (the app already has a
  dialog for New project: `web/src/app/new-project/`; reuse its pattern).
- How each "needs backend" part shows before its endpoint exists: hidden, not a fake control.

## OPEN

> **OPEN — the preset list.** The known providers and their URLs are written from memory and not
> checked. Closed when the request's answer names the URLs the backend accepts for each preset.
