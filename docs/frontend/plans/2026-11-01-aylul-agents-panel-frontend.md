# AYLUL agents you can shape, and the panel: frontend plan

Branch `feature/aylul-agents-panel`. Serves the accepted design
[`docs/aylul/specs/2026-11-01-aylul-agents-and-panel-design.md`](../../aylul/specs/2026-11-01-aylul-agents-and-panel-design.md)
(Mohammed, 2026-11-01). That design owns every decision; this plan only cuts the client half
into tasks. The backend half is Hamoud's
(`docs/backend/plans/2026-11-01-aylul-agents-panel-backend.md`); where this plan needs an HTTP
shape the design does not fix, it is written under [Shapes we assume](#shapes-we-assume-from-the-backend)
so the two plans can be checked against each other.

The desktop IPC between the native side and the companion UI is asked in
[`docs/aylul/shared/requests/2026-11-01-companion-panel.md`](../../aylul/shared/requests/2026-11-01-companion-panel.md).

## Scope

| Part | Design | Tasks | Stages file |
|---|---|---|---|
| Agent form: outfit, description, instructions | §7.1, §7.3, §7.5 | F-62, F-63 | `docs/frontend/stages.md` |
| MCP items in the library, capabilities in the form | §7.2, §7.5 | F-64, F-65 | `docs/frontend/stages.md` |
| Permission card in the conversation | §7.8 "Web" | F-66 | `docs/frontend/stages.md` |
| Main-window destinations the panel opens | §7.6 | F-67 | `docs/frontend/stages.md` |
| Studio export of outfit previews, manifest test | §7.3 | C-8 | `docs/aylul/character/stages.md` |
| The character wears the bound agent's outfit | §7.7 | N-22, U-5, C-9 | native, ui, character |
| Panel surface, agents, selection, destinations | §7.6 | N-23, N-24, U-6, U-7 | native, ui |
| Waiting permissions on the panel: pill, card answer, `blocked` | §7.6, §7.8 | N-25, U-8 | native, ui |

Not here: everything under `crates/` and `api/` (Hamoud), our own library items (§7.4, Core
only), the bundled `CLAUDE.md` projection (§7.1, Core only).

## What the code does today

Read on 2026-11-01 at `578adda0`.

- `web/src/app/agents/agent-settings.tsx`: `AgentForm` renders `AgentFields` (one `<form>`:
  name, provider, default model, colour swatches; saves `AgentInput` through `saveAgent`), then,
  only for a saved agent, `AgentExtensions`, `AgentEnv` and `DeleteAgent`. After a save it calls
  `refreshCompanionAppearance()` when the agent is the bound one.
- `web/src/api/extensions.ts`: `ExtensionKind` is `plugin | skill`; `saveAgentExtensionAnswer`
  maps a kind to `plugins`/`skills` with a two-way ternary. `agent-extensions.tsx` draws one
  switch per item; `settings/extension-import.tsx` imports a folder path.
- `web/src/app/agents/character-colors.ts` imports `assets/aylul/character-colors.json` from the
  repository root and falls back to the base id: the precedent for the outfit manifest.
- `web/src/app/conversation/entry.tsx` (`Entry`) branches on `ThreadEntry.kind`;
  `PermissionRefused` is a one-line shield. `ThreadEntry.card` is an untyped object.
- `web/src/stream/refresh.ts` (`classifyRefresh`) maps durable events to query refreshes. The
  only streams are `/api/subscribe?thread_id=` and `/api/projects/{id}/events`.
- `web/src/desktop/navigation.ts` (`parseNavigation`) accepts `companionSettings`, `agentDraft`,
  `conversation`; `web/src/main.tsx` maps them to routes. Settings open with the search params
  `settings=<section>&agent=<slug|new>` (`useOpenSettings`).
- Native (`apps/desktop/src-tauri/src/`): `presence::CompanionAppearance { color }` comes from
  `core_client::observer::Client::validate_appearance`, which reads `GET /api/aylul/agents`
  (`{ project_id, agents }`) every 10 s while bound; `windows::companion::surface::SurfaceAppearance`
  serialises it. The observer subscribes to one thread (`/api/subscribe`). `navigation::MainDestination`
  has the three kinds above. `select_companion_agent` is callable by `main` only
  (`commands/companion.rs`, `allowed_caller`). The companion window is a 240 px logical surface
  that grows only for the editor (`windows/companion/input.rs`).
- Desktop TS: `platform/companion-state.ts` (`acceptState`) reads `appearance.color` only.
  `features/companion/DesktopCompanion.tsx` and `Companion.tsx` pass `outfit: 'default'`.
  A click on the character opens main (`desktop-gesture.ts`, `open: openMainFromCompanion`).
  `studio/export.ts` renders a 512 px PNG blob and downloads it in the browser; the opt-in
  `EXPORT_COLORS` test in `tests/companion/renderer.spec.ts` is the precedent for writing
  review images from Playwright.

## Order and parallelism

```text
no backend needed         C-8 ─► F-62 ─┐
                          F-67 ─► N-23 ─┼──────────────► U-6 ─► U-7 ─► U-8
                          N-22 ─► U-5 ─► C-9 ───────────────────┘       ▲
                          N-22 ─► N-24 ─────────────────► U-6           │
backend A (agent fields)  ──────► F-63 (after F-62) ─► F-65             │
backend B (MCP items)     ──────► F-64 ──────────────► F-65             │
backend C (permissions)   ──────► F-66                                  │
                          ──────► N-25 (after N-23, N-24) ──────────────┘
```

- **Run now, in parallel** (different files): C-8, F-67; N-22 too once the AYLUL request is
  accepted (every N- and U- task waits on that acceptance).
- **Then, in parallel:** F-62 (after C-8), N-23 (after F-67), U-5 (after N-22), N-24 (after N-22).
- C-9 after U-5. U-6 after N-23 and N-24. U-7 after U-6 and C-9.
- **Wait on backend shape A** (agent `description`, `instructions`, `outfit`): F-63. N-22 and
  N-24 read the new fields with a default and can be built before A lands; their real-app check waits on A.
- **Wait on backend shape B** (MCP items): F-64, then F-65 (also after F-63: both edit
  `agent-settings.tsx`).
- **Wait on backend shape C** (permissions): F-66 and N-25; U-8 after N-25 and U-7.
- F-63, F-64 and F-66 can run in parallel once their shapes land; each adds rows to
  `docs/frontend/codebase/README.md`, so commit them one at a time.
- The first web task after a backend shape lands runs `npm run gen:api` and commits
  `web/src/api/schema.d.ts`; a later task whose regeneration shows no diff commits nothing there.
- N-23 must land after F-67: N-23 rewrites the Rust-serialised navigation fixture that
  `web/src/desktop/native-parity.test.ts` parses, so the web parser has to accept the new
  kinds first.

## Shapes we assume from the backend

Names are proposals; Hamoud's plan wins on names, and this section is then corrected in place.
Every write carries `command_id` under the usual replay rules; every error is the usual error body.

### A. Agent identity (§7.1, §7.3)

- `Agent` gains `description: string` (`""` when none) and `outfit: string` (same pattern as
  `character_color`, `default` for new and migrated agents). **`instructions` is not in `Agent`**:
  the companion reads `GET /api/aylul/agents` every 10 s, and 32 KiB per agent does not belong there.
- `AgentInput` gains `description?: string | null`, `outfit?: string | null`,
  `instructions?: string | null`. Absent or `null` keeps the stored value, as `character_color`
  does today. `instructions: ""` means "use the bundled template"; `description: ""` clears it.
- `GET /api/aylul/agents/{slug}/instructions` → `AgentInstructions { instructions: string }`,
  the stored text (`""` when the template is used).
- `GET /api/aylul/agent-template` → `AgentInstructions { instructions: string }`, the bundled
  template, to prefill the form of a new agent.
- A description over 200 characters or with a line break, instructions over 32 KiB or with NUL,
  or an outfit id outside the pattern is a 422; the form shows Core's message.

### B. MCP items (§7.2)

- `ExtensionKind` gains `"mcp"`.
- `McpTransport = { kind: "http", url: string, headers: string[] } | { kind: "stdio", command: string, args: string[], env: string[] }`
  (`headers` and `env` are names only).
- `McpItem { id, name, description, transport: McpTransport, values_set: string[] }`:
  `values_set` names the header/env names whose secret is stored. No value is ever returned.
- `ExtensionLibrary` gains `mcp: McpItem[]`; `AgentExtensions` gains `mcp: AgentItem[]`.
- `POST /api/aylul/extensions/mcp`, body `McpInput { command_id, name, transport, values: { [name]: string } }`
  → `McpItem`.
- `PUT /api/aylul/extensions/mcp/{id}`, body `McpInput`; `values` holds only the names whose
  value changes, a name left out keeps its value, a name removed from the transport drops its value.
  The id stays, so every agent keeps its switch.
- The per-agent switch and the deletion reuse the existing routes with `kind = mcp`.
  `POST /api/aylul/extensions` (folder import) refuses `mcp`.
- A `stdio` item on a daemon that cannot run one (§8 OPEN) is refused with a 422 the form shows.

### C. Permissions (§7.8)

- `ThreadEntryKind` gains `PermissionAsked` and `PermissionAnswered`.
- `PermissionAsked.card` is `PermissionAskedCard { request_id, title, options: PermissionOption[] }`,
  `PermissionOption { id, label, kind }` with `kind` the adapter's (`allow_once`,
  `allow_always`, `reject_once`, `reject_always`), passed through.
