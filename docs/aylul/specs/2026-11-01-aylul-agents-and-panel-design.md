# AYLUL agents you can shape, and a panel to reach them

Date: 2026-11-01. Status: **design for review. Nothing implemented.** Decided in
conversation with Mohammed on 2026-10-31 and 2026-11-01, branch
`feature/aylul-agents-panel`.

This file owns two things: what an AYLUL agent can be given when it is created
or edited, and the panel that opens when Mohammed clicks AYLUL on the desktop.
It builds on, and does not replace:

- [the brain design](2026-10-19-aylul-claude-code-brain-design.md) §11: providers, agents, homes.
- [settings backend](../../backend/specs/2026-10-21-settings-dialog-backend-design.md) §4 (character colour) and §7 (the extension library).
- [desktop architecture](../native/specs/2026-10-26-desktop-architecture-design.md): native owners and boundaries.
- [character in window](../character/specs/2026-10-16-aylul-in-window-design.md): outfits and expressions.

Where this file and one of those disagree on what they own, they are right and
this file is amended.

## 1. Intent and why

Today Mohammed can create an AYLUL agent and pick its name, provider, model,
colour, environment variables and library items. He cannot say **who the agent
is or what it is for**: its instructions come from one bundled template
(`crates/core-agents/assets/aylul-profile/CLAUDE.md`) and are never edited from
the UI. He cannot give it **MCP tools**. And every agent looks the same on the
desktop: the outfit is fixed to `default` in
`apps/desktop/src/features/companion/Companion.tsx` and `DesktopCompanion.tsx`.

He compared this with Hermes, where a bot is created in one dialog (name, face,
description, persona, skills, provider, model) and each lives in its own
profile. He wants the same ease, built on what Shadows already has.

The second half is how he reaches his agents. Clicking AYLUL today opens the
main window. He wants a small panel first, shaped like Coucou's island: the
agents as a grid, the chosen one as a card, a status pill on top, and a way into
the full app for everything else. Coucou is the reference for the shape only;
its character and artwork are not ours to use (its `LICENSE-ASSETS.md`).

## 2. Goals

1. An agent has **instructions** (who it is, how it works) and a short
   **description** (what it is for), both edited in its form.
2. An agent has **MCP tools**, chosen from a shared list, like its skills and
   plugins.
3. An agent has an **outfit**, chosen in its form from the character's outfits.
4. The **library** of skills and plugins can ship **our own items** from the
   repository, beside the ones Mohammed imports.
5. Clicking AYLUL opens the **panel**: agent grid, chosen agent's card, status
   pill, and buttons into the app.
6. The character on the desktop **wears the outfit of the agent it is bound to**.
7. An AYLUL agent **asks Mohammed** before a tool its mode does not allow, and
   waits for his answer, in the conversation and on the panel, instead of
   being refused at once.

## 3. Non-goals for this branch

- Watching Claude Code, Codex or Claude Desktop sessions, and answering them
  from AYLUL. That is the messaging design; its facts are in
  [the messaging probe](../../evidence/2026-11-01-agent-messaging-probe.md).
- Voice.
- Hermes skills and Hermes plugins. Hermes plugins are Python inside Hermes and
  do not run in Claude Code; its skills are not ours.
- Installing from a marketplace, Git or a URL. See the OPEN block in §8.
- Writing the content of our own skills. This branch builds the way they ship;
  which skills we write is its own task.
- Agents messaging each other.

## 4. Anti-goals (a failure even if it works)

- An agent's instructions, tools or library items leak into another agent's
  home, or into Mohammed's `C:\Users\Mohammed\.claude`.
- A secret of an MCP server (a token, a key) appears in a response, a log, a
  file the renderer can read, or the database in plaintext.
- Editing an agent restarts or breaks a conversation that is running.
- The panel becomes a second main window: lists that grow, forms, settings.
  Anything bigger than a card belongs in the app.
- A Coucou image, its Mochi character or its sounds appear anywhere.
- The character shows a state it was not told: the panel never implies a
  microphone, a running turn or an approval that did not happen.

## 5. Constraints (measured or already decided)

- **Isolation holds per home.** Each agent home is its own `CLAUDE_CONFIG_DIR`
  opened with `settingSources: ["user"]` (brain design §5).
- **Library items reach only enabled agents.** One library, a switch per agent,
  projected into the home at each opening; a switched-off item leaves at the
  next opening (settings backend §7.1, proved by
  [the extensions probe](../../evidence/2026-10-21-agent-home-extensions-probe.md)).
