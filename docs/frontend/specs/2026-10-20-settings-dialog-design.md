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
