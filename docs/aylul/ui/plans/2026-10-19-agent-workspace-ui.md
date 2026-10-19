# Agent workspace UI implementation plan

> **For agentic workers:** Use `superpowers:executing-plans` for inline,
> task-by-task execution in the current checkout. No worktrees or implementer
> subagents. One local commit per verified task; one final whole-branch review.

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
| `app/SettingsPanel.tsx` | Settings category navigation with actual availability |

## Task 1 - Ship the workspace shell with working account chat

**Files:** create `src/app/{WorkspaceShell.tsx,AccountPanel.tsx,workspace.css}`;
modify `src/app/{main.tsx,shell.css,contract.yaml}`;
create `tests/companion/workspace.spec.ts`; adapt
`tests/companion/{shell.spec.ts,fixture.tsx}` to the intentional page relocation;
update `docs/aylul/{codebase/README.md,ui/stages.md}`.

**Interfaces:**
- `WorkspacePage = 'chat' | 'activity' | 'settings'`.
- `WorkspaceShell({page,onNavigate,chat,activity,settings,presence,onHide})` accepts
  React nodes and callbacks. All content owners stay mounted; inactive views use
  `hidden`, not conditional unmounting. `presence` is a compact native control slot.
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
- [ ] Run `npx playwright test tests/companion/workspace.spec.ts` from desktop;
  confirm failures describe missing workspace behavior, not fixture setup.
- [ ] Implement the sidebar/full-window shell using the approved restrained purple
  palette. Remove global `main`/`section` geometry that forces every page into the
  old centered document. Scope all workspace styling; preserve the independent
  companion bundle. Reuse existing account/assistant/session/approval operations.
- [ ] Adapt old shell assertions to navigate to Settings/Activity before their
  controls. Preserve their behavioral assertions; do not simply delete them.
- [ ] Run `npm run build`, the new workspace suite and existing shell suite.
  Update app contract/map/stage and commit `feat: add the live desktop workspace shell`.

## Task 2 - Put real conversations in the workspace sidebar

**Files:** create
`src/features/assistant/{useAssistantConversation.ts,ConversationSidebar.tsx,ConversationView.tsx}`;
modify `AssistantPage.tsx`, `assistant.css`, its `contract.yaml`, `app/main.tsx`,
`app/WorkspaceShell.tsx`, app contract; create
`tests/companion/workspace-conversations.spec.ts`; extend the existing shell fixture
with explicit account/list/snapshot/command responses, preserving current flags;
update assistant acceptance fixture/runner only where composition intentionally
changes; update code map and stages.

**Interfaces:**
- `useAssistantConversation(auth: AuthView | null): AssistantController` returns
  `conversations,selected,snapshot,draft,error,ready,busy,activeTurn` plus
  `setDraft(text),select(id),create(),send(),stop(turnId),recover(turnId)`.
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
  new command and explicit retry retains its identity.
- [ ] Run the new conversation suite to observe RED, then move existing state,
  polling and command logic into the hook without changing 4096-byte input,
  1500ms visible polling, stop/recovery or error recovery semantics.
- [ ] The sidebar searches existing metadata locally and displays timestamps to
  distinguish same-titled conversations. No transcript fetch for search; no
  invented profile IDs or copied backend wire definitions.
- [ ] Run build, workspace-conversation suite and `tests/assistant-state.mjs`
  using its documented test-only Vite fixture. No paid acceptance call is required.
- [ ] Update affected contracts/map/stage and commit
  `feat: share live conversation state with the workspace sidebar`.

## Task 3 - Expose truthful bot/settings surfaces and verify the UI batch

**Files:** create `src/app/SettingsPanel.tsx` and
`src/features/assistant/AssistantDetails.tsx`; modify the existing workspace/app/
assistant components and CSS/contracts; extend workspace browser suites and
`tests/companion/production.test.ts`; update desktop README, code map, UI stages
and dated workspace evidence under `docs/evidence/aylul/`.

**Interfaces:**
- `AssistantDetails()` describes the current assistant connection and unavailable
  bot-management capabilities. It takes no secret or assumed Hermes profile ID.
- `SettingsPanel({account,presence,studio})` receives existing presentation nodes
  and organizes Account, Appearance and Integrations. The studio remains DEV-only.
- The Bots navigation presents the existing AYLUL assistant and a clear capability
  boundary. Creating additional bots and saving provider/model/profile settings
  stay disabled with a reason until a backend management contract exists.

- [ ] RED: `workspace_unavailable_bot_configuration_never_dispatches` verifies
  unsupported create/model controls are disabled, name the missing capability,
  and issue no native command. No fake saving/success state is permitted.
- [ ] RED: `workspace_settings_keep_account_and_character_independent` verifies
  character failure does not disable account navigation/actions or local activity.
- [ ] RED: extend production graph checks: companion cannot import workspace,
  account, conversation or bot administration; no test data/studio in production.
- [ ] Implement the inspector/settings pages without exposing unsupported schemas
  as saved configuration. Match the approved layout; include keyboard and mixed-
  text checks with the existing character shown only by its native owner.
- [ ] Run the complete affected desktop gate once: build, `npm test`, companion
  unit tests, companion browser suite and production graph check. Rebuild the
  existing native debug executable after frontend build; no protocol change or
  Core Cargo gate. Physical native use remains distinct from browser fixtures.
- [ ] Record tests, screenshots and exact remaining backend/character gates;
  commit `feat: add scoped workspace settings and assistant details`.
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

Read this written task breakdown before product edits. The chosen workflow is
inline in this checkout, preserving the user's existing preference. After this
plan review, execute tasks continuously with their individual checks and commits.