- **Edits apply at the next opening.** A live adapter keeps what it started
  with (brain design §10). The form says so.
- **MCP already reaches a session through ACP.** `core-harness/src/setup.rs`
  builds one `McpServerSpec` (HTTP, with headers) into the ACP session setup
  (`shadows-agent/src/acp/session.rs`, field `mcp: Option<McpServerSpec>`):
  Shadows' own `/mcp` for the Planner. AYLUL agents receive none today.
- **Records stay machine-independent** (settings backend §8.2): ids are
  strings, answers carry no absolute path.
- **Colours are opaque ids; clients own palettes** (settings backend §4). The
  same holds for outfits.
- **Desktop boundaries:** commands translate only; `CompanionWindow` holds no
  conversation state; the renderer makes no HTTP call; main screens stay in
  `web/` (desktop architecture).
- **The outfits that exist:** `default`, `crown`, `moon`, `scarf`, `starry`,
  `winter`, `headphones`, `mask`, `halo`, `special`
  (`apps/desktop/src/features/companion/catalog.ts`).

## 6. Decided by Mohammed

- Agents are shaped when created: instructions, purpose, skills, tools, outfit.
- The library holds **our own** skills, and items can be added later from a
  marketplace. Hermes skills are not used.
- A different agent looks like **the same AYLUL in a different outfit**, not
  another character.
- Clicking AYLUL opens a panel shaped like Coucou's island, in our own style,
  with the agents as its grid. The app opens from it for everything else.
- Monitoring and replying to outside sessions leave this branch for the
  messaging design.
- **The approval path is built in this branch** (2026-11-01), not deferred:
  AYLUL agents ask, and Mohammed answers.

## 7. My calls (Claude), for review

### 7.1 Instructions and description

- **Owner: `Agents`.** They are part of the agent's identity, like its name.
- `description`: optional, trimmed, at most 200 characters, one line. Shown on
  the panel card and under the agent's name in the sidebar.
- `instructions`: optional text, at most 32 KiB, no NUL. Empty means "use the
  bundled template".
- **Stored in the database, not only in the home.** That keeps the agent
  machine-independent (§5) and lets the web client read and write it through
  the API without file access.
- **At opening, the harness writes the home's `CLAUDE.md`** from the stored
  instructions, or from the bundled template when they are empty, in the same
  projection step that places library items. A `CLAUDE.md` changed by hand in
  the home is overwritten at the next opening; the form says the form is the
  place to edit. Existing agents keep their current `CLAUDE.md` as their
  instructions: the migration reads each live agent's home file once into
  `instructions`.
- The person line of settings backend §5.2 stays a system-prompt append and is
  unaffected.

### 7.2 MCP tools

- **Owner: `Extensions`,** whose job is "the shared extension library". An MCP
  server becomes a third `ExtensionKind`, `Mcp`, beside `Plugin` and `Skill`,
  with the same rules: defined once in the library, switched on per agent,
  deleting it removes every switch.
- An MCP item is **configuration, not files:** a name, and either
  - `http`: a URL (`https://`, or `http://` for localhost only, as providers)
    and header names, or
  - `stdio`: a command, its arguments and environment variable names.
- **Secret values go to `Secrets`,** under the scope `mcp/<item id>`, entered
  in password fields and never shown again, exactly as provider keys.
  `/` cannot appear in an agent slug, so scopes do not collide (brain design §11).
  - Every header value and every environment value of an MCP item is a
    secret; the item's configuration keeps only their names.
  - A secret holds the whole value as sent (`Bearer x`, not `x`).
  - Names in an `mcp/` scope follow their own rule inside `Secrets`: 1..=128
    characters of `A-Z`, `a-z`, `0-9`, `-`, `_`, so header names such as
    `Authorization` and `X-Api-Key` are accepted. The reserved environment
    names of brain design §10–§11 stay refused in every scope.
- **An MCP item can be edited** (URL, command, arguments, names, values)
  without losing any agent's switch: its id stays. Plugins and skills stay
  immutable; only `Mcp` items are configuration.
- **`Extensions` depends on `Secrets`** (a new edge, listed in its
  `Cargo.toml` and contract): saving an item stores its values through
  `Secrets::put_in`, and deleting it removes the item, every switch and its
  `mcp/<id>` scope in one write.
