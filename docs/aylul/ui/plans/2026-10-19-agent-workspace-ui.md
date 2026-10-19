# Agent workspace UI implementation plan

> **For agentic workers:** Masada runs each task with `frontend-builder`
> (`Do task U-<n> in docs/aylul/ui/stages.md`) and failures with
> `frontend-fixer`, under [`AGENTS.md`](../AGENTS.md) (the human's ruling,
> 2026-10-19). No worktrees. One local commit per verified task; one final
> whole-branch review. Each task changes only its `Touch:` files.

**Goal:** Deliver the approved Chat-first desktop workspace while preserving real
account chat, local hook actions and the independent character.

**Architecture:** The React shell owns navigation and native account state. The
assistant feature owns conversation state and commands; the workspace consumes
its public controller. The current 0.5.0 backend has one assistant connection;
additional bots/configuration remain visibly unavailable until their real backend
contract exists. Preview data remains test-only.

**Tech stack:** Existing React 19, TypeScript, Vite, Tauri and Playwright; no new
product dependency or backend/Core/native protocol change in this UI batch.

**Spec:** [Approved workspace design](../specs/2026-10-19-agent-workspace-design.md).
The human approved the interactive mockup and requested task-by-task execution on
2026-10-19. This written plan is the reviewable breakdown of the first UI batch.

## Global constraints

- Work on `feat/agent-workspace`; preserve unrelated changes; no worktrees.
- English source, tests, assertions and product copy; Arabic/mixed chat content works.
- Reuse generated backend DTOs and native `assistant_call`; no duplicate network path.
- No fake bot ID, runtime creation, model change, memory or monitoring success.
- Keep one native character window, its ACL, render-permission and failure isolation.
- No API keys or ignored credential values in source, logs, evidence or model context.
- Account/security actions belong to settings, not the main conversation.
- One focused check/commit per task; the complete affected UI gate runs at the end.

## Review focus

1. Navigation must not remount conversation state or lose an unsent draft (Task 1).
2. Account changes must invalidate old list/snapshot results and clear drafts (Task 2).
3. Lost send replies must retain the original command ID; reads never POST (Task 2).
4. Narrow windows and long Arabic/English text must preserve composer/navigation (Tasks 1/3).
5. Missing bot management must never look like a successful configurable bot (Task 3).
6. The composer never names a model the backend did not report (Task 3).
7. A person runs the native app before the batch is called done (Task 4).

## File ownership

`src/` below means `apps/desktop/src/`. App files stay under the existing app
contract. Assistant files stay under its existing feature contract. Contracts/map
describe implemented ownership at each commit, not the proposed final tree.

| File | Job |
| --- | --- |
| `app/main.tsx` | Compose native-backed main experience |
| `app/WorkspaceShell.tsx` | Workspace navigation and persistent content slots |
| `app/AccountPanel.tsx` | Existing account/security action presentation |
| `app/workspace.css` | Workspace geometry and theme |
| `features/assistant/useAssistantConversation.ts` | Account-scoped conversation lifetime and commands |
| `features/assistant/ConversationSidebar.tsx` | Real conversation selection/search |
| `features/assistant/ConversationView.tsx` | Conversation history/composer presentation |
| `features/assistant/AssistantPage.tsx` | Standalone compatibility wrapper for existing acceptance |
| `features/assistant/AssistantDetails.tsx` | Truthful current-assistant capability inspector |
| `features/assistant/BotList.tsx` | The Bots list showing the one real assistant |
| `app/SettingsPanel.tsx` | Settings category navigation with actual availability |

## Task 1 - Ship the workspace shell with working account chat

**Files:** create `src/app/{WorkspaceShell.tsx,AccountPanel.tsx,workspace.css}`;
modify `src/app/{main.tsx,shell.css,contract.yaml}`;
create `tests/companion/workspace.spec.ts`; modify
`tests/companion/{shell.spec.ts,fixture.tsx}` for the intentional page relocation;
modify `docs/aylul/{codebase/README.md,ui/stages.md}`.

**Interfaces:**
- `WorkspacePage = 'chat' | 'activity' | 'settings'`. Activity is the home of the
  existing Sessions/Requests views (spec §3).
- `SidebarList = 'chats' | 'bots'`: the spec §3 Chats | Bots switch inside the
  list column of the Chat page, not a fourth page. Task 1 renders the switch with
  Chats only active; Task 3 fills the Bots list.
- `WorkspaceShell({page,onNavigate,list,onList,chat,bots,activity,settings,presence,
  onHide})` accepts React nodes and callbacks. All content owners stay mounted;
  inactive views use `hidden`, not conditional unmounting. `presence` is a compact
  native control slot.
- Contract rows added to `src/app/contract.yaml`: `WorkspaceShell`, `AccountPanel`.
  The existing `shell.spec.ts` obligations are rewritten to name their new page.
- `AccountPanel({auth,busy,loggingIn,onAction,onCancel})` consumes existing
  `AuthView | null`, `AuthAction` and native callbacks. Preserve confirmation text
  for new installation, sign-out-all and revoke. It holds no credential.
- Keep `Shell` as the existing composition/auth owner; do not add a global store.