- `PermissionAnswered.card` is `PermissionAnsweredCard { request_id, option_id, label }`;
  `author` is the human actor (the client shows "by you" from it).
- Both card schemas are named in `api/openapi.json`, so the client types them without reading
  an untyped object by hand.
- `GET /api/aylul/permissions` → `WaitingPermissions { waiting: WaitingPermission[] }`, oldest
  first, `WaitingPermission { project_id, thread_id, agent, request_id, title, options, asked_at }`.
  This list is the only authority on what still waits; neither client infers it from entries.
- `POST /api/threads/{id}/permissions/{request_id}`, body `{ command_id, option_id }` →
  `PermissionAnsweredCard`. 404 when the request no longer waits, 409 when it was already
  answered (a second surface lost the race), 422 when the option was not offered.
- A durable event `PermissionsChanged { thread_id }` is published on the thread's
  `/api/subscribe` and on the AYLUL project's `/api/projects/{project_id}/events`, where
  `project_id` is the one `GET /api/aylul/agents` returns. Both clients re-read the waiting list on it.

## Task notes

Each task's exact `Touch:` and `Done when:` are in its stages file; these notes say what the
task means.

### F-62

The outfit picker, from the previews C-8 commits under `web/src/assets/outfits/`
(`manifest.json`, `{ "base": "default", "outfits": [{ "id", "label" }] }`, and `<id>.png`).
`outfits.ts` reads the manifest and the images (`import.meta.glob`, eager URLs) and owns
`outfitOf(id)`, which falls back to the base for an id it does not know (the client owns the
look, design §5). `outfit-picker.tsx` is a radio group like `ColorSwatches`: one preview per
outfit, its label as the accessible name. Not yet wired into the form.