- **Delivery: through ACP, at opening.** The session setup carries a list of
  MCP servers instead of one optional server: the Planner's grant for Planner
  turns, the agent's switched-on items for AYLUL turns. Nothing is written into
  the home's `.claude.json`, so a removed tool is gone at the next opening.
- An AYLUL agent never receives Shadows' own `/mcp` grant (brain design §8).

### 7.3 Outfit

- **Owner: `Agents`,** beside `character_color`, with the same rule: an opaque
  id of 1..=32 characters `a-z`, `0-9`, `-`, starting with a letter. New and
  migrated agents are `default`. Absent on update keeps the stored value.
- The renderer draws `default` for an id it does not know. Adding an outfit
  needs no Core change.
- **Previews are images exported by the studio.** The renderer and
  `catalog.ts` stay in `apps/desktop`; `web/` imports nothing from there and
  also runs in a plain browser. The studio (`features/companion/studio/export.ts`)
  writes one image per outfit and a manifest of ids into `web/`'s assets, and
  the web form shows those. A test fails when the manifest and `OUTFITS`
  differ, so the two lists cannot drift. The desktop panel draws live with
  the renderer.

### 7.4 Our own library items

- The repository holds `crates/core-extensions/assets/library/`, one folder
  per item in the same layout an import accepts.
- At start, `Extensions` imports each bundled item whose name is not in the
  library yet, under a command id derived from the item's name and digest, so
  a restart replays instead of duplicating. A bundled item Mohammed deleted is
  not brought back: deletion records the bundled name as declined.
- A bundled item starts switched off for every agent.
- This branch ships the folder with **one** small item that proves the path.
  The real skills are their own task (§3).

### 7.5 The agent form

The form that exists (`web/src/app/agents/agent-settings.tsx`) grows into the
Hermes-like dialog. In order:

1. Outfit picker (previews) and colour.
2. Name, description.
3. Provider, default model (as today).
4. Instructions: a large text area, prefilled with the bundled template for a
   new agent.
5. Capabilities: skills, plugins and MCP tools from the library, each with a
   switch; "Add to library" opens the import already in Settings. An MCP item
   is added from the same place.
6. Environment variables (as today).

The note "Applies to new conversations" stays under the save button.

### 7.6 The panel

- **It is the companion's own surface.** Clicking the character opens the
  panel beside it; clicking outside or pressing Escape closes it. It lives in
  `features/companion/` and is controlled by `CompanionWindow`; no new native
  owner. Opening the main window is a `Navigation` destination, as today.
- **Top bar:** Home (the panel), Chat (the composer that exists), New agent
  (opens the main window on the new-agent form), the status pill, Settings
  (main window, Settings), and Sound (the existing interaction tones on or off).
- **Card (left):** the bound agent drawn in its outfit, its name, description
  and state (ready, thinking, working, blocked), and three actions: Talk (opens
  the composer), Settings (main window on that agent's settings), Open (main
  window on the bound conversation).
- **Grid (right):** every live agent, each a small character in its outfit and
  colour with its name; the bound one is marked. Clicking another agent
  **selects** it, under the binding rules that exist: the selection changes,
  the target clears, and the conversation is created by the first message, as
  today. No thread is created by a click, and the presence contract and its
  tests stay as they are. The last tile is "New agent".
- **Open** with no conversation yet opens the main window on that agent's
  page.
- **Status pill:** the number of AYLUL permission requests waiting on
  Mohammed (§7.8). Zero shows "Ready". Clicking it opens the main window on the
  oldest waiting conversation. Outside sessions join this count with the
  messaging design, not here.
- **The card answers its own agent:** when the selected agent's conversation
  has a request waiting, the card shows what is asked and its options as
  buttons, and a click answers it (§7.8).
- **Data:** the panel reads the agent list and binding through `Companion`'s
  private Core client, which already fetches agents for their colour (settings
  backend §4.3). The renderer stays free of HTTP.
- More than eight agents scroll inside the grid; the panel never grows past
  one screen.

### 7.7 The character wears the outfit

`Companion` publishes the bound agent's `outfit` in the same appearance record
that carries `color` today. `Companion.tsx` and `DesktopCompanion.tsx` read it
instead of the fixed `'default'`. Changing the binding changes the outfit with
the existing motion; no new animation is required.

### 7.8 Asking before a tool runs