- [ ] RED: browser `workspace_routes_account_to_settings_and_preserves_chat_draft`
  verifies Account is absent from chat, appears in settings, and navigation back
  preserves the typed draft with no extra submit.
- [ ] RED: `workspace_activity_keeps_approval_actions_and_native_hide` verifies
  Sessions/Requests remain actionable under Activity and hide still invokes native.
- [ ] RED: `workspace_narrow_layout_preserves_navigation_and_composer` checks 390px
  and 1280px, no horizontal overflow, accessible navigation and visible composer.
  At 390px the list column is closed behind a drawer button that opens and
  closes it by mouse and keyboard.
- [ ] Run `npx playwright test tests/companion/workspace.spec.ts` from desktop;
  confirm failures describe missing workspace behavior, not fixture setup.
- [ ] Implement the sidebar/full-window shell using the approved restrained purple
  palette. Remove global `main`/`section` geometry that forces every page into the
  old centered document. Scope all workspace styling; preserve the independent
  companion bundle. Reuse existing account/assistant/session/approval operations.
- [ ] Adapt old shell assertions to navigate to Settings/Activity before their
  controls. Preserve their behavioral assertions; do not simply delete them.
- [ ] From `apps/desktop/`: `npm run build && npx playwright test
  tests/companion/workspace.spec.ts tests/companion/shell.spec.ts`.
  Update app contract/map/stage and commit `feat: add the live desktop workspace shell`.

## Task 2 - Put real conversations in the workspace sidebar

**Files:** create
`src/features/assistant/{useAssistantConversation.ts,ConversationSidebar.tsx,ConversationView.tsx}`;
modify `src/features/assistant/{AssistantPage.tsx,assistant.css,contract.yaml}`,
`src/app/{main.tsx,WorkspaceShell.tsx,workspace.css,contract.yaml}`; create
`tests/companion/workspace-conversations.spec.ts`; modify
`tests/companion/fixture.tsx` (explicit account/list/snapshot/command responses,
current flags preserved, and an `authAction` that can switch account/session
mid-test for the race test), `tests/assistant-fixture.tsx` and
`tests/assistant-state.mjs` (only for the hook/view split); modify
`docs/aylul/{codebase/README.md,ui/stages.md}`.

**Interfaces:**
- `useAssistantConversation(auth: AuthView | null): AssistantController` returns
  `conversations,selected,snapshot,draft,error,ready,busy,activeTurn,pinned,connection`
  plus `setDraft(text),select(id),create(),send(),retry(),stop(turnId),
  recover(turnId),togglePin(id)`. `retry()` resends the uncertain send with its
  original command ID; it never mints a new one.
- `pinned` is a set of conversation IDs kept in `localStorage` under a key that
  includes the account ID; another account never sees it (spec §5 first slice).
- `connection` is `'connected' | 'unavailable'` from the existing
  `assistant.status`/call failures. While unavailable, the last loaded list and
  snapshot stay readable and the view shows a small indicator with a Retry action.
- Contract rows added to `src/features/assistant/contract.yaml`:
  `useAssistantConversation`, `ConversationSidebar`, `ConversationView`.
- The new hook and views are written as readable multi-line source, not the
  single-line style of the current `assistant.css`/`shell.css`.
- DTOs remain generated `AssistantConversation`/`AssistantSnapshot` aliases.
- `ConversationSidebar({controller})` reads that controller; no independent poller.
- `ConversationView({controller})` renders the existing history, states and composer.
- `AssistantPage({auth})` wraps the same hook/view for existing standalone tests;
  production workspace calls the hook once and shares it between sidebar/view.

- [ ] RED: `workspace_selects_exact_conversation_and_creates_once` uses two real-
  shaped conversation IDs and asserts exact native targets and one create command.
- [ ] RED: `workspace_ignores_previous_account_results` delays a snapshot, changes
  account/session, then resolves it; neither prior text nor draft can reappear.
- [ ] RED: `workspace_uncertain_send_keeps_command_without_automatic_repost`
  rejects the response after accepting a send; polling/navigation never sends a
  new command and `retry()` sends the original command ID.
- [ ] RED: `workspace_pins_conversation_per_account` pins one conversation, sees
  it first in the list, switches account and sees no pin.
- [ ] RED: `workspace_offline_keeps_saved_history_and_recovery` fails status and
  calls after a loaded snapshot; history stays readable, the indicator shows, and
  Retry restores the connected state with no send.
- [ ] Run the new conversation suite to observe RED, then move existing state,
  polling and command logic into the hook without changing 4096-byte input,
  1500ms visible polling, stop/recovery or error recovery semantics.
- [ ] The sidebar searches existing metadata locally and displays timestamps to
  distinguish same-titled conversations. No transcript fetch for search; no
  invented profile IDs or copied backend wire definitions.
- [ ] From `apps/desktop/`: `npm run build && npx playwright test
  tests/companion/workspace-conversations.spec.ts tests/companion/workspace.spec.ts`
  and `node tests/assistant-state.mjs` with its documented test-only Vite fixture.
  No paid acceptance call is required.
- [ ] Update affected contracts/map/stage and commit
  `feat: share live conversation state with the workspace sidebar`.