### F-63

Regenerate the API for shape A. The form follows design §7.5's order: outfit picker and
colour, name and description, provider and default model, instructions (a large text area;
for a new agent prefilled from `GET /api/aylul/agent-template`, for an existing one loaded from
`GET /api/aylul/agents/{slug}/instructions`), then the existing sections. The note "Changes
apply to new and resumed conversations." stays under the save button, and a line under the
instructions says the form is the place to edit them (a hand edit in the home is overwritten).
`instructions` is sent only when its text changed (absent keeps it), so an untouched 32 KiB
text is not resent on every save. The
sidebar shows the description under the agent's name. After a save of the bound agent,
`refreshCompanionAppearance()` already runs; it now also carries the outfit (N-22).

### F-64

Regenerate the API for shape B. Settings → Plugins & Skills gains an MCP tools list beside
plugins and skills, and an "Add MCP tool" form: name; transport `http` (URL, header rows) or
`stdio` (command, arguments, environment rows, with the note "Runs on this machine's daemon
only", §8); each header/env row is a name plus a password field. An item opens the same form to
edit: names show, values never do ("Set" when in `values_set`, an empty password field replaces
it). URL, name and value rules are Core's: the form shows Core's 422, it checks nothing itself.
Deleting reuses `ExtensionList`.

### F-65

The agent form's "Plugins & Skills" section becomes "Capabilities": skills, plugins and MCP
tools, each with its switch, and an "Add to library" button that opens Settings → Plugins & Skills
(MCP items are added there too). `saveAgentExtensionAnswer` maps all three kinds. A new agent
shows capabilities after it is created, as today (switches need its slug).

### F-66

Regenerate the API for shape C. `web/src/api/permissions.ts` owns the waiting query and the
answer call. In the conversation, a `PermissionAsked` entry is a card with the title and the
adapter's options as buttons, enabled only while the waiting list holds its `request_id`; a
click answers with a command id and keeps the attempt for a retry. A `PermissionAnswered` entry
shows the chosen label and "by you". A 409 or 404 refreshes the list and shows Core's message.
`refresh.ts` refreshes the waiting list on `PermissionsChanged` and on any `Permission*` entry.

### F-67

`parseNavigation` and the router map accept three more destinations, ready before native sends
them: `agentSettings { agent }` → `/aylul?settings=agents&agent=<slug>`, `newAgent` →
`/aylul?settings=agents&agent=new`, `settings` → `/aylul?settings=general`. The parity test keeps
refusing anything else.

### C-8

The studio gains `exportOutfitPreview(outfit)` in `studio/export.ts`: the `idle`, `front`,
base-colour frame of one outfit through the existing `exportCharacterPNG` path, at 160 px. An
opt-in Playwright spec (`EXPORT_OUTFITS=1`, like `EXPORT_COLORS`) calls it for every entry of
`OUTFITS` and writes `web/src/assets/outfits/<id>.png` and `manifest.json`; the images are
committed. A node test (`tests/companion/outfit-manifest.test.ts`, added to
`test:companion:unit` so CI runs it) fails when the manifest's ids and labels differ from
`OUTFITS` in content or order, or when an image is missing.

### N-22

`CompanionAppearance` and `SurfaceAppearance` gain `outfit`, copied opaque from the bound agent
in `/api/aylul/agents` exactly as `color` is (absent field → absent; no validation, no native
list). The surface wire pin and the appearance tests cover it; the presence fixture is rewritten.

### U-5

`acceptState` reads `appearance.outfit` as an opaque id of 1..=32 characters into
`CompanionState.outfit`, defaulting to `default` when absent or malformed, as it does for colour.

### C-9

`DesktopCompanion.tsx` passes `state.outfit` to the motion input, and `Companion.tsx` takes an
`outfit` prop; an id the renderer does not know draws `default` (one check against `OUTFITS`,
in the companion). A binding change swaps the outfit with the existing motion.

### N-23

`MainDestination` gains `agentSettings { agent }`, `newAgent` and `settings`.
`open_main_from_companion` takes an optional `target` (`bound` when absent: today's behaviour;
`agentSettings`, `newAgent`, `settings`); `Presence` resolves and validates it with reads only,
never creating a thread, and an invalid one falls back to `companionSettings` as today.

### N-24

The panel surface. `CompanionWindow` gains a panel mode beside the editor: open fits the whole
panel rectangle on the character's monitor (never larger than the work area), keeps the
character's anchor, takes focus; close, blur or Escape restores the compact bounds and the
prior foreground, as the editor does. `Companion` publishes, in the companion state, the live
agents from the same metadata read (`slug`, `name`, `description`, `color`, `outfit`, `bound`;
at most 64, names and descriptions bounded). `select_companion_agent` becomes callable by the
companion surface too, through the same `Presence` method, so the binding rules, the
generation, and the "first message creates the thread" rule stay exactly as they are.

### N-25

Waiting permissions. The private Core client reads `GET /api/aylul/permissions` and follows
`PermissionsChanged` on the AYLUL project's event stream (a second, read-only feed owned by the
same watcher, with the existing deadlines and caps). `Companion` publishes the waiting count and
the requests of the bound agent's conversation (title, options; bounded). A request waiting on
the bound thread makes the projection's expression `blocked` with status "Waiting for you".
`answer_companion_permission { threadId, requestId, optionId, commandId }` posts the answer
through `core_client` and returns Core's outcome; `open_main_from_companion` gains the target
`oldestWaiting`, which resolves to the first entry's conversation.