Today Shadows answers every ACP `session/request_permission` with the reject
option at once and writes a `PermissionRefused` entry
(`shadows-agent/src/acp/connection.rs`; harness controls §12.2 owns that rule).
For AYLUL threads this branch replaces it with a question to Mohammed. The
Planner keeps today's refusal; harness controls §12.2 is amended to say so.

- **Owner of the waiting request: `Harness`,** because it is live session
  state: the ACP request is held on the session's connection until it is
  answered. `Harness` is the only way to see or answer one.
- **The thread records it** through `Turns`, as two entry kinds:
  `PermissionAsked` (request id, the tool's title, its options as the adapter
  sent them) when it arrives, and `PermissionAnswered` (the option chosen, and
  who answered: agent `human`, principal the local person, as approval
  authority §24.1 records every approval) when it is answered. The existing
  `PermissionRefused` stays for the Planner and for a request that ends
  unanswered.
- **Answering:** one method, `Harness::answer_permission(thread, request id,
  option id, command id)`, under the usual replay rules. One HTTP route calls
  it. An option id that is not one the adapter offered is refused. A second
  answer to the same request is a conflict, so the conversation and the panel
  cannot both answer it.
- **The options are the adapter's,** shown with their own labels (allow once,
  allow always, reject). Shadows remembers nothing itself: an "always" is the
  adapter's to keep for its session.
- **No timeout.** A request waits until it is answered, the turn is cancelled,
  the session closes or the daemon stops; each of the last three ends it as
  `PermissionRefused` with the reason, and the turn goes on as the adapter
  decides.
- **Waiting requests are listed** by `Harness::waiting_permissions()` across
  every AYLUL thread, oldest first, and each change is published on Core's
  event stream, so the panel's pill and card follow without polling.
- **Web:** the conversation shows a waiting request as a card in the turn,
  with the options as buttons; once answered it shows the choice and "by you".
- While a request waits, the character's state is `blocked` for that agent.

## 8. Open

> **OPEN: marketplace sources.** Mohammed wants to add items from a
> marketplace later. Settings backend §7.1 limits the first version to a local
> folder. Closes when he names the first marketplace (Claude Code's plugin
> marketplaces, or one of our own) and a probe shows an isolated home can
> install from it with `settingSources: ["user"]`.

> **OPEN: MCP over `stdio` on the server daemon.** Settings backend §8.2 says
> the daemon also runs beside the server. A `stdio` MCP item names a command on
> one machine. Closes when the server daemon's first AYLUL agent is designed;
> until then `stdio` items work on the local daemon only, and the form says so.

## 9. Left to the builder

- The exact migration order for `instructions`, `description` and `outfit`.
- How the ACP setup's MCP list is typed, as long as the Planner's grant
  behaves as today and is held by its existing tests.
- The panel's exact sizes, spacing and transitions, within the existing
  companion style.
- Whether the outfit previews are drawn live or cached as images at start.

## 10. Acceptance

1. Mohammed creates agent "Researcher" with description "Finds and checks
   sources", his own instructions, outfit `headphones`, one bundled skill and
   one HTTP MCP tool with a token. A new conversation answers in the voice the
   instructions set and lists the tool and the skill among its commands.
2. A second agent, "ay", created before this branch, still answers with the
   instructions it had, now visible and editable in its form.
3. Turning the tool off and opening a new conversation removes it; the running
   conversation is unaffected until it ends.
4. The token appears in no response, log, home file or plaintext database byte.
5. Clicking AYLUL on the desktop opens the panel: both agents in the grid in
   their outfits, the bound one marked, the card showing its description.
   Clicking "Researcher" re-binds; the character on the desktop now wears
   headphones.
6. In a "Researcher" conversation in a mode that asks, a command such as
   `curl` stops and asks. The conversation shows the request with its
   options; the character is `blocked`; the pill shows 1. Answering "allow
   once" on the panel card runs the command, the conversation shows "by you",
   and the pill returns to "Ready". A second request, rejected from the
   conversation, ends as rejected and the turn continues.
7. Cancelling a turn while a request waits ends it as refused, and the pill
   drops it.
8. Settings, New agent and Open each bring up the main window on the right
   page.
9. Mohammed's own Claude sessions and the Shadows Planner behave as before: the
   Planner still receives Shadows' `/mcp` and no AYLUL tool, and its permission
   requests are still refused at once.
10. Run by a person in the real desktop app, logs read, before it is called done.