## Task 3 - Expose truthful bot/settings surfaces

**Files:** create `src/app/SettingsPanel.tsx`,
`src/features/assistant/{AssistantDetails.tsx,BotList.tsx}`; modify
`src/app/{main.tsx,WorkspaceShell.tsx,workspace.css,contract.yaml}`,
`src/features/assistant/{ConversationView.tsx,assistant.css,contract.yaml}`,
`tests/companion/{workspace.spec.ts,workspace-conversations.spec.ts,production.test.ts}`;
modify `docs/aylul/{codebase/README.md,ui/stages.md}`.

**Interfaces:**
- `AssistantDetails()` describes the current assistant connection and unavailable
  bot-management capabilities. It takes no secret or assumed Hermes profile ID.
- `BotList()` fills the `bots` list slot with the one existing AYLUL assistant
  and a disabled Create bot control that names the missing capability.
- `SettingsPanel({account,presence,studio})` receives existing presentation nodes
  and organizes Account, Appearance and Integrations. The studio remains DEV-only.
- Creating additional bots and saving provider/model/profile settings stay
  disabled with a reason until a backend management contract exists.
- The composer shows `Model: managed by the account backend` and no model name:
  the generated `AssistantStatus` carries only `state`, so the spec §5 effective
  model has no truthful source yet (spec §5 OPEN - Effective model readback).
- Contract rows added: `SettingsPanel` (app); `AssistantDetails`, `BotList`
  (assistant).

- [ ] RED: `workspace_unavailable_bot_configuration_never_dispatches` verifies
  unsupported create/model controls are disabled, name the missing capability,
  and issue no native command. No fake saving/success state is permitted.
- [ ] RED: `workspace_composer_never_invents_a_model_name` verifies the composer
  shows the managed-by-backend line and no provider/model string.
- [ ] RED: `workspace_settings_keep_account_and_character_independent` verifies
  character failure does not disable account navigation/actions or local activity.
- [ ] RED: `workspace_narrow_layout_collapses_inspector` at 390px: the inspector
  is closed, opens from a button and closes again; the composer stays visible.
- [ ] RED: `workspace_keyboard_reaches_navigation_list_and_composer` tabs through
  page navigation, list, composer and inspector without a trap.
- [ ] RED: `workspace_mixed_text_uses_auto_direction`: Arabic and mixed
  Arabic/English messages and titles carry `dir="auto"`.
- [ ] RED: extend production graph checks: companion cannot import workspace,
  account, conversation or bot administration; no test data/studio in production.
- [ ] Implement the inspector/settings/bots pages without exposing unsupported
  schemas as saved configuration. Match the approved layout; the character is
  shown only by its native owner.
- [ ] From `apps/desktop/`: `npm run build && npx playwright test
  tests/companion/workspace.spec.ts tests/companion/workspace-conversations.spec.ts
  && npm run test:companion:production`.
- [ ] Update contracts/map/stage and commit
  `feat: add scoped workspace settings and assistant details`.

## Task 4 - Run the batch gate and the native journey

**Files:** modify `apps/desktop/README.md` and `docs/aylul/ui/stages.md`; create
`docs/evidence/aylul/<run date>-agent-workspace.md` (the date the journey runs).

- [ ] Run the complete affected desktop gate once from `apps/desktop/`:
  `npm run build && npm test && npm run test:companion:unit &&
  npm run test:companion:browser && npm run test:companion:production`.
  No protocol change, so no Core Cargo gate.
- [ ] Rebuild the native debug app from the repository root:
  `cargo build --locked --manifest-path apps/desktop/src-tauri/Cargo.toml`.
- [ ] Mohammed runs the debug app by hand and walks one journey: sign in; chat
  and leave a draft, navigate away and back, the draft remains; approve one
  request under Activity; open Settings and see account actions; hide and show the
  character. Read the app logs afterwards. Browser fixtures do not count here.
- [ ] Write the evidence file: gate output and test counts, screenshots, the
  journey result as Mohammed reported it, and the remaining backend/character
  gates. Update the README and stages; commit
  `docs: record the agent workspace UI batch`.
- [ ] Request the single final whole-branch review; fix concrete findings with
  reproductions and scoped commits. No push/merge without the person's request.

## Follow-on plans and coverage boundary

This UI plan implements the spec's first delivery slice. It does not certify
multiple live bots or settings parity. The source inventory is
`docs/aylul/ui/specs/2026-10-19-hermes-settings-inventory.md`; dynamic schema/plugin
capabilities require the isolated backend-management proof before enabled writes.

Separate subsequent plans own: the installed-Hermes management trial and accepted
account bot API, durable bot/model/credential binding and migration, then selected-
bot character activity. They precede enabling those UI controls. Code context,
external agent control, voice, jobs and server deployment retain their own gates.

## Execution review

Masada reviewed this written breakdown on 2026-10-19; those fixes are applied
(exact `Touch:` lists, model readback, Bots/pinning/offline/narrow checks, the
Task 3/4 split and the execution ruling above). Execute tasks continuously with
their individual checks and commits; Task 4's native journey needs Mohammed.