### U-6

`platform/companion-state.ts` parses the panel part of the state (open flag, agents), and
`platform/companion.ts` gains `setCompanionPanel(open)`, `selectAgentFromCompanion(agent)` and
`openMainFromCompanion(target?)`.

### U-7

The panel (`features/companion/panel/`), shaped by §7.6 in the existing companion style. A click
on the character opens it instead of the main window (keyboard Enter/Space likewise); Escape or
a click outside closes it. Top bar: Home, Chat (opens the composer), New agent, the status pill
(shows "Ready" until U-8), Settings, Sound (turns the existing interaction tones of `audio.ts`
on or off for the session). Card: the bound agent drawn live by the renderer in its colour and
outfit, its name, description, state (the presentation's status), and Talk, Settings, Open
(Open with no conversation opens the agent's page: native resolves `bound` to `agentDraft`).
Grid: every live agent as a small live character in its outfit and colour with its name, the
bound one marked, "New agent" last, scrolling past eight. Clicking a tile selects that agent.
The panel holds no list, form or setting beyond this (anti-goal §4).

### U-8

The pill shows the waiting count ("Ready" at zero); clicking it opens main on the oldest waiting
conversation. When the bound agent's conversation has a request waiting, the card shows its title
and the options as buttons; a click answers it with a fresh command id, a retry reuses it, and
Core's refusal (already answered, ended) shows as a short line. The character's `blocked` comes
from native (N-25), never from the panel.

## Points raised on the design

Reported to the controller on 2026-11-01; none is changed here.

1. **The panel's folder.** §7.6 puts the panel in `features/companion/`, which
   `docs/aylul/character/AGENTS.md` gives to the character side (only the colour was ruled
   UI-owned). U-7 and U-8 wait for the teacher's ruling.
2. **"The studio writes into `web/`'s assets."** The studio is a browser page and can only
   download. C-8 writes the files from an opt-in Playwright run that calls the studio's export,
   as `EXPORT_COLORS` does.
3. **Previews are untinted.** One image per outfit (§7.3) shows the base colour; the web form
   shows the chosen colour on the swatches, not on the preview.
4. **"Without polling" across every AYLUL thread.** The companion follows one thread today; the
   pill needs a second, project-wide feed (N-25) and the `PermissionsChanged` event of shape C.
5. **Prefilled template.** A new agent saved with the prefilled template stores it as its own
   text, so a later template change does not reach it. Core could store `""` when the text equals
   the template; that is Hamoud's call, the form sends what the box holds.
6. **Capabilities of a new agent** appear after it is created: a switch needs the agent's slug,
   unless Core accepts switches in the create body.
7. **Sound** is session-only: the native companion has no tone preference to keep it.

## Acceptance

Design §10 items 1–9 are checked by a person in the real desktop app with logs read (§10.10),
after every task above and Hamoud's tasks are done. Visual tasks (F-62, F-63, F-66, U-7, U-8)
attach a screenshot of the result to their report; Masada compares it against design §7.5/§7.6.
