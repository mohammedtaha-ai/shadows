# Desktop UI stages

Where this side's work is now. Masada keeps this file; builders read
their task here and set its status. Task shape: [`AGENTS.md`](AGENTS.md#a-task-in-stagesmd).

## Now

2026-10-19 new direction: the human requested a multi-bot workspace inspired by
Hermes Desktop's bots/chats/settings and selected one visible character for the
selected bot. The [written proposal](specs/2026-10-19-agent-workspace-design.md)
and Chat-first mockup were approved. The [first UI plan](plans/2026-10-19-agent-workspace-ui.md)
was reviewed by Masada on 2026-10-19 and its fixes applied; U-W1 is next. Since
2026-10-19 (the human's ruling) Masada leads this side: she runs
`frontend-builder` with `Do task U-<n> in docs/aylul/ui/stages.md`, and
`frontend-fixer` for failures; each task changes only its `Touch:` files. No workspace/bot-registry
implementation has started; normal chat below remains delivered. Backend
management capabilities, and the effective-model readback (spec §5 OPEN), need
their own proof.

Normal-chat H-2 is implemented. On 2026-10-18 the human selected
AYLUL as the primary UI with the existing Shadows workspace loaded on request.
The [integration draft](../plans/2026-10-18-aylul-hermes-semantic-context.md)
and [transition plan](../plans/2026-10-18-nylra-transition.md)
record that direction. The earlier backend chat/provider work remains its own
implemented owner. The human subsequently requested normal UI chat now, deferring
MCP/Core. The new assistant facade uses bounded snapshot polling.
Character design/renderer and native companion lifetime keep their separate owners.

## Tasks

### U-B1 · Review the agent-workspace design
- Status: done (design/mockup approved; first written UI plan prepared)
- Read: docs/aylul/ui/specs/2026-10-19-agent-workspace-design.md; current assistant/platform/companion contracts.
- Touch: this proposal and its existing vision/navigation/stage references only.
- Needs: human review of the written design before implementation planning.
- Done when: layout, bot/configuration ownership, staged feature inventory and selected-character semantics are accepted; then write an exact implementation plan.

### U-W1 · Live workspace shell
- Status: done
- Plan: docs/aylul/ui/plans/2026-10-19-agent-workspace-ui.md#task-1---ship-the-workspace-shell-with-working-account-chat
- Read: workspace design §3; source settings inventory; app/assistant/features contracts.
- Touch: apps/desktop/src/app/{WorkspaceShell.tsx,AccountPanel.tsx,workspace.css,main.tsx,shell.css,contract.yaml}; apps/desktop/tests/companion/{workspace.spec.ts,shell.spec.ts,fixture.tsx}; docs/aylul/codebase/README.md; docs/aylul/ui/stages.md.
- Needs: none
- Done when: from apps/desktop/, `npm run build && npx playwright test tests/companion/workspace.spec.ts tests/companion/shell.spec.ts` passes; task committed.

### U-W1b · Match the workspace shell to the approved mockup
- Status: done
- Plan: docs/aylul/ui/plans/2026-10-19-agent-workspace-ui.md#task-1---ship-the-workspace-shell-with-working-account-chat (same behaviour; this task changes only the shell's look and placement)
- Read: docs/aylul/ui/specs/2026-10-19-agent-workspace-mockup/agent-workspace-desktop.png and agent-workspace-settings.png (the target), agent-workspace.html (exact colours, sizes, spacing: copy its values); workspace design §3; apps/desktop/src/app/contract.yaml.
- Touch: apps/desktop/src/app/{WorkspaceShell.tsx,AccountPanel.tsx,workspace.css,shell.css,main.tsx,contract.yaml}; apps/desktop/tests/companion/{workspace.spec.ts,shell.spec.ts}; docs/aylul/ui/stages.md.
- Needs: U-W1
- Scope: the shell only, as in the mockup.
  - A top bar across the window: the AYLUL mark, "AYLUL" and "/ personal workspace" on the left. Its right side holds the existing presence controls and "Hide to tray" (no "Design preview" chip).
  - One left column, replacing today's separate rail and list column: Chats | Bots tabs at the top (Bots stays disabled; Chats also returns to the Chat page), a search box (disabled until U-W2), a list area, then a bottom group of Activity, Scheduled jobs (disabled, labelled "Not available yet") and Settings, then the signed-in person's footer (name, "Personal workspace") or a "Sign in" entry when signed out.
  - The mockup's theme: background, panel and border colours, the lilac accent and selected states, radii, font sizes and spacing, taken from agent-workspace.html.
  - Not in this task: the conversation header, messages, composer and the right "Bot details" inspector (U-W2/U-W3), and "Create bot".
  - Keep every U-W1 behaviour: pages stay mounted, the drawer at 390px opens and closes by mouse and keyboard, no horizontal overflow at 390px and 1280px. Update tests only where a label or position moved.
- Done when: from apps/desktop/, `npm run build && npx playwright test tests/companion/workspace.spec.ts tests/companion/shell.spec.ts` passes; the report lists every visible difference from agent-workspace-desktop.png that remains in the shell; task committed.

### U-W1c · Shell icons and spacing polish
- Status: done
- Plan: docs/aylul/ui/plans/2026-10-19-agent-workspace-ui.md#task-1---ship-the-workspace-shell-with-working-account-chat (look only; no behaviour change)
- Read: docs/aylul/ui/specs/2026-10-19-agent-workspace-mockup/agent-workspace-desktop.png (target) and agent-workspace.html (it names each Lucide icon with `data-lucide="<name>"`); apps/desktop/src/app/contract.yaml; docs/aylul/codebase/README.md (app rows).
- Touch: apps/desktop/src/app/{icons.tsx (new),WorkspaceShell.tsx,workspace.css,contract.yaml}; apps/desktop/tests/companion/workspace.spec.ts; docs/aylul/codebase/README.md; docs/aylul/ui/stages.md.
- Needs: U-W1b
- Scope:
  - `icons.tsx` holds the shell's icons as small inline SVG components, with the path data of the Lucide icons the mockup names (Lucide is ISC-licensed; say so in a one-line comment). No new npm dependency.
  - Icons as in the mockup: the AYLUL mark, Chats (`messages-square`), Bots (`bot`), the search box's magnifier (`search`), Activity (`activity`), Scheduled jobs (`calendar-clock`), Settings (`settings-2`), and the drawer button (`panel-left`). Use the names the HTML actually has where they differ from these. Icons are `aria-hidden`; accessible names stay as today.
  - "Scheduled jobs" stays on one line at 1248px; "Not available yet" goes under it or becomes its tooltip/title, not a squeezed column.
  - Top bar right side: presence text and buttons on one row, aligned and spaced like the mockup's top bar (small, muted, no wrapping at 1248px).
  - The selected list/nav item gets the mockup's left accent bar.
- Done when: from apps/desktop/, `npm run build && npx playwright test tests/companion/workspace.spec.ts tests/companion/shell.spec.ts` passes; a 1248x948 screenshot of the Chat page is compared with agent-workspace-desktop.png and the report lists what still differs in the shell; task committed.

### U-W2 · Shared live conversation navigation
- Status: done
- Plan: docs/aylul/ui/plans/2026-10-19-agent-workspace-ui.md#task-2---put-real-conversations-in-the-workspace-sidebar
- Read: Task 2 interfaces; workspace design §3 and §5; docs/aylul/ui/specs/2026-10-19-agent-workspace-mockup/ (visual reference: agent-workspace-desktop.png, agent-workspace-settings.png); app/assistant/platform contracts.
- Touch: apps/desktop/src/features/assistant/{useAssistantConversation.ts,ConversationSidebar.tsx,ConversationView.tsx,AssistantPage.tsx,assistant.css,contract.yaml}; apps/desktop/src/app/{main.tsx,WorkspaceShell.tsx,workspace.css,contract.yaml}; apps/desktop/tests/companion/{workspace-conversations.spec.ts,fixture.tsx}; apps/desktop/tests/{assistant-fixture.tsx,assistant-state.mjs}; docs/aylul/codebase/README.md; docs/aylul/ui/stages.md.
- Needs: U-W1c
- Done when: from apps/desktop/, `npm run build && npx playwright test tests/companion/workspace-conversations.spec.ts tests/companion/workspace.spec.ts` and `node tests/assistant-state.mjs` pass; task committed.
- Also: U-W1 left two checks weak because its fixture is signed out. With a signed-in assistant fixture, `workspace_routes_account_to_settings_and_preserves_chat_draft` must type a draft and find it after Settings -> Chat, and `workspace_narrow_layout_preserves_navigation_and_composer_390` must assert the composer is visible.

### U-W3 · Scoped settings, bots list and assistant inspector
- Status: done
- Plan: docs/aylul/ui/plans/2026-10-19-agent-workspace-ui.md#task-3---expose-truthful-botsettings-surfaces
- Read: Task 3; workspace design §3, §5 (OPEN - Effective model readback); docs/aylul/ui/specs/2026-10-19-agent-workspace-mockup/ (visual reference: agent-workspace-desktop.png, agent-workspace-settings.png); character and app/assistant contracts; source settings inventory.
- Touch: apps/desktop/src/app/{SettingsPanel.tsx,main.tsx,WorkspaceShell.tsx,workspace.css,contract.yaml}; apps/desktop/src/features/assistant/{AssistantDetails.tsx,BotList.tsx,ConversationView.tsx,assistant.css,contract.yaml}; apps/desktop/tests/companion/{workspace.spec.ts,workspace-conversations.spec.ts,production.test.ts}; docs/aylul/codebase/README.md; docs/aylul/ui/stages.md.
- Needs: U-W2
- Also (found in U-W2's screenshot, design §3 and the mockup):
  - The composer is pinned to the bottom of the chat area; only the message history scrolls. Today it sits right under the last message.
  - Conversation header as in the mockup: bot mark, "AYLUL", its role line, and a small truthful connection indicator, not a "Connected" paragraph.
  - Messages as in the mockup: the person's message is a right-aligned bubble; AYLUL's reply is plain text under a small "AYLUL" label, not a full-width card. Arabic keeps automatic direction.
  - Composer as in the mockup: one rounded box with the textarea, a model chip reading "Model: managed by the account backend" (design §5 OPEN), and a round send icon button.
  - Sidebar Pin/Unpin becomes a small icon button (`pin`), with the same accessible name.
- Done when: from apps/desktop/, `npm run build && npx playwright test tests/companion/workspace.spec.ts tests/companion/workspace-conversations.spec.ts && npm run test:companion:production` passes; a 1248x948 screenshot of `/tests/companion/fixture.html?shell&signedIn` is compared with agent-workspace-desktop.png and the report lists what still differs; task committed.

### U-W4 · Batch gate and native journey
- Status: blocked (gate and compile pass; waits for Mohammed's hands-on journey on a rebuilt debug app, then whole-branch review)
- Plan: docs/aylul/ui/plans/2026-10-19-agent-workspace-ui.md#task-4---run-the-batch-gate-and-the-native-journey
- Read: Task 4; normal-chat evidence for the evidence format.
- Touch: apps/desktop/README.md; docs/evidence/aylul/<run date>-agent-workspace.md; docs/aylul/ui/stages.md.
- Needs: U-W3
- Done when: from apps/desktop/, `npm run build && npm test && npm run test:companion:unit && npm run test:companion:browser && npm run test:companion:production` passes; `cargo build --locked --manifest-path apps/desktop/src-tauri/Cargo.toml` succeeds; Mohammed's hands-on journey is recorded in the evidence file; whole-branch review done.

### H-2 · Present one account-backend conversation
- Status: done (Mohammed sent a message and got a reply in the desktop window on 2026-10-19; stop/recovery in the window not checked by hand)
- Plan: docs/aylul/native/plans/2026-10-18-aylul-local-runtime-hermes.md#h-2--one-main-window-backend-conversation
- Read: generated backend assistant contract and local/native shared request once accepted.
- Touch: H-2 feature/platform/test files and their actual contracts only.
- Needs: account-scoped backend facade and native authenticated HTTP (satisfied); no direct Hermes credentials.
- Done when: selected conversation, reply/recovery and stop states work with local monitoring and optional character preserved.

See [normal-chat evidence](../../evidence/aylul/2026-10-19-normal-chat.md) for the
real backend/MiniMax browser journey, separate native HTTP checks and explicit
remaining physical GUI gate. No avatar redesign, code bridge or Core change.
