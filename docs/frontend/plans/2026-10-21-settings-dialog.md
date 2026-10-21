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
- Desktop-only sections (`character`, `core`) are listed only when `isDesktop()`. `account` is
  listed everywhere from F-7 on; its sign-in state and actions stay desktop only.
- A section or card that needs the backend is **not drawn** until its endpoint exists: no fake
  control. `plugins` and `usage` are not listed until F-10 and F-11.
- No second dialog on top of the settings dialog: sub-steps replace the section body and show a
  back arrow (`←`, `aria-label="Back"`).
- An API key is never shown after it is saved (today's write-only behaviour stays).
- Every HTTP call goes through `web/src/api/`; each changed folder updates its `contract.yaml`
  and each new or removed file its row in `docs/frontend/codebase/README.md` in the same commit.
- No new dependency.
- **Two daemons** (spec "Two daemons"): no code in this plan reads a base URL, builds a URL or
  calls `fetch` outside `web/src/api/client.ts`, and no test asserts that two sections share one
  origin. A section's queries go through `web/src/api/` functions only, so a later change can point
  them at another daemon in one place.

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
    no dialog, path `/projects/p1/new` (the existing project redirect), search has no `settings`, and focus is back on the
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
- Modify: `web/src/app/settings/settings-dialog.tsx` (map entry), `web/src/app/desktop/native-settings.tsx`
  (drop the `AccountPanel` import, its state and its render; F-4 deletes the file),
  `web/src/app/desktop/native-sections.test.tsx` (its account cases leave)
- Delete: `web/src/app/desktop/account-panel.tsx`; move `NativeSections`' account state into
  `account.tsx` (character and connection move in F-4)
- Test: `web/src/app/settings/account-actions.test.ts`, `web/src/app/settings/account.test.tsx`
  (the account cases of `web/src/app/desktop/native-sections.test.tsx` move here)

**Interfaces:**
  - Consumes: `AuthView`, `AuthAction`, `authAction`, `cancelLogin` from `web/src/desktop/auth`.
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
  (paths under a closed Advanced), `web/src/app/settings/settings-dialog.tsx`,
  `web/src/app/desktop/companion-settings.tsx` (reply-preview switch),
  `web/src/app/desktop/native-sections.test.tsx` (remaining companion case renders CompanionControls;
  the removed NativeSettings browser wrapper case is covered by the dialog's browser section test)
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
- Modify (mechanical follow-up): `web/src/app/agents/delete-zone.tsx` (the existing confirmation
  can render inline in Settings, retaining its text and request without nesting a dialog)
- Delete: `web/src/app/agents/provider-settings.tsx`
- Test: `web/src/app/agents/aylul.test.tsx` (provider cases move to the dialog),
  `web/src/app/agents/provider-picker.test.tsx`

**Interfaces:**
- Produces: `type Preset = { id: string; name: string; region: 'international' | 'china' | null;
  baseUrl: string }`, the request answer's shape; `PRESETS: Preset[]`, its seven rows in its order:
  `minimax` MiniMax international `https://api.minimax.io/anthropic`; `minimax-china` MiniMax
  china `https://api.minimaxi.com/anthropic`; `anthropic` Anthropic null `https://api.anthropic.com`;
  `deepseek` DeepSeek null `https://api.deepseek.com/anthropic`; `openrouter` OpenRouter null
  `https://openrouter.ai/api`; `moonshot` Moonshot (Kimi) international
  `https://api.moonshot.ai/anthropic`; `moonshot-china` Moonshot (Kimi) china
  `https://api.moonshot.cn/anthropic`. F-8 replaces this constant by the backend's list; nothing
  else changes then.
  `presetGroups(presets: Preset[]): { name: string; presets: Preset[] }[]`: one group per `name`,
  in first-seen order, its presets in list order (International first).
  `presetFor(baseUrl: string): Preset | null`.
  `ProviderForm` gains prop `group: { name: string; presets: Preset[] } | null` and its chosen
  `Preset`: with a group, the name is prefilled with the group's name, the URL is set from the
  chosen preset and its field is not drawn; a group of two draws a `Region` radio group
  (International, China; the first checked for a new provider); with `null` (Custom), name and
  URL are drawn and required. Until F-8, the POST still sends today's `name` and `base_url`.

- [ ] **Step 1: Write the failing tests:**
  - `search filters the presets and keeps Custom`: type `mini` → cards `MiniMax`, `Custom` only
    (one MiniMax card for its two presets).
  - `presets are grouped by name`: `presetGroups(PRESETS)` names are, in order, `MiniMax`,
    `Anthropic`, `DeepSeek`, `OpenRouter`, `Moonshot (Kimi)`; MiniMax's presets are
    `minimax`, `minimax-china`.
  - `no match leaves Custom and says so`: type `zzz` → `Custom` and "No provider matches.".
  - `a preset hides the URL and sends it`: pick MiniMax, Next, type a key and models, Save → the
    `POST /api/aylul/providers` body has `base_url: 'https://api.minimax.io/anthropic'` and
    `name: 'MiniMax'`; no `Provider URL` field was drawn.
  - `Custom asks for the URL`: pick Custom → `Provider URL` is drawn and required.
  - `MiniMax China sends the China URL`: pick MiniMax; a `Region` group with `International`
    checked and `China`; click China, type a key and models, Save → the body has
    `base_url: 'https://api.minimaxi.com/anthropic'`. Same for Moonshot with
    `https://api.moonshot.cn/anthropic`.
  - `a one-region preset shows no region`: pick Anthropic → no `Region` group.
  - `Edit keeps the saved region`: a provider with `base_url: 'https://api.moonshot.cn/anthropic'`
    opens with Moonshot fixed and `China` checked.
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
- Modify (mechanical follow-up): `web/src/app/agents/delete-zone.tsx` (compact confirmation
  trigger for an Agents list row; the existing deletion confirmation stays inline)
- Test: `web/src/app/agents/aylul.test.tsx`, `web/src/app/agents/agent-env.test.tsx`
  (reach the form through the dialog)

**Interfaces:**
- Consumes: the callback returned by `useOpenSettings()`, called with `('agents', slug | 'new')`, from F-2.
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
| F-7 | Account: "You" card (name, what AYLUL calls you); steps below. Devices left this plan (spec Account): the account server and device management are a separate piece of work | Account: the person (B-5, built) |
| F-8 | Steps below. Providers: Label shown as "Provider (Label)"; `GET /api/aylul/provider-presets` replaces F-5's `PRESETS` (same shape, grouped by name, so the Region choice stays); save sends `preset` without `name`/`base_url`; Test connection through `POST /api/aylul/providers/test` and, on Edit without a key, `POST /api/aylul/providers/{slug}/test`; `reason` mapped to the spec's sentences, `message` for `unexpected`; `listed: false` offers add-by-hand only; model checklist with search, default and hand-added ids; per-model context window (`null` shows 200,000, editable); the provider-level window field goes (B-2). Lands on the same branch as B-2 | Providers (B-1, B-2, B-3) |
| F-9 | Agents: character colour swatches; steps written below | Agents: character colour |
| F-10 | Shared library management and immediate per-agent switches; full steps below, awaiting controller review | Plugins & Skills (B-9 at `5a34995`, built) |
| F-11 | Period-based usage tables with a token-comparison bar; full steps below, awaiting controller review | Usage (B-7 at `58b4138`, built) |
| F-12 | General: language and theme read and written with the person in Core (`/api/me`, whole record on PUT; `theme` default `system`); start with Windows and hide to tray on close through native, desktop only (native request) | Account: the person (B-5); native switches |

### Task F-7: Account: the You card

Against the built API (B-5): `GET /api/me` → `Person` `{ name?, aylul_calls_me?, language?, theme,
updated_at? }`; `PUT /api/me` `PersonCommand` (`PersonInput & { command_id }`, `theme` required)
→ `Person`. Names ≤64 characters, no line breaks; Core reads `""` as `null`. The PUT takes the
whole record, so the card sends `language` and `theme` as last read (F-12 edits them).

My call, following the spec ("You" works without an account): **Account is listed in the browser
too**, showing only the You card; its sign-in state and actions stay desktop only.

**Files:**
- Create: `web/src/api/person.ts`, `web/src/app/settings/you-card.tsx`
- Modify: `web/src/app/settings/sections/account.tsx` (You card first; the native state and actions
  under it only when `isDesktop()`), `web/src/app/settings/sections.ts` (`account` gets
  `desktopOnly: false`), `web/src/test/fake-daemon.ts` (`GET`/`PUT /api/me`: by default all
  `null`, `theme: 'system'`; `Overrides.person?: Answer`), `web/src/app/settings/contract.yaml`,
  `docs/frontend/codebase/README.md` (rows for the two new files)
- Test: `web/src/app/settings/you-card.test.tsx` (new), `web/src/app/settings/settings-dialog.test.tsx`
  (the browser's nav now holds Account), `web/src/app/settings/account.test.tsx` (unchanged
  assertions; it must still pass)
- Mechanical follow-up: `web/src/api/contract.yaml` documents the new person transport;
  the native-only `account.test.tsx` mocks YouCard so its existing action assertions stay unchanged.

**Interfaces:**
- Produces, in `person.ts`: `type Person = components['schemas']['Person']`,
  `type PersonInput = components['schemas']['PersonInput']`, `personQuery` (`queryKey: ['person']`,
  `GET /api/me`), `savePerson(commandId: string, input: PersonInput): Promise<Person>`
  (`PUT /api/me`), both through `client` and `unwrap` as `aylul-profiles.ts` does.
- Produces, in `you-card.tsx`: `YouCard()`, a form with fields `Your name` and `What AYLUL calls
  you`, the hint "AYLUL uses this name when it speaks to you. Without it, AYLUL uses your name.",
  and **Save**; it uses `attemptFor` so an unchanged retry reuses its `command_id`.

- [ ] **Step 1: Write the failing tests** in `you-card.test.tsx`:
  - `the You card shows the saved names`: `/api/me` answers `{ name: 'Mohammed', aylul_calls_me:
    'Abu Taha', language: 'ar', theme: 'dark' }` → `Your name` is `Mohammed`, `What AYLUL calls
    you` is `Abu Taha`.
  - `saving sends the whole record`: change `What AYLUL calls you` to `Mo`, click Save → one
    `PUT /api/me` with body `{ command_id, name: 'Mohammed', aylul_calls_me: 'Mo', language: 'ar',
    theme: 'dark' }`; no PUT before Save; the field shows `Mo` after the answer.
  - `an emptied field saves as null`: clear `Your name`, Save → the body has `name: null`.
  - `a fresh person shows empty fields`: the default answer → both fields empty, the hint shown.
  - `more than 64 characters is refused before sending`: type 65 characters in `Your name`, Save →
    "At most 64 characters." and no PUT.
  - `a failed save keeps the typed values and retries with the same command`: the PUT answers
    500 → an error line, the typed values stay; Save again → the second PUT carries the first
    one's `command_id`.
  - `the browser lists Account with only You`: browser fixture, open the dialog on Account → the
    You card, no `role="status"` account state and no `Sign in` button.
- [ ] **Step 2: Update** `settings-dialog.test.tsx`'s browser nav expectation to include
  `Account`; run `cd web && npx vitest run src/app/settings/you-card.test.tsx
  src/app/settings/settings-dialog.test.tsx src/app/settings/account.test.tsx`; expected: FAIL.
- [ ] **Step 3: Implement.** The card loads with `personQuery`; until it answers the fields are
  disabled; an error loading it shows `ErrorLine` and no form. The desktop's account state keeps
  today's behaviour under the card.
- [ ] **Step 4: Run** the three files, `src/codemap.test.ts`, `npm run typecheck`, `npm run lint`; PASS.
- [ ] **Step 5: Commit** `feat(web): the You card keeps your name and what AYLUL calls you`.

### Task F-8: Providers: label, presets from Core, test connection, model checklist

Against the built API (B-1..B-3; `web/src/api/schema.d.ts` is already regenerated by `npm run
gen:api` on this branch and is committed with this task):
- `GET /api/aylul/provider-presets` → `ProviderPreset[]` `{ id, name, region?: 'international' |
  'china' | null, base_url }`, seven rows (request answer).
- `Provider` `{ slug, name, base_url, preset?, label?, key_set, models: { id, context_window }[], ... }`;
  the provider-level `context_window` is gone.
- `ProviderInput` `{ preset?, label?, models: { id, context_window? }[], name?, base_url? }`: with a
  preset, send neither `name` nor `base_url`; with `preset: null` (Custom) both are required. A
  missing model window is 200,000 in Core.
- `POST /api/aylul/providers/test` `ProviderTest` `{ api_key, preset? | base_url? }` and
  `POST /api/aylul/providers/{slug}/test` `SavedProviderTest` `{ api_key? }` (without it, the stored
  key), both → `ProviderTestOutcome` `{ ok, listed, models: { id, context_window? }[], reason?:
  'key_refused' | 'unreachable' | 'unexpected', status?, message? }`.
- The provider slug `test` is reserved by Core (it is a route): no screen offers it.

**Files:**
- Create: `web/src/app/agents/provider-models.tsx` (the model checklist: search, ticks, default,
  per-model window, add by id), `web/src/app/agents/provider-test.ts` (outcome → the line to show)
- Create (mechanical lint follow-up): `web/src/app/agents/model-picks.ts` holds shared model
  validation separately from the React component to satisfy Fast Refresh.
- Modify: `web/src/api/schema.d.ts` (already regenerated; commit it), `web/src/api/aylul-profiles.ts`
  (`providerPresetsQuery`, `testProvider`, `testSavedProvider`, the `ProviderPreset` and
  `ProviderTestOutcome` types), `web/src/app/agents/provider-presets.ts` (the `PRESETS` constant
  goes; `presetGroups(presets)`, `presetOf(presets, provider)` by `provider.preset`,
  `providerTitle(provider, presets)`), `web/src/app/agents/provider-picker.tsx` (groups from the
  query), `web/src/app/agents/provider-form.tsx` (label, test, checklist; no provider-level
  window), `web/src/app/settings/sections/providers.tsx` (row titles, Edit's group),
  `web/src/app/agents/agent-settings.tsx` (models are objects: options and the first model use
  `.id`), `web/src/app/conversation/draft.tsx` (the same at lines 174 and 187),
  `web/src/test/fake-daemon.ts` (default answers for the presets and the two test routes),
  `web/src/app/agents/contract.yaml`, `web/src/app/conversation/contract.yaml` (only if a line
  names the model shape), `docs/frontend/codebase/README.md` (rows for the two new files)
- Test: `web/src/app/agents/provider-setup.test.tsx` (new), `web/src/app/agents/provider-picker.test.tsx`,
  `web/src/app/agents/aylul.test.tsx`, `web/src/app/agents/agent-env.test.tsx`,
  `web/src/app/conversation/companion-send.test.tsx`, `web/src/app/conversation/draft.test.tsx`
  (where they hold a provider fixture: it takes `models: [{ id, context_window }]` and loses
  the provider-level `context_window`)
- Mechanical follow-ups: `web/src/app/settings/sections/agents.tsx` and
  `web/src/app/desktop/companion-settings.tsx` use the provider title including its label;
  `web/src/app/desktop/companion-binding.test.tsx` supplies the preset query in its API mock.
  Update `web/src/api/contract.yaml`, `web/src/app/settings/contract.yaml` and
  `web/src/app/desktop/contract.yaml` with the changed folder interfaces.

**Interfaces:**
- Consumes: the schema types above; `attemptFor` from `@/api/command-id`; `ProviderForm`,
  `ProviderPicker`, `ProvidersSection` from F-5.
- Produces, in `aylul-profiles.ts`: `providerPresetsQuery` (`queryKey: ['aylul-provider-presets']`,
  `staleTime: Infinity`); `testProvider(body: ProviderTest): Promise<ProviderTestOutcome>`;
  `testSavedProvider(slug: string, apiKey: string | null): Promise<ProviderTestOutcome>` (sends
  `{}` when `apiKey` is null).
- Produces, in `provider-presets.ts`: `type Preset = ProviderPreset`; `type PresetGroup = { name:
  string; presets: Preset[] }`; `presetGroups(presets: Preset[]): PresetGroup[]` (as F-5, input
  from the query); `presetOf(presets: Preset[], provider: Provider): Preset | null` (by
  `provider.preset`); `providerTitle(provider: Provider, presets: Preset[]): string`: the
  preset's name plus ` China` for a china region, else `provider.name`, then ` (label)` when
  `label` is set, for example `MiniMax China (Work key)`.
- Produces, in `provider-test.ts`: `testLine(outcome: ProviderTestOutcome, providerName: string):
  { kind: 'ok' | 'error'; text: string }`: `key_refused` → "The provider refused this key.";
  `unreachable` → "Could not reach <providerName>. Check your connection and retry.";
  `unexpected` → `message` (or "The test failed." when absent); `ok` and `listed` → "Key works.
  Choose the models to add."; `ok` and not `listed` → "Key works. This provider does not list its
  models: add them by id."
- Produces, in `provider-models.tsx`: `type ModelPick = { id: string; contextWindow: number }`;
  `ProviderModels({ listed, picked, onChange }: { listed: { id: string; context_window?: number |
  null }[]; picked: ModelPick[]; onChange: (picked: ModelPick[]) => void })`: a search box
  (`Search models`, filters the listed ids by substring, case-insensitive), one checkbox per
  listed model, for each ticked model a `Default` radio and a `Context window` number field
  (prefilled from the list, else 200000; 1000 to 10000000), and `Add a model by id` with **Add**
  (refuses an empty id, a duplicate, `default`, more than 128 characters or a space). The default
  is `picked[0]`: choosing Default moves that model first. 1 to 20 models.

- [ ] **Step 1: Absorb the new shapes, behaviour unchanged.** Update the provider fixtures of the
  four test files above; in `provider-form.tsx` read `provider.models.map(m => m.id)` into the
  textarea and send `models: ids.map(id => ({ id }))` without `context_window` (the provider-level
  field is removed from the form); in `agent-settings.tsx` and `draft.tsx` use `.id` where a model
  string was read. Run `npm run typecheck` (0 errors; it had 9) and
  `npx vitest run src/app/agents src/app/conversation/draft.test.tsx src/app/conversation/companion-send.test.tsx`;
  PASS. Commit `fix(web): read provider models as id and context window` with the schema.
- [ ] **Step 2: Write the failing tests** in `provider-setup.test.tsx`:
  - `presets come from Core, grouped by name`: the default presets answer (seven rows) → the
    picker's cards read `MiniMax`, `Anthropic`, `DeepSeek`, `OpenRouter`, `Moonshot (Kimi)`,
    `Custom`; no file still imports a `PRESETS` constant.
  - `the list names a provider by preset and label`: providers `{ preset: 'minimax', label:
    'Work key' }` and `{ preset: 'minimax-china', label: null }` → rows starting `MiniMax (Work
    key)` and `MiniMax China`.
  - `a preset saves its id, not its URL`: pick MiniMax, China, Label `Home`, API key `sk-1`, add
    `MiniMax-M3` by id, Save → `POST /api/aylul/providers` body `{ command_id, preset:
    'minimax-china', label: 'Home', models: [{ id: 'MiniMax-M3', context_window: 200000 }] }` with
    no `name` or `base_url` key; then the key PUT with `sk-1`.
  - `Test connection lists the key's models`: pick MiniMax, key `sk-1`, Test connection → `POST
    /api/aylul/providers/test` body `{ preset: 'minimax', api_key: 'sk-1' }`; answer `{ ok: true,
    listed: true, models: [{ id: 'a', context_window: 1000000 }, { id: 'b', context_window: null
    }] }` → "Key works. Choose the models to add." and checkboxes `a`, `b`, both unticked; tick
    both, choose `b` as Default → the button reads `Add 2 models`; Save sends `models: [{ id: 'b',
    context_window: 200000 }, { id: 'a', context_window: 1000000 }]`.
  - `search filters the models`: listed `MiniMax-M3`, `MiniMax-M3.1-Flash`, `MiniMax-M2.5`; type
    `flash` → only `MiniMax-M3.1-Flash`; a ticked model hidden by the search stays ticked.
  - `a refused or unreachable key says so`: `{ ok: false, reason: 'key_refused', status: 401 }` →
    "The provider refused this key." and no checklist; `unreachable` → "Could not reach MiniMax.
    Check your connection and retry."; `unexpected` with `message: 'Odd answer.'` → `Odd answer.`.
  - `Custom tests its URL and offers add by hand when nothing is listed`: Custom, name `Mine`,
    URL `https://x.example/anthropic`, key, Test → body `{ base_url: 'https://x.example/anthropic',
    api_key }`; answer `{ ok: true, listed: false, models: [] }` → the "does not list its models"
    line and `Add a model by id`; Save sends `preset: null, name: 'Mine', base_url: ...`.
  - `Edit tests with the stored key`: provider `p` with `key_set: true`; Test connection with the
    key field empty → `POST /api/aylul/providers/p/test` body `{}`; with `sk-2` typed → `{ api_key:
    'sk-2' }`. The key field stays empty after either.
  - `Edit shows the saved models with their windows`: provider models `[{ id: 'm1',
    context_window: 500000 }, { id: 'm2', context_window: 200000 }]` → `m1` and `m2` ticked, `m1`
    Default, windows `500000` and `200000`; no provider-level `Context window` field; the button
    reads `Save provider`.
  - `a window out of range is refused`: set `m1`'s window to `999` → "Context window must be a
    whole number from 1000 to 10000000." and no request.
- [ ] **Step 3: Update** the F-5 test `a preset hides the URL and sends it` in `aylul.test.tsx` to
  the new body (`preset: 'minimax'`, no `base_url`), and `provider-picker.test.tsx` to read the
  presets from the fake daemon. Run `npx vitest run src/app/agents/provider-setup.test.tsx
  src/app/agents/provider-picker.test.tsx src/app/agents/aylul.test.tsx`; FAIL.
- [ ] **Step 4: Implement.** Set-up step order: provider name (fixed) and Region, Label (optional,
  ≤64 characters, "Shown in parentheses, to tell two keys apart."), API key with **Test
  connection** beside it (disabled while testing, and when there is no typed key and no stored
  key), the test line, `ProviderModels`, Save. A new provider's Save reads `Add N models` (N
  ticked); Edit reads `Save provider`. The key is never put in a query key, a log or the DOM after
  saving. Deleting and key removal keep `delete-zone.tsx`.
- [ ] **Step 5: Run** the touched tests, `src/codemap.test.ts`, `npm run typecheck`, `npm run lint`; PASS.
  If `provider-form.tsx` passes 300 lines, the commit says why (the set-up step's one job).
- [ ] **Step 6: Commit** `feat(web): providers keep a label, test their key and pick their models`.

### Task F-11: Usage: periods, provider comparison, model totals

**Review gate:** released by the controller, 2026-10-21. B-7 at `58b4138`
implements the accepted Usage request. The generated schema passed typecheck when these steps
were written and is included in this documentation commit; do not regenerate it unless the
canonical OpenAPI changes. No implementation or failing tests are added by this steps commit.

**Built API:** `GET /api/aylul/usage?from=<RFC3339>&to=<RFC3339>` returns `UsageReport`:
- `by_provider: ProviderUsage[]`, each with `provider_slug`, `provider_name`, optional nullable
  `label`, `requests`, `tokens_in`, `tokens_out`, optional nullable `cost` and `currency`.
- `by_model: ModelUsage[]`, each with `model`, `provider_slug`, the same counts and nullable
  cost/currency. Both arrays are required. Alias the generated schemas; do not copy their shapes.
- Core validates `from < to`, normalizes offsets and reads `[from,to)`. It returns 400 for bad
  periods and an ErrorBody for storage failures. The GET has no command id and writes nothing.
  Names/labels are historical snapshots; use the report, not the current Providers list. Core
  already counts recorded invocations and reported token components; do not reinterpret counts.

**Files:**
- Create: `web/src/api/usage.ts`, `web/src/app/settings/sections/usage.tsx`,
  `web/src/app/settings/usage-period.ts`, `web/src/app/settings/usage-tables.tsx`.
- Modify: `web/src/app/settings/sections.ts` (make `usage` visible, leave `plugins` unchanged),
  `web/src/app/settings/settings-dialog.tsx` (map entry), `web/src/test/fake-daemon.ts`
  (default empty usage report; `Overrides.usage?: Answer`), `web/src/api/contract.yaml`,
  `web/src/app/settings/contract.yaml`, `docs/frontend/codebase/README.md` (four new rows),
  `docs/frontend/stages.md` (this task's status).
- Test: `web/src/app/settings/usage.test.tsx`, `web/src/app/settings/usage-period.test.ts`
  (new); `web/src/app/settings/settings-dialog.test.tsx` (Usage nav entry; change its old
  hidden-`usage` fallback case to a genuinely unknown id).
- `schema.d.ts` is already generated in the steps commit. If canonical API drift requires a
  new generation that breaks callers, list mechanical fixes before implementing; do not hand-edit it.

**Touch:** exactly the Files list above; no backend, native shell, pricing service or dependency.

**Interfaces and presentation:**
- `usage.ts`: `type UsageReport/ProviderUsage/ModelUsage = components['schemas'][...]`;
  `usageQuery(from: string, to: string)` → queryOptions, key `['aylul-usage', from, to]`, calling
  `client.GET('/api/aylul/usage', { params: { query: { from, to } } })` through `unwrap`.
- `usage-period.ts`: `type UsagePeriod = '7d' | '30d' | 'month'`;
  `usageRange(period: UsagePeriod, now: Date): { from: string; to: string }`. Last 7/30 days are
  rolling 7/30 × 24 hours ending at the captured time. This month starts at local midnight on
  its first day (`new Date(year, month, 1)` in the person's time zone) and ends at that time
  (controller's call, 2026-10-21: a month is the person's month, not UTC's). No UTC note.
  At the exact first instant of a month, advance only `to` by 1 ms so `from < to` stays valid.
  Serialize with `toISOString()`. These period choices were reviewed and released by the controller.
- `UsageSection()`: controlled `Period` select with `Last 7 days` (default), `Last 30 days`,
  `This month`. Capture `new Date(Date.now())` on opening or selecting a period, not on each
  render; no polling. Use the range in the query key. A new pending period must not show the
  previous period's rows under its label. Loading, ErrorLine with explicit Retry, and a report
  whose two arrays are empty ("No usage in this period.") are distinct states.
- `UsageTables({ report }: { report: UsageReport })`: semantic `By provider` and `By model`
  tables. Show requests, input/output tokens, total tokens (`tokens_in + tokens_out`), cost
  and currency. Provider title is `provider_name` plus ` (label)` when present. Model rows use
  `(provider_slug, model)` identity and join the provider title from `report.by_provider`,
  falling back to the slug if absent. Preserve backend order; never fetch live providers to
  rename historical usage. A missing/nullable cost or currency displays "–"; numeric zero with
  currency is a real zero. Show currency text, never a guessed dollar sign or mixed-currency sum.
- The provider comparison bar compares **total tokens**, not price. Label it `Tokens compared`;
  each row has an accessible meter named `Tokens for <provider title>`, value equal to its
  total tokens, maximum equal to the largest row total (at least 1). Visual width is
  `100 * total / maxTotal` percent; all-zero rows have width 0. No chart dependency. Explain
  "Tokens include only reported usage."; do not imply that unreported tokens or unknown costs
  are measured zero. No client cost estimation, pricing lookup or summing context-window usage.

- [ ] **Step 1: Write the failing tests** (`usage-period.test.ts`, `usage.test.tsx`):
  - `periods produce bounded UTC ranges`: at `2026-10-21T12:00:00.000Z`, assert 7d starts
    `2026-10-14T12:00:00.000Z`, 30d starts `2026-09-21T12:00:00.000Z`, month starts at
    `new Date(2026, 9, 1).toISOString()` (local midnight), and each ends at the input time. Check year rollover and exact
    month start (`to` is start +1 ms); all ranges satisfy `from < to`.
  - `Usage opens with Last 7 days`: open `/?settings=usage`; title Usage, Period value `7d`
    and both table captions. Capture the fake GET's Request URL: its `from`/`to`
    parse as RFC3339, span 7 days, and occur in the query, with no command id/body or mutation.
    Read parameters from the Request: `TestApp.calls` intentionally records only paths.
  - `changing the period fetches its own report`: control `Date.now`, select Last 30 days then
    This month; assert the exact ranges and separate query-cache entries. Hold the month GET
    pending: show loading, no stale 30-day rows. A late older response cannot replace the month.
  - `provider snapshots and labelled keys stay distinct`: two rows with different slugs but
    the same provider name and labels Work/Home appear separately as `MiniMax (Work)` and
    `MiniMax (Home)`, even with the current provider renamed/deleted. Assert no provider-list GET.
  - `provider bars compare total tokens`: inputs `(600,400)` and `(300,200)` produce totals
    1000/500, meter values 1000/500 with max 1000, and widths 100%/50%. Zero-only rows produce
    finite width 0 and meter max 1. No requests/tokens are added a second time.
  - `model rows include their provider`: same model id under two provider slugs remains two
    rows, named from report snapshots; an unmatched slug displays itself.
  - `unknown charges are not zero`: null/absent cost or currency renders "–"; `0, 'USD'`
    renders zero and USD; `0.25, 'USD'` renders that amount and currency. Never add a total
    across USD and EUR or derive price from tokens.
  - `empty and failed usage are distinct`: both arrays empty gives the empty sentence;
    a 500 ErrorBody gives its ErrorLine and Retry, no empty sentence. Retry refetches the same
    range; after success show rows. A 400 period error is also shown without inventing data.
  - Update the dialog's nav-order assertions to include Usage after Agents (after Plugins &
    Skills if F-10 is already built). Unknown ids still fall back to General; desktop-only
    sections still fall back in the browser. Opening another section closes the usage query's UI.
- [ ] **Step 2: Run red**, from `web/`: `npx vitest run src/app/settings/usage.test.tsx
  src/app/settings/usage-period.test.ts src/app/settings/settings-dialog.test.tsx`; observe FAIL.
- [ ] **Step 3: Implement** only these interfaces and states. Update API/settings contracts and
  code-map rows in the same commit. Keep existing sections and F-10's review status unchanged.
- [ ] **Step 4: Run green**, sequentially: those three test files, `npx vitest run
  src/codemap.test.ts`, `npm run typecheck`, `npm run lint`; all PASS. Never whole suite or cargo.
- [ ] **Step 5: Set done and commit** `feat(web): usage compares providers and models by period`,
  stage each file by name, ending `Co-Authored-By: Codex <noreply@openai.com>`.

**Done when:** all named tests/checks pass, Usage is reachable in browser and desktop, all
periods request real reports, bars compare measured tokens, unknown charges stay unknown,
contracts/map are current, and the task commit exists. Native/provider billing acceptance is
not established by fake-daemon tests.

### Task F-10: Plugins & Skills: shared library and immediate agent switches

**Review gate:** released by the controller, 2026-10-21. B-9 at `5a34995`
implements the accepted request after B-8. Use its built schemas and routes, not the older
intended-route fragment this section replaces. No implementation is authorized by this plan commit.

**Built API:** `ExtensionKind = 'plugin' | 'skill'`; `Item { id, name, description }`;
`AgentItem = Item & { enabled: boolean }`; `ExtensionLibrary` and `AgentExtensions` each have
required `plugins` and `skills` arrays. IDs are opaque UUID strings. The library has no enabled
flag and no switch. All schema aliases come from `components['schemas']`.
- `GET /api/aylul/extensions` → 200 ExtensionLibrary.
- `POST /api/aylul/extensions` `{ command_id, kind, path }` → 201 Item; folder is on the
  daemon's machine. It copies/imports once; metadata contains no path. No URL/Git import.
- `DELETE /api/aylul/extensions/{kind}/{id}?command_id=...` → 204, removes every agent's assignment.
- `GET /api/aylul/agents/{slug}/extensions` → 200 AgentExtensions, the entire library with
  enablement for that agent. New agents begin with everything disabled.
- `PUT /api/aylul/agents/{slug}/extensions/{kind}/{id}` `{ command_id, enabled }` → 200 AgentItem.
  Backend validates source/kind (400), missing item/agent (404), duplicate kind/name or conflicting
  replay (409), storage errors (ErrorBody). No screen error may invent a filesystem diagnosis.
  Changed assignments apply when the next agent session opens; the UI must not claim to restart
  or modify a running session. Use the answer's enabled value as the saved result.

**Files:**
- Create: `web/src/api/extensions.ts`, `web/src/app/settings/sections/plugins.tsx`,
  `web/src/app/settings/extension-import.tsx`, `web/src/app/settings/extension-list.tsx`,
  `web/src/app/agents/agent-extensions.tsx`.
- Modify: `web/src/app/settings/sections.ts` (make `plugins` visible, leave `usage` availability
  unchanged), `web/src/app/settings/settings-dialog.tsx` (map entry),
  `web/src/app/agents/agent-settings.tsx` (saved-agent picker after Character colour, outside
  the Save agent form), `web/src/test/fake-daemon.ts` (empty library and per-agent fixtures),
  `web/src/api/contract.yaml`, `web/src/app/settings/contract.yaml`,
  `web/src/app/agents/contract.yaml`, `docs/frontend/codebase/README.md` (five new rows),
  `docs/frontend/stages.md` (this task's status).
- Test: `web/src/app/settings/plugins.test.tsx`, `web/src/app/agents/agent-extensions.test.tsx`
  (new); `web/src/app/settings/settings-dialog.test.tsx` (nav order),
  `web/src/app/agents/aylul.test.tsx`, `web/src/app/agents/agent-env.test.tsx` (regressions).
  No generated schema edit is needed unless the canonical API changes after the steps commit.

**Touch:** exactly the Files list above; no Core files, native commands, local filesystem
access from the browser, dependency, extension execution or global enablement setting.

**Interfaces and presentation:**
- `extensions.ts`: aliases `ExtensionKind`, `Item`, `AgentItem`, `ExtensionLibrary`,
  `AgentExtensions`, `ExtensionImport = components['schemas']['Import']`. Use shared `client`,
  `unwrap` for JSON and `unwrapEmpty` for DELETE 204; callers supply command ids:
  `extensionsQuery` (key `['aylul-extensions']`),
  `addExtension(commandId: string, kind: ExtensionKind, path: string): Promise<Item>`,
  `removeExtension(commandId: string, kind: ExtensionKind, id: string): Promise<void>`,
  `agentExtensionsQuery(slug: string)` (key `['aylul-agent-extensions', slug]`),
  `setAgentExtension(slug: string, kind: ExtensionKind, id: string, enabled: boolean,
  commandId: string): Promise<AgentItem>`.
- `PluginsSection()`: two accessible tabs Plugins (default) and Skills, showing only the
  chosen library array via `ExtensionList`. Add replaces the section body with
  `ExtensionImport({ kind, onDone })` and an accessible Back arrow; no stacked dialog.
  `ExtensionList({ kind, items })` shows name, one-line description (full text available via
  title/accessibility) and Remove. Its inline confirmation is exactly
  `Remove <name> from the library and from every agent?`, with Cancel/Remove.
- Import form: `Folder path` text input, hint `Folder on the daemon's machine.`, Add button;
  refuse blank/whitespace before sending. Do not use a browser file picker, fetch Git/URLs,
  infer metadata from folder names or expose paths outside this input. The active tab fixes
  kind. On success clear the input, return to the same tab and refresh library/agent queries.
  Keep the input after failure; retries of the same trimmed path/kind reuse `attemptFor`'s id;
  a changed path/kind gets a new id. Disable duplicate submissions while pending.
- Remove is never sent before confirmation. Keep a separate attempt per `(kind,id)`; a failed
  delete retains its error/confirmation so explicit retry reuses its id. Pending confirmation
  controls cannot submit twice. Successful add/remove invalidates `['aylul-extensions']` and
  the whole `['aylul-agent-extensions']` prefix, including cached forms for other agents.
- `AgentExtensions({ slug }: { slug: string })`: group `Plugins & Skills`, each item's name,
  description and native checkbox with `role="switch"`. Query only the saved agent's slug.
  A switch sends its PUT immediately, independently of Save agent. Optimistically display
  the desired value, disable that row while pending, then accept the returned enabled state;
  on error restore the last confirmed value and show ErrorLine beside the row. Keep per-item
  command attempts with `(slug,kind,id,enabled)` fingerprint; unchanged explicit retry reuses
  its id. Capture the query key/slug for mutation completion, so a late reply cannot write
  another agent's cache. Reconcile through that agent's scoped query after success.
- New agent: no picker and no agent-extension GET until creation; current F-6 save returns
  to the list. Reopening the created agent shows the picker with all items off. Empty saved
  library: `No plugins or skills yet.` plus a button opening `plugins` in the same settings
  dialog via `useOpenSettings()`; loading/failure must not masquerade as empty. Failure of this
  part must leave ordinary agent fields and environment controls usable.
- Fake defaults: empty library plus empty extension arrays for existing test-agent slugs
  `work`, `aylul`, `agent-a`; allow per-route overrides in individual tests. Do not implement a
  second library store in production or force the query to share an origin with another section.

- [ ] **Step 1: Write the failing tests** in `plugins.test.tsx`:
  - `the library has tabs and no switches`: plugin git and skill review fixtures show names
    and descriptions under their respective tabs, no library `role="switch"`. Empty selected
    tab says `No plugins yet.` or `No skills yet.`; both arrays come from one library GET.
  - `Add imports a folder on the daemon`: choose Skills, Add, enter `C:/skills/review`, submit;
    one POST body is `{ command_id, kind: 'skill', path: 'C:/skills/review' }`. Before Add no
    mutation. The 201 answer supplies id/name/description, input clears, Skills list refreshes.
    Whitespace-only input sends nothing. No native call or filesystem/browser picker is used.
  - `import retry preserves command identity`: first POST fails 500; typed path remains and
    ErrorLine shows; same-input retry has identical body/id. Change path then retry: new id.
    Closing or Back before submit sends nothing; reopening Add starts with an empty input.
  - `Remove confirms its library-wide effect`: click Remove, assert exact question and only
    one settings dialog; Cancel sends no DELETE. Confirm sends the item kind/id in the path
    and `command_id` in query (assert via Request URL), with no JSON body; 204 is success.
  - `a failed Remove can be retried`: 409/500 ErrorBody stays inline, item remains; explicit
    retry uses the same command id. Success removes the item and invalidates library plus
    cached agent-extension queries for two different slugs. No silent automatic mutation retry.
  - `library load failure is not empty`: 500 displays ErrorLine and Retry; no empty sentence.
    Retry recovers. Add/Delete refusal messages remain visible without disabling Settings nav.
- [ ] **Step 2: Write the failing tests** in `agent-extensions.test.tsx`:
  - `the form lists the library's items for this agent`: saved ay answers plugin git enabled
    and skill review disabled; group has two labelled switches, correct checked values and
    descriptions. Another agent's query has a distinct cache key and does not reuse ay's flags.
  - `a switch saves at once`: turn review on; one PUT to ay/skill/<id> with
    `{ enabled: true, command_id }`, no agent POST/PUT and no Save agent click. While pending
    that row is disabled. On answer it reflects the returned enabled value.
  - `a failed switch goes back`: PUT fails 500, switch returns off, fixed server ErrorLine
    shows and agent name/environment controls still work; repeat same toggle reuses the
    original command id. Changing desired enabled state produces a new fingerprint/id.
  - `late toggles remain scoped`: start ay's PUT, navigate to zed's form before it answers;
    settle ay's response and assert zed's values/cache are unchanged. Refetch ay to reconcile.
  - `a new agent shows the part after it is created`: New agent has no group/extension GET;
    create, return to list, reopen its form: group appears with every library item off.
  - `an empty library links to the section`: saved agent with both arrays empty displays
    the exact empty sentence; link opens Plugins & Skills in the same dialog and preserves
    the underlying route. Failed GET shows ErrorLine instead of that empty sentence.
  - `library removal clears every agent view`: cache two agents with the same item enabled,
    remove it from the library, revisit each agent and assert refetched lists omit it.
  - Update nav assertions: Plugins & Skills appears after Agents, before Usage when Usage is
    built. Preserve all existing agent-save, environment and dialog dismissal assertions.
- [ ] **Step 3: Run red**, from `web/`: `npx vitest run src/app/settings/plugins.test.tsx
  src/app/agents/agent-extensions.test.tsx src/app/settings/settings-dialog.test.tsx
  src/app/agents/aylul.test.tsx src/app/agents/agent-env.test.tsx`; observe FAIL.
- [ ] **Step 4: Implement** only the Files/interfaces above; update the three folder contracts
  and code-map rows. Preserve the live agent form's colour/environment controls and all
  accepted F-2 through F-9 behaviour. No backend/native or running-session work.
- [ ] **Step 5: Run green**, sequentially: those five files, `npx vitest run
  src/codemap.test.ts`, `npm run typecheck`, `npm run lint`; all PASS. Never whole suite or cargo.
- [ ] **Step 6: Set done and commit** `feat(web): manage shared plugins and skills per agent`,
  stage each file by name, ending `Co-Authored-By: Codex <noreply@openai.com>`.

**Done when:** library tabs/import/removal and immediate agent switches pass the named tests,
all mutations use explicit retry-stable command ids, no nested dialogs or global enable switch
exist, contracts/map are current, and the task commit exists. Synthetic UI checks establish
frontend requests/presentation, not real extension import/execution on a person's machine.

### Task F-9: Agents: character colour per agent

Spec: "Character colour" under Agents. The colours are dynamic: the web reads them from the one
colour table, `assets/aylul/character-colors.json` (created by U-2 in `docs/aylul/ui/stages.md`), and treats
`character_color` as an opaque id. If the request's answer names the field differently, Masada
amends this section before the task leaves `blocked`.

**Files:**
- Create: `web/src/app/agents/character-colors.ts`, `web/src/app/agents/color-swatches.tsx`
- Modify: `web/src/api/schema.d.ts` (only by `npm run gen:api`), `web/src/app/agents/agent-settings.tsx`
  (`AgentForm`: the field), `web/src/app/settings/sections/agents.tsx` (dot and Companion tag),
  `web/src/app/desktop/companion-settings.tsx` (the colour line under the picker),
  `web/src/app/sidebar/agent-list.tsx` (`AgentRow`'s `Bot` icon in the agent's colour),
  `web/tsconfig.app.json` (`resolveJsonModule: true`), `web/vite.config.ts` (only if the dev
  server refuses the file outside `web/`: `server.fs.allow` gains that one file),
  `web/src/app/agents/contract.yaml`, `web/src/app/desktop/contract.yaml`,
  `web/src/app/sidebar/contract.yaml`, `docs/frontend/codebase/README.md` (rows for the two new files)
- Read only, never changed by this task: `assets/aylul/character-colors.json`
- Mechanical follow-ups: update `web/src/app/settings/contract.yaml` for the list dot/tag.
  Use a fieldset with legend for Character colour: the existing `Field` renders a label and
  cannot wrap the swatches' individual labels without invalid nested labels.
- Test: `web/src/app/agents/character-colors.test.ts` (new), `web/src/app/agents/aylul.test.tsx`
  (its agent fixtures gain `character_color`), `web/src/app/desktop/companion-binding.test.tsx`

**Interfaces:**
- Consumes: `assets/aylul/character-colors.json` from U-2: `{ base: string; colors: { id:
  string; label: string; swatch: string; tones?: object }[] }` (the web reads `colors[].id`,
  `label`, `swatch` and ignores `base` and `tones`); `Agent['character_color']` and `AgentInput['character_color']` from the regenerated
  schema (a `string`, no union); `useCompanionBinding()` and `isDesktop()` from `web/src/desktop/`;
  `AgentForm` from F-6.
- Produces, in `character-colors.ts` (the only web file that imports the table):
  - `type CharacterColor = { id: string; label: string; swatch: string }`
  - `CHARACTER_COLORS: readonly CharacterColor[]`, the table's entries in its order, without `tones`
  - `DEFAULT_COLOR`, the table's `base` id (`'purple'`)
  - `colorOf(id: string | null | undefined): CharacterColor`: the entry with that id, else the
    purple entry
- Produces, in `color-swatches.tsx`: `ColorSwatches({ value, onChange, disabled }: { value: string;
  onChange: (id: string) => void; disabled?: boolean })`, a `role="radiogroup"` named
  `Character colour` with one native radio per `CHARACTER_COLORS` entry (round swatch filled with
  `swatch`, a ring on the checked one, accessible name and `title` = `label`); the checked radio
  is `colorOf(value).id`. `ColorDot({ id }: { id: string | null | undefined })`, an `aria-hidden`
  dot with `data-color={colorOf(id).id}` and that swatch as its colour.
- No component, test or other file names a colour except the table, `DEFAULT_COLOR` and the order
  test. Adding a colour is one table entry plus its id in the order test.

- [ ] **Step 1: Write the failing tests:**
  - `character-colors.test.ts`, `the table's colours, in order`: `CHARACTER_COLORS.map(c => c.id)`
    equals `['blue','green','purple','orange','pink']` (the one line a new colour adds to).
  - `character-colors.test.ts`, `every colour is complete`: for each entry, `id` matches
    `/^[a-z][a-z0-9-]{0,31}$/`, `label` is non-empty, `swatch` matches `/^#[0-9a-f]{6}$/i`; ids are
    unique; an entry with id `purple` exists.
  - `character-colors.test.ts`, `an unknown or missing id reads as purple`: `colorOf('red')`,
    `colorOf('teal')`, `colorOf('')`, `colorOf(null)` and `colorOf(undefined)` are the purple
    entry; for each table entry, `colorOf(entry.id)` is that entry.
  - `aylul.test.tsx`, `a new agent starts on Purple`: open New agent; the radio group
    `Character colour` holds one radio per `CHARACTER_COLORS` entry, named by its `label`, in
    table order, and only Purple is checked.
  - `aylul.test.tsx`, `an agent's colour is shown and saved with the form`: agent `ay` with
    `character_color: 'blue'`; open its form, Blue is checked; click Green, then `Save agent`: the
    `PUT /api/aylul/agents/ay` body is `{ name: 'ay', provider, default_model,
    character_color: 'green' }` with the fixture's provider and model; before Save no request
    was sent.
  - `aylul.test.tsx`, `an id the table does not know shows Purple and is replaced on save`: agent
    `ay` with `character_color: 'red'`; its form has Purple checked; `Save agent` sends
    `character_color: 'purple'`.
  - `aylul.test.tsx`, `the hint says when the colour shows`: desktop fixture with binding agent
    `ay`: "AYLUL wears this colour now." under `ay`'s swatches; another agent's form: "AYLUL
    wears this colour when this agent is the companion agent."; browser fixture: "AYLUL wears this
    colour on the desktop when this agent is the companion agent."
  - `aylul.test.tsx`, `the list shows each agent's colour and the companion`: two agents, `ay`
    blue and bound, `zed` pink; `ay`'s row holds `[data-color="blue"]` and the text `Companion`,
    `zed`'s row `[data-color="pink"]` and no `Companion`. F-6's row-text assertion stays true
    (the dot has no text; it changes only for the tag).
  - `aylul.test.tsx`, `the sidebar's agent icon wears the agent's colour`: agents `ay` blue and
    `zed` with no `character_color` in the fixture; in the app sidebar, `ay`'s row icon has
    `data-color="blue"` and its `color` style is `colorOf('blue').swatch`, `zed`'s has
    `data-color="purple"` and `colorOf('purple').swatch`; opening `ay` keeps its icon blue (the
    open agent is still shown by the row's text colour, as today).
  - `companion-binding.test.tsx`, `the picker names the chosen agent's colour`: binding agent
    `ay` with `character_color: 'blue'`: the text `Character colour: Blue` and
    `[data-color="blue"]` under the select; no binding: no such line.
- [ ] **Step 2: Run** `cd web && npx vitest run src/app/agents/character-colors.test.ts
  src/app/agents/aylul.test.tsx src/app/desktop/companion-binding.test.tsx`; expected: FAIL.
- [ ] **Step 3: Check the schema.** `Agent.character_color: string` and
  `AgentInput.character_color?: string | null` are already in `web/src/api/schema.d.ts` (B-4,
  regenerated and committed by F-8); run `npm run gen:api` only if F-8 has not landed, and commit
  the schema with this task then. Core checks the id's shape (1 to 32 of `a-z`, `0-9`, `-`,
  starting with a letter), which every table id passes.
- [ ] **Step 4: Implement.** `AgentForm`'s value gains `character_color: colorOf(original?.character_color).id`
  (purple for a new agent); a `Field label="Character colour"` after Default model holds
  `ColorSwatches` and the hint line (on the desktop, `useCompanionBinding()?.agent === slug`). The
  attempt fingerprint covers the new field as it covers the others (an unchanged retry reuses
  its command id). In `AgentRow`, the `Bot` icon drops `text-accent-line` and takes
  `style={{ color: colorOf(agent.character_color).swatch }}` and `data-color`. Nothing is sent to
  the native shell in this task. If the desktop build's module inventory
  (`apps/desktop/tests/desktop-build.test.mjs`) refuses the JSON outside `web/`, stop and report.
- [ ] **Step 5: Run** the three files, `src/codemap.test.ts`, `npm run typecheck`, `npm run lint`,
  `npm run build`; PASS.
- [ ] **Step 6: Commit** `feat(web): each agent picks AYLUL's character colour`.

### Task F-12: General: person preferences and desktop behaviour

**Review gate:** released by the controller, 2026-10-22; the light palette needs Mohammed's
visual check in the app. B-5 supplies the person record; N-7 at `cd7fd94` supplies the two native preferences.
The accepted native request `docs/aylul/shared/requests/2026-10-21-settings-dialog-native.md`
owns the camelCase arguments, defaults and failure semantics.

**Built interfaces:**
- Reuse `personQuery`, `Person`, `PersonInput`, `savePerson(commandId, input)` from
  `web/src/api/person.ts`. GET `/api/me` includes names, `language`, `theme`, `updated_at`;
  PUT takes the whole record plus command id. Preserve last-read `name` and `aylul_calls_me`
  when changing General preferences; do not send a partial person record. `theme` is
  `dark | light | system`, default `system`; `language` is an optional nullable opaque tag.
- `desktop_preferences`, no arguments, main only, returns the full
  `{ launchAtLogin: boolean, closeToTray: boolean }`. Fresh native state is false/true.
- `set_desktop_preferences` takes a flat partial object
  `{ launchAtLogin?: boolean, closeToTray?: boolean }`, not `{ preferences: ... }`, and
  returns both booleans after saving. Omitted values are retained by native.
- Fixed native codes are `desktop_preferences_load_failed`,
  `desktop_preferences_save_failed`, `desktop_preferences_autostart_failed`. An OS/storage
  failure can require rereading actual native values; do not claim a rollback succeeded.
  These preferences belong to this machine, never to Core's person or localStorage.

**Files:**
- Create: `web/src/app/settings/general-preferences.tsx` (person preference form),
  `web/src/app/settings/person-theme.tsx` (person theme synchronization),
  `web/src/app/settings/desktop-preferences.tsx` (native switches),
  `web/src/desktop/preferences.ts` (typed native wrappers).
- Modify: `web/src/app/settings/sections/general.tsx` (keep Active projects, add the forms),
  `web/src/app/shell.tsx` (mount PersonTheme once), `web/src/index.css` (light semantic palette,
  preserve the approved dark palette), `web/src/app/settings/contract.yaml`,
  `web/src/desktop/contract.yaml`, `docs/frontend/codebase/README.md` (four new source rows),
  `docs/frontend/stages.md` (this task's status).
- Test: `web/src/app/settings/general-preferences.test.tsx`,
  `web/src/app/settings/person-theme.test.tsx`, `web/src/app/settings/desktop-preferences.test.tsx`
  (new); `web/src/app/settings/settings-dialog.test.tsx`, `web/src/app/settings/you-card.test.tsx`,
  `web/src/app/settings/account.test.tsx` (regressions).
- Read: existing person transport, `web/src/api/saved-answer.ts`, native request/N-7 plan,
  General spec and the settings/desktop contracts. No API regeneration is needed for N-7.

**Touch:** exactly this Files list. No translation catalogue, RTL redesign, native/backend code,
OS login-item installation, personal preference migration or new dependency.

**Interfaces and presentation for review:**
- `GeneralPreferences()`: reads the existing person query, edits Language and Theme, and has
  `Save preferences` (distinct from Active projects' Save). Theme offers System, Dark, Light.
  Use `attemptFor` over the complete PersonInput; unchanged explicit retries reuse their id.
  Disable fields while loading/saving; errors keep drafts. Read the latest cached person when
  submitting so a names update observed while editing is preserved. Write success through
  `writeSaved(client, personQuery.queryKey, answer)` from F-15, not bare `setQueryData`, then
  clear the draft. A pre-save GET must not replace that answer. Core has no revision/CAS here;
  this does not promise conflict-free simultaneous saves by different clients.
- Language must follow the spec's availability rule: do not offer Arabic before its separate
  translations/RTL slice exists. Current choices are Default (English, stored null) and English
  (`en`). An already stored other tag is shown as a disabled current option `Saved (<tag>)`
  and preserved unless the person selects a supported choice; never silently normalize it to
  English while saving only Theme. This availability decision is explicit for controller review.
- The current stylesheet explicitly has one dark palette. Theme must work, not merely save a
  value: `PersonTheme()` renders no UI, subscribes to `personQuery` in Shell after existing
  readiness, and applies the resolved theme through the root's `dark` class and
  `data-theme="dark|light"`. System follows `matchMedia('(prefers-color-scheme: dark)')`, with
  listener cleanup and immediate response to system changes; fixed themes ignore those changes.
  Add light values for the existing semantic palette tokens in `index.css`; preserve all dark
  values and token names, with matching `color-scheme`. No second theme store, pricing/API
  origin assumption, or theme query in the pre-readiness native bootstrap. A failed person read
  leaves the existing initial dark fallback intact. Clean DOM attributes/listeners on unmount.
- `preferences.ts`: `type DesktopPreferences = { launchAtLogin: boolean; closeToTray: boolean }`;
  `getDesktopPreferences(): Promise<DesktopPreferences>` and
  `setDesktopPreferences(patch: Partial<DesktopPreferences>): Promise<DesktopPreferences>`
  through `nativeCall`, with no nesting or snake_case wire fields. Validate both returned
  booleans; malformed replies follow the same fixed load/save error presentation.
- `DesktopPreferencesControls()` mounts only when `isDesktop()` in General. Read on each
  opening; do not write defaults on mount. Two native checkboxes with `role="switch"`, named
  `Start AYLUL with Windows` and `Closing the window hides it to the tray`. Each saves at once,
  outside the person Save action; send only the changed key. Disable both while loading or
  a setter/recovery read is pending, preventing concurrent partial updates from this UI.
  Accept the returned full object, including the untouched switch. These controls do not
  invoke companion-window commands, change visibility, or simulate native close behaviour.
- If a setter fails, show exactly `Could not change this setting.` and reread native
  preferences before re-enabling the controls, displaying what native actually reports.
  If reread also fails, retain the last confirmed display, keep switches disabled, and offer
  `Retry desktop settings` with `Could not load desktop settings.`. Never automatically retry
  the setter. A first read failure uses that same load message/Retry and no invented defaults.
  Errors in native controls do not disable Core preferences or Active projects, and vice versa.

- [ ] **Step 1: Write failing tests** in `general-preferences.test.tsx`:
  - `General reads and saves the whole person`: seed names, language `en`, theme `system`;
    change Theme to Dark, click Save preferences; one PUT includes both names, `language: 'en'`,
    `theme: 'dark'`, command id; no PUT before Save. Choosing Default sends language null.
    An unknown saved language survives a theme-only change, and no enabled Arabic option exists.
  - `observed names are preserved while editing preferences`: while a theme draft exists,
    update the shared person cache with names saved from Account; General sends those latest
    names, not the values captured when the form first opened.
  - `a failed preference save retains its draft and command`: first PUT 500, error shown,
    chosen values retained; same retry has identical body/id, changed preference has a new id.
    Pending saves cannot submit twice; closing or changing sections before Save causes no mutation.
  - `an old read cannot undo saved preferences`: hold a person GET across PUT; complete
    PUT and then the stale GET. New language/theme and the shared person cache remain saved,
    exercising writeSaved cancellation. Loading/error states never become editable fake values.
- [ ] **Step 2: Write failing tests** in `person-theme.test.tsx`:
  - `theme follows the shared person`: mount through Shell; Dark/Light set root class/data
    consistently. A successful General save updates the shared record and root without reload.
  - `System follows the OS and fixed themes do not`: controlled matchMedia false/true selects
    light/dark for System; simulated changes apply only in System. Switching to a fixed theme
    removes that listener. Unmount removes listeners and restores prior DOM attributes.
  - `failed person loading preserves the initial theme`: read refusal leaves initial dark
    state intact; no native preference call is made to choose a theme. Theme code never mounts
    in the local-Core bootstrap. Validate the light token set during the later manual app check;
    DOM tests alone do not establish colour contrast or visual acceptance.
- [ ] **Step 3: Write failing tests** in `desktop-preferences.test.tsx`:
  - `browser General has no native preferences`: no two native switches and no preference
    invokes; language/theme and Active projects stay present.
  - `desktop defaults come from native without a write`: held GET keeps controls disabled;
    resolve false/true, assert unchecked/checked and no setter. Reopening does another read.
  - `each switch sends flat partial camelCase arguments`: launch toggle sends
    `set_desktop_preferences`, `{ launchAtLogin: true }`; close toggle sends
    `{ closeToTray: false }`; no preferences wrapper, snake_case fields or Core PUT. Returned
    full object controls both states. Both switches remain disabled until completion.
  - `a failed setter rereads actual native values`: run each fixed save/autostart failure;
    show the exact change-error sentence, then GET; if actual launch state differs from its
    pre-save value, show actual native state rather than assuming rollback. No automatic setter
    replay. Failed recovery read keeps controls disabled until explicit Retry succeeds.
  - `load failure stays local`: fixed load/autostart failure and malformed reply show the
    fixed load message and Retry, not raw codes/OS details. Core fields remain usable; retry
    reads only. A failed Core save likewise never disables or invokes native switches.
- [ ] **Step 4: Run red**, from `web/`: `npx vitest run
  src/app/settings/general-preferences.test.tsx src/app/settings/person-theme.test.tsx
  src/app/settings/desktop-preferences.test.tsx src/app/settings/settings-dialog.test.tsx
  src/app/settings/you-card.test.tsx src/app/settings/account.test.tsx`; observe FAIL.
- [ ] **Step 5: Implement** the reviewed Files/interfaces, keep the current Projects card,
  update settings/desktop contracts and code-map rows. Preserve API transport/readiness and
  F-15's saved-answer rule. Do not implement translations or make a real OS login-item change.
- [ ] **Step 6: Run green**, sequentially: those six files, `npx vitest run
  src/codemap.test.ts`, `npm run typecheck`, `npm run lint`; all PASS. No whole suite or cargo.
- [ ] **Step 7: Set done and commit** `feat(web): General keeps person and desktop preferences`,
  stage exact paths, ending `Co-Authored-By: Codex <noreply@openai.com>`.

**Done when:** all named tests/checks pass, Core preferences preserve the complete person
record and drive the supported theme, native controls reflect the actual machine state with
independent failures, browser exposes no native switches, contracts/map are current, and the
task commit exists. Controller acceptance of the light palette and real OS login-item/sign-in
behaviour remains separate from the synthetic checks. This steps commit builds none of it.

### Task F-14: Saving the companion agent repaints AYLUL at once

**Files:**
- Modify: `web/src/desktop/presence.ts` (the native call), `web/src/app/agents/agent-settings.tsx`
  (`AgentForm`'s `onSuccess`), `web/src/desktop/contract.yaml`,
  `web/src/app/agents/contract.yaml` (the changed folder's save obligation)
- Test: `web/src/app/agents/aylul.test.tsx`

**Interfaces:**
- Consumes: the accepted main-only native command `refresh_companion_appearance`, no arguments,
  returning exactly `{ ok: true }` (N-6 at `0c4d8c6`); it does not return companion state.
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
