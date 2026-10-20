# Settings dialog Implementation Plan

> **For agentic workers:** each task is an `F-<n>` in [`../stages.md`](../stages.md); a
> `frontend-builder` builds one task, then one `frontend-fixer` reviews the branch. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Settings becomes one modal dialog with a sidebar of sections, each showing only what
applies; Providers and agent settings move into it.

**Architecture:** A `SettingsDialog` mounted once in the app shell reads its open section from the
URL search (`?settings=<section>&agent=<slug|new>`), so every door (the sidebar's Settings, an
agent's Agent settings, + New agent, the AYLUL home, the native bootstrap) only navigates. Each
section is its own component under `web/src/app/settings/sections/`. Tasks F-2 to F-6 use today's
API; F-7 to F-12 wait on the [settings dialog request](../../shared/requests/2026-10-20-settings-dialog.md).

**Tech Stack:** React 19, TanStack Router (hash history) and Query, the app's `@/components/ui/dialog`,
Vitest with happy-dom and the fake daemon (`web/src/app/test-app.tsx`, `@/test/fake-daemon`).

**Spec:** [`../specs/2026-10-20-settings-dialog-design.md`](../specs/2026-10-20-settings-dialog-design.md)

## Global Constraints

- Frontend only: no file under `crates/`, `api/` or `apps/desktop/src-tauri/` changes.
- Section ids, in sidebar order: `account`, `general` (group "You"); `providers`, `agents`,
  `plugins`, `usage` (group "AI"); `character`, `core` (group "This machine").
- Desktop-only sections (`account`, `character`, `core`) are listed only when `isDesktop()`.
- A section or card that needs the backend is **not drawn** until its endpoint exists: no fake
  control. `plugins` and `usage` are not listed until F-10 and F-11.
- No second dialog on top of the settings dialog: sub-steps replace the section body and show a
  back arrow (`←`, `aria-label="Back"`).
- An API key is never shown after it is saved (today's write-only behaviour stays).
- Every HTTP call goes through `web/src/api/`; each changed folder updates its `contract.yaml`
  and each new or removed file its row in `docs/frontend/codebase/README.md` in the same commit.
- No new dependency.

## Review Focus

1. Escape or a backdrop click while an add-provider or agent form has typed values: the dialog
   closes and nothing is saved; reopening shows the list, not a half-filled form. (F-5, F-6)
2. Reload with `?settings=agents&agent=<deleted slug>`: the Agents list shows with
   "This agent no longer exists.", not a blank pane. (F-6)
3. `?settings=character` opened in the browser build: falls back to `general`. (F-2)
4. Account while an action is running: every action button is disabled, only Cancel sign-in is
   enabled during sign-in. (F-3)
5. Search in Add provider that matches nothing: only Custom shows, with "No provider matches." (F-5)

---

### Task F-2: Dialog shell, sidebar of sections, General

**Files:**
- Create: `web/src/app/settings/settings-dialog.tsx`, `web/src/app/settings/sections.ts`,
  `web/src/app/settings/use-settings.ts`, `web/src/app/settings/sections/general.tsx`
- Modify: `web/src/router.tsx` (root `validateSearch`; `/settings` redirects), `web/src/app/shell.tsx`
  (mount `SettingsDialog` once, beside `NewProjectProvider`), `web/src/app/sidebar/sidebar.tsx` (Settings
  becomes a button), `web/src/desktop/bootstrap.tsx:28` (hash `/?settings=core`)
- Delete: `web/src/app/settings/settings-page.tsx`
- Test: `web/src/app/settings/settings-dialog.test.tsx` (replaces `settings-page.test.tsx`; its
  active-limit tests move here unchanged except for how the field is reached)

**Interfaces:**
- Produces:
  - `type SectionId = 'account'|'general'|'providers'|'agents'|'plugins'|'usage'|'character'|'core'`
  - `SECTIONS: { id: SectionId; label: string; group: 'You'|'AI'|'This machine'; desktopOnly: boolean }[]`
    in `sections.ts`; labels "Account", "General", "Providers", "Agents", "Plugins & Skills",
    "Usage", "AYLUL character", "Local Core".
  - `useOpenSettings(): (section: SectionId, agent?: string) => void` and
    `useCloseSettings(): () => void` in `use-settings.ts` (navigate keeping the current path,
    setting or clearing the `settings` and `agent` search keys).
  - `settings-dialog.tsx` holds a fixed map `SectionId -> component`; later tasks add their entry.
  - Root route search: `{ settings?: SectionId; agent?: string }`; an unknown or hidden section
    reads as `general`.

- [ ] **Step 1: Write the failing tests** in `settings-dialog.test.tsx`:
  - `the sidebar's Settings opens the dialog on General`: from `/`, click the button named
    `Settings`; expect `document.querySelector('[role="dialog"]')` to exist, the path to stay `/`,
    the heading "General", and the field `Active projects` with value `5`.
  - `the dialog lists its sections in three groups`: on the desktop fixture, the nav text reads,
    in order, `You Account General AI Providers Agents This machine AYLUL character Local Core`.
  - `Escape closes the dialog and keeps the page`: open from `/projects/p1`, press Escape;
    no dialog, path `/projects/p1`, search has no `settings`, and focus is back on the
    `Settings` button. A click on the backdrop closes it the same way.
  - `/settings redirects to the dialog`: `startApp('/settings')` → path `/`, dialog open on General.
  - `a desktop-only section in the browser falls back to General`: browser fixture,
    `startApp('/?settings=character')` → heading "General".
  - The two moved active-limit tests pass unchanged in their assertions.
- [ ] **Step 2: Run** `cd web && npx vitest run src/app/settings/settings-dialog.test.tsx`
  — expected: FAIL (no dialog).
- [ ] **Step 3: Implement.** `SettingsDialog` uses `Dialog`/`DialogContent` from
  `@/components/ui/dialog` (as `new-project-dialog.tsx` does), about 900×590 px, nav 210 px wide
  on the left, the section's title as `DialogTitle`. `general.tsx` renders today's
  `ActiveProjects` under a "Projects" card. Plugins and Usage are omitted from the nav until their
  tasks. Remove the `/settings` page route's component; keep `/settings` as a redirect to
  `{ to: '/', search: { settings: 'general' } }`.
- [ ] **Step 4: Run** the test file, `src/codemap.test.ts`, `npm run typecheck`, `npm run lint`
  — expected: PASS.
- [ ] **Step 5: Commit** `feat(web): settings opens as a dialog with a sidebar of sections`.

### Task F-3: Account shows only the actions its state allows

**Files:**
- Create: `web/src/app/settings/sections/account.tsx`, `web/src/app/settings/account-actions.ts`
- Modify: `web/src/app/settings/settings-dialog.tsx` (map entry)
- Delete: `web/src/app/desktop/account-panel.tsx`; move `NativeSections`' account state into
  `account.tsx` (character and connection move in F-4)
- Test: `web/src/app/settings/account-actions.test.ts`, `web/src/app/settings/account.test.tsx`
  (the account cases of `web/src/app/desktop/native-sections.test.tsx` move here)

**Interfaces:**
- Consumes: `AuthView`, `AuthAction`, `authAction`, `cancelLogin` from `web/src/desktop/native`.
- Produces: `accountView(auth: AuthView | null, busy: boolean, loggingIn: boolean):
  { message: string; actions: { action: AuthAction | 'cancel'; label: string; confirm?: string;
  danger?: boolean }[] }` in `account-actions.ts`.

- [ ] **Step 1: Write the failing table test** `accountView gives each state its actions`, one
  row per state of the spec's Account table:

  | Input | `message` | `actions` labels |
  |---|---|---|
  | `{state:'signed_out',hasEnrollment:false}` | `You are not signed in. Everything works on this machine.` | `Sign in` |
  | `{state:'signed_in',hasEnrollment:true}` | `Signed in` | `Sign out`, `Sign out all sessions`, `Revoke this device` |
  | `{state:'unavailable',hasEnrollment:true}` | `The account server is unavailable. Your device stays registered.` | `Check again` |
  | `{state:'verification_refused',hasEnrollment:true}` | `Your session could not be verified.` | `Sign in again`, `Check again` |
  | `{state:'device_revoked',hasEnrollment:true}` | `This device was revoked.` | `Register this device again` |
  | any, `busy=true, loggingIn=true` | `Finish signing in in your browser.` | `Cancel sign-in` |

  Also: `Sign in` maps to `'login'` when `hasEnrollment` else `'enroll'`;
  `Register this device again` maps to `'enroll'`; `Check again` to `'status'`;
  `Sign out all sessions` and `Revoke this device` carry today's confirm texts;
  `Revoke this device` has `danger: true` and comes last.
- [ ] **Step 2: Write** `account.test.tsx`: `signed in shows three actions and no Sign in`;
  `an action in progress disables every button`; `the state is checked when the section opens`
  (one `authAction('status')` call per opening, no "Check session" button).
- [ ] **Step 3: Run** both files — expected: FAIL.
- [ ] **Step 4: Implement** `accountView` and `account.tsx` (the danger action in red, at the
  bottom, separated). The principal line shows the account id when signed in.
- [ ] **Step 5: Run** both files, `src/codemap.test.ts`, typecheck, lint — PASS.
- [ ] **Step 6: Commit** `feat(web): account shows only the actions its state allows`.

### Task F-4: AYLUL character and Local Core sections

**Files:**
- Create: `web/src/app/settings/sections/character.tsx`, `web/src/app/settings/sections/core.tsx`
- Modify: `web/src/app/desktop/companion-controls.tsx` (switches), `web/src/app/desktop/connection-settings.tsx`
  (paths under a closed Advanced), `web/src/app/settings/settings-dialog.tsx`
- Delete: `web/src/app/desktop/native-settings.tsx`
- Test: `web/src/app/desktop/companion-binding.test.tsx` (adjust), `web/src/app/settings/core.test.tsx`

**Interfaces:**
- Consumes: `CompanionControls`, `CompanionSettings`, `ConnectionSettings` as they are.

- [ ] **Step 1: Write the failing tests:** `character shows three switches`
  (`role="switch"` named `Show AYLUL on the desktop`, `Motion`, `Show a short reply preview`;
  toggling `Motion` calls `setCompanionPaused(true)` when it was on); `the Local Core paths are
  under a closed Advanced` (the `Shadows executable` field is not visible until the `Advanced`
  disclosure is opened; the Core's address and state show above it).
- [ ] **Step 2: Run** — FAIL.
- [ ] **Step 3: Implement.** The reply-preview checkbox in `companion-settings.tsx` becomes a
  switch with the same label; the visible/paused buttons become switches; error and retry texts
  stay as today.
- [ ] **Step 4: Run** the touched tests, codemap, typecheck, lint — PASS.
- [ ] **Step 5: Commit** `feat(web): character and Local Core sections in the settings dialog`.

### Task F-5: Providers section with search and presets

**Files:**
- Create: `web/src/app/settings/sections/providers.tsx`, `web/src/app/agents/provider-presets.ts`,
  `web/src/app/agents/provider-picker.tsx`
- Modify: `web/src/app/agents/provider-form.tsx` (preset-aware), `web/src/router.tsx` (remove
  `/aylul/providers`), `web/src/app/sidebar/agent-list.tsx` (remove `ProvidersLink`; empty hint
  opens the dialog), `web/src/app/agents/home.tsx` and `web/src/app/agents/agent-settings.tsx:95`
  (their provider links open the dialog), `web/src/app/settings/settings-dialog.tsx`
- Delete: `web/src/app/agents/provider-settings.tsx`
- Test: `web/src/app/agents/aylul.test.tsx` (provider cases move to the dialog),
  `web/src/app/agents/provider-picker.test.tsx`

**Interfaces:**
- Produces: `PRESETS: { id: string; name: string; baseUrl: string }[]` with, in order,
  `minimax` MiniMax `https://api.minimax.io/anthropic`, `anthropic` Anthropic
  `https://api.anthropic.com`, `deepseek` DeepSeek `https://api.deepseek.com/anthropic`,
  `openrouter` OpenRouter `https://openrouter.ai/api`, `moonshot` Moonshot (Kimi)
  `https://api.moonshot.ai/anthropic` (spec OPEN: replaced by the backend's list in F-8);
  `presetFor(baseUrl: string): Preset | null`.
  `ProviderForm` gains prop `preset: Preset | null`: with a preset, the name is prefilled with
  the preset's name, the URL is set from it and its field is not drawn; with `null` (Custom),
  both are drawn and required.

- [ ] **Step 1: Write the failing tests:**
  - `search filters the presets and keeps Custom`: type `mini` → cards `MiniMax`, `Custom` only.
  - `no match leaves Custom and says so`: type `zzz` → `Custom` and "No provider matches.".
  - `a preset hides the URL and sends it`: pick MiniMax, Next, type a key and models, Save → the
    `POST /api/aylul/providers` body has `base_url: 'https://api.minimax.io/anthropic'` and
    `name: 'MiniMax'`; no `Provider URL` field was drawn.
  - `Custom asks for the URL`: pick Custom → `Provider URL` is drawn and required.
  - `the list names each provider with its model count`: two providers → rows
    `MiniMax 3 models connected Edit`, `OpenAI No key not set Set up`.
  - `closing mid-form saves nothing`: type a key, press Escape, reopen Providers → the list, and
    no POST was sent.
  - `the sidebar has no Providers link` and `the AYLUL home's Add a provider opens the dialog on Providers`.
- [ ] **Step 2: Run** — FAIL.
- [ ] **Step 3: Implement.** Edit opens `ProviderForm` with `presetFor(provider.base_url)`;
  key removal and deletion keep `delete-zone.tsx` and their confirmations.
- [ ] **Step 4: Run** the touched tests, codemap, typecheck, lint — PASS.
- [ ] **Step 5: Commit** `feat(web): providers live in the settings dialog, with search and presets`.

### Task F-6: Agents section; Agent settings and New agent open it

**Files:**
- Create: `web/src/app/settings/sections/agents.tsx`
- Modify: `web/src/app/agents/agent-settings.tsx` (form takes `slug: string | null` and
  `onDone()` instead of route params and navigation), `web/src/router.tsx` (remove
  `/aylul/agents/new` and `/aylul/agents/$slug/settings`; keep `/aylul/agents/$slug/new`),
  `web/src/app/sidebar/agent-list.tsx:128-138` and `web/src/app/sidebar/sidebar.tsx` (New agent),
  `web/src/app/agents/home.tsx`, `web/src/app/settings/settings-dialog.tsx`
- Test: `web/src/app/agents/aylul.test.tsx`, `web/src/app/agents/agent-env.test.tsx`
  (reach the form through the dialog)

**Interfaces:**
- Consumes: `useOpenSettings('agents', slug | 'new')` from F-2.
- Produces: `AgentForm({ slug, onDone }: { slug: string | null; onDone: () => void })`.

- [ ] **Step 1: Write the failing tests:**
  - `Agent settings opens the dialog on that agent`: click `Agent settings` under `ay` → dialog,
    heading "Agents", back arrow, the form's `Name` is `ay`, the path is unchanged.
  - `New agent opens an empty form`; saving it returns to the list with the new agent.
  - `the list shows each agent's provider and model`: row `ay MiniMax MiniMax-M3.1-Flash-Preview Edit Delete`.
  - `a deleted agent in the URL shows the list with a note`: `?settings=agents&agent=gone` →
    the list and "This agent no longer exists.".
  - The environment-variable tests of `agent-env.test.tsx` pass through the dialog.
- [ ] **Step 2: Run** — FAIL.
- [ ] **Step 3: Implement.** Back and save both return to the list (`agent` search key cleared).
- [ ] **Step 4: Run** the touched tests, codemap, typecheck, lint — PASS.
- [ ] **Step 5: Commit** `feat(web): agents are edited in the settings dialog`.

---

## Tasks that wait on the request

Each stays `blocked` until the request is `accepted`, its backend side is built and
`api/openapi.json` shows it (`docs/shared/README.md`, step 6). Their steps are written when the
answer fixes the shapes; each follows the TDD shape above.

| Task | Builds (spec section) | Needs from the request |
|---|---|---|
| F-7 | Account: "You" card (name, what AYLUL calls you) and "Devices" with Revoke | Account: the person and their devices |
| F-8 | Providers: Label shown as "Provider (Label)", Test connection, model checklist with default and hand-added ids, per-model context window, presets from the backend (closes the spec's OPEN) | Providers: label, test, models |
| F-9 | Agents: character colour swatches; steps written below | Agents: character colour |
| F-10 | Plugins & Skills section, listed in the nav | Plugins & Skills |
| F-11 | Usage section with period picker, by provider and by model, listed in the nav | Usage |
| F-12 | General: language, theme, start with Windows, hide to tray on close | General |

### Task F-9: Agents: character colour per agent

Spec: "Character colour" under Agents. Written before the request's answer because the field is
small; if the answer renames `character_color` or its values, Masada amends this section before
the task leaves `blocked`.

**Files:**
- Create: `web/src/app/agents/character-colors.ts`, `web/src/app/agents/color-swatches.tsx`
- Modify: `web/src/api/schema.d.ts` (only by `npm run gen:api`), `web/src/app/agents/agent-settings.tsx`
  (`AgentForm`: the field), `web/src/app/settings/sections/agents.tsx` (dot and Companion tag),
  `web/src/app/desktop/companion-settings.tsx` (the colour line under the picker),
  `web/src/app/agents/contract.yaml`, `web/src/app/desktop/contract.yaml`,
  `docs/frontend/codebase/README.md` (rows for the two new files)
- Test: `web/src/app/agents/character-colors.test.ts` (new), `web/src/app/agents/aylul.test.tsx`
  (its agent fixtures gain `character_color`), `web/src/app/desktop/companion-binding.test.tsx`

**Interfaces:**
- Consumes: `Agent['character_color']` and `AgentInput['character_color']` from the regenerated
  schema; `useCompanionBinding()` and `isDesktop()` from `web/src/desktop/`; `AgentForm` from F-6.
- Produces, in `character-colors.ts`:
  - `type CharacterColor = 'blue' | 'red' | 'green' | 'purple' | 'orange' | 'pink'` (equal to the
    schema's union; a `satisfies` check fails the typecheck if they drift)
  - `CHARACTER_COLORS: readonly { id: CharacterColor; label: string; swatch: string }[]`, in
    order: `blue` Blue `#3b82f6`, `red` Red `#ef4444`, `green` Green `#22c55e`, `purple` Purple
    `#a855f7`, `orange` Orange `#f97316`, `pink` Pink `#ec4899`
  - `colorOf(value: string | null | undefined): CharacterColor`: the value when known, else `'purple'`
- Produces, in `color-swatches.tsx`: `ColorSwatches({ value, onChange, disabled }: { value:
  CharacterColor; onChange: (c: CharacterColor) => void; disabled?: boolean })`, a
  `role="radiogroup"` named `Character colour` of native radio inputs (visually round swatches,
  a ring on the checked one, `title` = label); `ColorDot({ color }: { color: CharacterColor })`,
  an `aria-hidden` dot with `data-color={color}`.

- [ ] **Step 1: Write the failing tests:**
  - `character-colors.test.ts`, `the six colours keep their order and swatches`: ids equal
    `['blue','red','green','purple','orange','pink']` and purple's swatch is `#a855f7`;
    `an unknown or missing colour reads as purple`: `colorOf('teal')`, `colorOf(null)` and
    `colorOf(undefined)` are `'purple'`, `colorOf('blue')` is `'blue'`.
  - `aylul.test.tsx`, `a new agent starts on Purple`: open New agent; the radio group
    `Character colour` holds radios named, in order, Blue Red Green Purple Orange Pink, and only
    Purple is checked.
  - `aylul.test.tsx`, `an agent's colour is shown and saved with the form`: agent `ay` with
    `character_color: 'blue'`; open its form, Blue is checked; click Green, then `Save agent`: the
    `PUT /api/aylul/agents/ay` body is `{ name: 'ay', provider, default_model,
    character_color: 'green' }` with the fixture's provider and model; before Save no request
    was sent.
  - `aylul.test.tsx`, `the hint says when the colour shows`: desktop fixture with binding agent
    `ay`: "AYLUL wears this colour now." under `ay`'s swatches; another agent's form: "AYLUL
    wears this colour when this agent is the companion agent."; browser fixture: "AYLUL wears this
    colour on the desktop when this agent is the companion agent."
  - `aylul.test.tsx`, `the list shows each agent's colour and the companion`: two agents, `ay`
    blue and bound, `zed` pink; `ay`'s row holds `[data-color="blue"]` and the text `Companion`,
    `zed`'s row `[data-color="pink"]` and no `Companion`. F-6's row-text assertion stays true
    (the dot has no text; it changes only for the tag).
  - `companion-binding.test.tsx`, `the picker names the chosen agent's colour`: binding agent
    `ay` with `character_color: 'blue'`: the text `Character colour: Blue` and
    `[data-color="blue"]` under the select; no binding: no such line.
- [ ] **Step 2: Run** `cd web && npx vitest run src/app/agents/character-colors.test.ts
  src/app/agents/aylul.test.tsx src/app/desktop/companion-binding.test.tsx`; expected: FAIL.
- [ ] **Step 3: Regenerate** with `npm run gen:api` (only once `api/openapi.json` shows
  `character_color`); the schema is committed with the task.
- [ ] **Step 4: Implement.** `AgentForm`'s value gains `character_color: colorOf(original?.character_color)`
  (purple for a new agent); a `Field label="Character colour"` after Default model holds
  `ColorSwatches` and the hint line (on the desktop, `useCompanionBinding()?.agent === slug`). The
  attempt fingerprint covers the new field as it covers the others (an unchanged retry reuses
  its command id). Nothing is sent to the native shell in this task.
- [ ] **Step 5: Run** the three files, `src/codemap.test.ts`, `npm run typecheck`, `npm run lint`; PASS.
- [ ] **Step 6: Commit** `feat(web): each agent picks AYLUL's character colour`.

### Task F-13: A still of the character beside the swatches

**Files:**
- Create: `web/public/aylul/character/<color>.png`, six files copied unchanged from the stills the
  character side delivers (one per `CHARACTER_COLORS` id, 192×192, transparent)
- Modify: `web/src/app/agents/color-swatches.tsx` (the still), `web/src/app/agents/contract.yaml`
- Test: `web/src/app/agents/aylul.test.tsx`

- [ ] **Step 1: Write the failing test** `the still follows the chosen colour`: open `ay`'s form
  (blue): an `img` named `AYLUL in Blue` whose `src` ends `/aylul/character/blue.png`, 96×96 CSS
  px; click Pink: its name is `AYLUL in Pink` and its `src` ends `pink.png`.
- [ ] **Step 2: Run**; FAIL.
- [ ] **Step 3: Implement.** The image sits left of the swatch row; the six files are referenced
  by URL, not imported, so the bundle does not grow.
- [ ] **Step 4: Run** the file, codemap, typecheck, lint; PASS. Open the form in a browser and
  compare the six stills with the studio's export (screenshot in the report).
- [ ] **Step 5: Commit** `feat(web): a still of AYLUL in the chosen colour`.

### Task F-14: Saving the companion agent repaints AYLUL at once

**Files:**
- Modify: `web/src/desktop/presence.ts` (the native call), `web/src/app/agents/agent-settings.tsx`
  (`AgentForm`'s `onSuccess`), `web/src/desktop/contract.yaml`
- Test: `web/src/app/agents/aylul.test.tsx`

**Interfaces:**
- Consumes: the main-only native command named in the native side's answer (asked as
  `refresh_companion_appearance`, no arguments, returns the companion state).
- Produces: `refreshCompanionAppearance(): Promise<void>` in `presence.ts`, through `nativeCall`;
  a no-op outside the desktop.

- [ ] **Step 1: Write the failing tests:** `saving the companion agent refreshes the character`:
  desktop, binding agent `ay`; change its colour and Save: one `refresh_companion_appearance`
  call, after the PUT succeeded; `saving another agent does not`: save `zed`, no call; `a failed
  refresh does not fail the save`: the call rejects, the form shows no error and the list shows
  the saved agent (the native 10-second check repaints later).
- [ ] **Step 2: Run**; FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** the file, codemap, typecheck, lint; PASS.
- [ ] **Step 5: Commit** `feat(web): saving the companion agent repaints AYLUL`.
