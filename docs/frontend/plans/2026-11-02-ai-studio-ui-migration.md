# AI Studio UI migration: frontend plan

Branch `feature/aylul-agents-panel` (Mohammed, 2026-11-02). Moves the web design Google AI
Studio built (export at `E:\Globalprojects\ui-modernizer-nov`, outside the repository) into
`web/src`, without losing the five web commits made since the export's base `dc2ae544`, and
without breaking the client's rules. The facts this plan rests on are in
[`docs/evidence/2026-11-02-ai-studio-ui-analysis.md`](../../evidence/2026-11-02-ai-studio-ui-analysis.md)
(four analyst reports and Masada's own reading).

The visual direction is owned by [`docs/frontend/specs/2026-11-01-neon-clarity.md`](../specs/2026-11-01-neon-clarity.md).
Mohammed chose AI Studio's look over Codex's neon hierarchy (242984d8, F-68). Masada amends
that spec in place before F-69 starts (step M-0 below); this plan does not decide the look.

## Decisions Mohammed gave

- He likes everything AI Studio built. Its look replaces F-68's palette and neon surfaces.
- Features AI Studio built without a backend stay visible in the UI, clearly marked as
  planned. Their backend comes later.
- `lucide-react` is approved. It is already in `web/package.json` (^1.47.0), and every icon
  the export imports exists there, so no dependency change is needed for it.

## Rules every task keeps

1. **Never taken from the export:** `src/api/daemon-origin.ts` (it falls back to
   `location.origin`, which skips the desktop's loopback check and breaks `npm run dev`; it was
   only there to reach AI Studio's fake `server.ts`), `server.ts`, `package.json`,
   `vite.config.ts`, `tsconfig*`, `index.html`, `public/assets/aistudio/`, and
   `web/src/test/native-presence.json` (native tasks rewrite it). The export's files are read and
   ported by hand, never copied over ours: our HEAD has moved since `dc2ae544`.
2. **Colours are tokens.** No hex, `rgba`, or raw Tailwind palette class (`purple-500/20`,
   `emerald-300`, `text-white`, `bg-[#0d091a]`) in a `.tsx`. A missing tint becomes one token in
   `index.css`, defined in the dark block and in `html[data-theme="light"]`. Glows are one named
   shadow token. `text-[10px]`/`text-[11px]` become `text-xs` or one `--text-2xs` token.
3. **A class string used three times is a primitive:** the F-70 `Panel` variants, or an
   existing `components/ui` variant.
4. **Line width:** a JSX tag over 100 characters puts one prop per line (the export's sidebar
   does this, and it reads better than our long lines). Conditional classes go through `cn()`.
5. **Show Core's answer; invent nothing.** No percentage, count, status, "Ready" or stage that
   Core does not send. Where the design shows such a thing, it sits inside the F-70 `Planned`
   marker with no computed value: a dash, an empty bar, a disabled control. Never a guess,
   never a sample value. AGENTS.md rule 0 holds.
6. **Keep what AI Studio added for accessibility** (labels, tooltips, roles, Escape handling),
   and add what it missed: the Chat/Agent switch needs `aria-pressed` (or a radio group), and
   any new animation gets `motion-reduce:`.
7. **Comments say why.** Restore every why-comment the export deleted (`messages.tsx`,
   `entry.tsx`, `plan-page.tsx`; listed in the evidence). Brand banners such as
   `NYLRA · AYLUL Signature Neon Surface Utilities` become why-comments or go.
8. **Visible copy follows the export** (including "NYLRA · AYLUL Agent Mode" and the leaf mark),
   except copy that claims a runtime state Core does not report.
9. **Every visual task compares screenshots.** The builder screenshots the screen it changed, in
   dark and light, and compares with the export. The references are
   `E:\Globalprojects\ui-modernizer-nov\public\assets\aistudio\*.png`, plus the export run
   locally (`npm install` then `npm run dev` in that folder, port 3000, fake data) when a screen
   has no reference picture. Screenshots stay in the session's scratch folder and are attached to
   the report, never committed.
10. **Never touched by these tasks:** `apps/`, `crates/`, `api/`, `web/src/desktop/`,
    `web/src/main.tsx`, `web/src/assets/outfits/`, `web/src/test/native-presence.json`.

## Order against the branch's open work

U-8 (`apps/desktop/src/…`) and N-25 (`apps/desktop/src-tauri/…`) touch no `web/` file and use no
web token; the companion panel has its own `panel.css`. The migration therefore neither blocks
nor breaks them, and they can run first or alongside. The one shared file,
`web/src/test/native-presence.json`, is never touched here. Runs are serialized: one build at a
time.

F-69 → F-70 → F-71, F-72, F-73, F-74, F-75 (any order) → F-76 → F-77.

## M-0 · Amend the owner spec (Masada, before F-69)

Amend `docs/frontend/specs/2026-11-01-neon-clarity.md` in place: AI Studio's palette, radius,
type and surfaces replace the neon hierarchy; planned features may show, inside the `Planned`
marker, with no invented value (this replaces the blanket ban on "artificial progress
percentages, model state, memory metrics or runtime capabilities", which now reads: none is
computed in the client); fonts ship locally. Mark the neon-clarity plan superseded by this one.

## Shared requests (Masada writes them; the UI does not wait)

Each `Planned` block names the request it waits for. Written in `docs/shared/requests/` from
`TEMPLATE.md` before any backend work, after F-77:
- task status on a plan task (`PlanTask` has none): the plan progress bar and toggles (F-75);
- saved acceptance ticks per task: the inspect checklist (F-75);
- run stages of an agent turn: the board's stepper (F-76);
- context use per operation: the board's context figures beyond the turn summary (F-76).

## Tasks

### F-69 · Palette, type and surfaces from the AI Studio design
- Read: export `src/index.css`; ours `web/src/index.css` at HEAD; evidence, Report D
  "242984d8" and "Token and class gap".
- Touch: web/src/index.css, web/package.json, web/package-lock.json,
  web/src/app/conversation/permission-card.tsx, web/src/app/agents/agent-extensions.tsx,
  web/src/app/agents/outfit-picker.tsx, web/src/app/agents/agent-settings.tsx,
  web/src/app/agents/contract.yaml, web/src/app/conversation/contract.yaml, docs/frontend/stages.md
- Needs: M-0 done; Mohammed approves `@fontsource/plus-jakarta-sans` and
  `@fontsource/jetbrains-mono` (libraries, the font files; no CDN).
- Work: start from HEAD's `index.css`. Take the export's palette values for both themes,
  `--radius: .75rem` and its typefaces (served by the two packages; serif follows the export
  and becomes the sans face; the header comment says so). Keep `--palette-warning`, `--warning`
  and `--color-warning` (HEAD's values or matching ambers). Drop HEAD's neon block and
  `--palette-glow/-sheen/-primary-text`. Rebuild the export's four utilities as `surface-panel`,
  `surface-glow`, `pill-active`, `grid-backdrop` from `var(--palette-*)` and `color-mix` so they
  hold in light; the body gradients the same. Port the neon consumers: `permission-card`
  (`neon-surface` → `surface-glow`; `neon-status` → a pill with `text-warning`, its text
  "Waiting for your approval" kept); `agent-extensions` (`neon-surface` → `surface-panel`);
  `outfit-picker` (`neon-choice` → a `has-checked:` glow on tokens). Keep `agent-settings`'s
  `dirty` logic and its "Unsaved changes" line. Drop the agents contract sentence "The shared
  neon palette owns visual emphasis." and reword the conversation contract's `presentation:` line.
- Done when: from `web/`, `npx vitest run src/app/agents/agent-form.test.tsx
  src/app/agents/outfit-picker.test.tsx src/app/agents/agent-extensions.test.tsx
  src/app/conversation/permission-card.test.tsx` passes, then `npm run typecheck`,
  `npm run lint`, `npm run build` and `npx vitest run src/codemap.test.ts src/architecture`
  pass; the home and agent form screenshots in dark and light are compared with the export.

### F-70 · Shared primitives: `Panel` and the `Planned` marker
- Read: `docs/frontend/codebase/README.md` invariant "`components/ui/` ... knows nothing of
  Shadows"; `web/src/components/ui/button.tsx` (the `cva` pattern); evidence, Report C (c) 3.
- Touch: web/src/components/ui/panel.tsx (new), web/src/components/ui/planned.tsx (new),
  web/src/components/ui/planned.test.tsx (new), docs/frontend/codebase/README.md,
  docs/frontend/stages.md
- Needs: F-69 done
- Work: `Panel` is a `cva` box with `tone` (`panel`, `glow`, `inset`) and `size` (`sm`, `md`,
  `lg`), on F-69's surfaces; it replaces the export's repeated `nylra-panel rounded-2xl p-*` and
  `rounded-xl border … p-3` strings. `Planned` wraps a block that has no backend yet: a visible
  "Planned" badge, an optional `reason` line, its children `inert` and dimmed. It takes no
  value props: what it wraps is static.
- Done when: from `web/`, `npx vitest run src/components/ui/planned.test.tsx` passes (the badge
  and reason are shown; a button inside cannot be focused or clicked; its region has an
  accessible name), then `npm run typecheck`, `npm run lint`, `npm run build` and
  `npx vitest run src/codemap.test.ts src/architecture` pass.

### F-71 · Conversation cards and entries
- Read: `web/src/app/conversation/contract.yaml`; export files of the same names; evidence,
  Report D "f747f296" (the `entry.tsx` merge) and Report B items 10 and "Acceptable".
- Touch: web/src/app/conversation/entry.tsx, web/src/app/conversation/messages.tsx,
  web/src/app/conversation/plan-card.tsx, web/src/app/conversation/plan-side-panel.tsx,
  web/src/app/conversation/map-proposal-card.tsx,
  web/src/app/conversation/roadmap-proposal-card.tsx,
  web/src/app/conversation/vision-proposal-card.tsx, web/src/app/conversation/status-badge.tsx,
  web/src/app/conversation/subagent-card.tsx, web/src/app/conversation/subagent-panel.tsx,
  web/src/app/conversation/todo-card.tsx, web/src/app/conversation/tool-diff.tsx,
  web/src/app/conversation/tool-group.tsx, web/src/app/conversation/contract.yaml,
  docs/frontend/codebase/README.md, docs/frontend/stages.md
- Needs: F-70 done
- Work: port the export's markup and look under the rules above. `entry.tsx`: start from the
  export, re-apply HEAD's `PermissionCard` import and its three permission branches, restore the
  PermissionRefused why-comment, and give `PermissionAnswered` and `PermissionRefused`-with-reason
  the export's success and warning pill treatment. `messages.tsx`: restore its deleted comments.
  `todo-card`'s percent from real items stays; the contract's TodoCard and ToolGroup lines are
  updated for line-through and the completed check.
- Done when: from `web/`, `npx vitest run src/app/conversation/conversation.test.tsx
  src/app/conversation/permission-card.test.tsx src/app/conversation/waiting-messages.test.tsx
  src/app/conversation/todo-card.test.tsx src/app/conversation/tool-group.test.tsx
  src/app/conversation/tool-diff.test.tsx src/app/conversation/subagent-card.test.tsx
  src/app/conversation/map-proposal-card.test.tsx
  src/app/conversation/roadmap-proposal-card.test.tsx
  src/app/conversation/plan-in-conversation.test.tsx src/stream/refresh.test.tsx` passes, then
  `npm run typecheck`, `npm run lint`, `npm run build` and
  `npx vitest run src/codemap.test.ts src/architecture` pass; a conversation with a permission
  card, a tool group and a todo card is screenshot in both themes and compared.

### F-72 · Composer, draft and context ring
- Read: `web/src/app/conversation/contract.yaml` (Composer, ContextRing); export files of the
  same names; evidence, Report B items 14 and "Acceptable" (`draft.tsx:117`).
- Touch: web/src/app/conversation/composer.tsx, web/src/app/conversation/composer-bar.tsx,
  web/src/app/conversation/draft.tsx, web/src/app/conversation/context-ring.tsx,
  web/src/app/conversation/context-ring.test.tsx, web/src/app/conversation/contract.yaml,
  docs/frontend/stages.md
- Needs: F-70 done
- Work: port the look, tooltips and labels. The context ring keeps the one `Ring` (no second
  `DonutGauge`) and gains the export's popover with its usage bar, from `shown` and `limits` only.
  Drop the invented label at the export's `draft.tsx:117`.
- Done when: from `web/`, `npx vitest run src/app/conversation/composer-bar.test.tsx
  src/app/conversation/composer-focus.test.tsx src/app/conversation/draft.test.tsx
  src/app/conversation/context-ring.test.tsx` passes (a new case: the popover shows the figures
  Core gave and nothing when it gave none), then `npm run typecheck`, `npm run lint`,
  `npm run build` and `npx vitest run src/codemap.test.ts src/architecture` pass; screenshots
  compared.

### F-73 · Sidebar, with its search filter
- Read: `web/src/app/sidebar/contract.yaml`; export `sidebar.tsx`, `agent-list.tsx`,
  `project-list.tsx`, `thread-list.tsx`, `workspace-switcher.tsx`; evidence, Report B item 12.
- Touch: web/src/app/sidebar/sidebar.tsx, web/src/app/sidebar/agent-list.tsx,
  web/src/app/sidebar/project-list.tsx, web/src/app/sidebar/thread-list.tsx,
  web/src/app/sidebar/workspace-switcher.tsx, web/src/app/sidebar/sidebar.test.tsx,
  web/src/app/sidebar/project-list.test.tsx, web/src/app/sidebar/contract.yaml,
  docs/frontend/codebase/README.md, docs/frontend/stages.md
- Needs: F-70 done
- Work: port the look, the search toggle and input, the tooltips and the narrow-screen backdrop
  (with reduced motion). The filter only narrows lists already fetched; it is display, not a
  Core rule. `ProjectList` and `AgentList` take `filter`; the contract signatures say so. The leaf
  mark is exported as `LeafMark`.
- Done when: from `web/`, `npx vitest run src/app/sidebar/sidebar.test.tsx
  src/app/sidebar/project-list.test.tsx src/app/sidebar/workflow-list.test.tsx` passes (new
  cases: typing filters projects by name and directory, and agents by name and description; no
  match shows the empty line; Escape closes and clears), then `npm run typecheck`,
  `npm run lint`, `npm run build` and `npx vitest run src/codemap.test.ts src/architecture`
  pass; screenshots compared.

### F-74 · Home pages, design header, settings dialog, status lines
- Read: export `app/home.tsx`, `app/agents/home.tsx`, `app/design/workspace-page.tsx`,
  `app/desktop/activity.tsx`, `app/daemon-status.tsx`, `app/settings/settings-dialog.tsx`;
  their folders' `contract.yaml`; evidence, Report A features 4, 5, 7 and Report B items 5, 6, 13.
- Touch: web/src/app/home.tsx, web/src/app/agents/home.tsx, web/src/app/design/workspace-page.tsx,
  web/src/app/desktop/activity.tsx, web/src/app/daemon-status.tsx,
  web/src/app/settings/settings-dialog.tsx, web/src/app/settings/sections/plugins.tsx,
  web/src/app/settings/extension-list.tsx, web/src/app/settings/mcp-form.tsx,
  web/src/app/agents/aylul.test.tsx, web/src/app/agents/contract.yaml,
  web/src/app/settings/contract.yaml, web/src/app/design/contract.yaml,
  web/src/app/desktop/contract.yaml, docs/frontend/codebase/README.md, docs/frontend/stages.md
- Needs: F-70 done
- Work: port the look. `home.tsx`: keep HEAD's status condition (the export's `'Success'` never
  matches) and HEAD's `assistant` rule; the "Design & Map" link goes to
  `/projects/$projectId/workspace`. `agents/home.tsx`: the agent grid and the counts are real and
  stay; no "Ready" badge and no "Autonomous Workspace Ready"; the stage strip and the capability
  and memory lists sit inside `Planned`; the list shows what the sidebar shows (no second
  `deleted` rule). The settings library sections (HEAD's MCP work) take the same panels and pills
  so they match the restyled dialog, classes only.
- Done when: from `web/`, `npx vitest run src/app/agents/aylul.test.tsx
  src/app/settings/settings-dialog.test.tsx src/app/settings/plugins.test.tsx
  src/app/settings/mcp-form.test.tsx src/app/desktop/native-sections.test.tsx` passes (a new
  `aylul.test.tsx` case: the agents home shows each agent with its provider and model, and the
  planned blocks are marked and inert), then `npm run typecheck`, `npm run lint`,
  `npm run build` and `npx vitest run src/codemap.test.ts src/architecture` pass; screenshots
  compared.

### F-75 · Plan map: staggered columns, nodes, edges, page, inspect panel
- Read: `web/src/app/workflows/contract.yaml` (the "box dagre spaced" trap); export `layout.ts`,
  `task-node.tsx`, `plan-edge.tsx`, `plan-page.tsx`, `inspect-panel.tsx`; evidence, Report A
  features 1 and 2, Report B items 2, 3, 7, 11, and Masada's own view.
- Touch: web/src/app/workflows/layout.ts, web/src/app/workflows/layout.test.ts,
  web/src/app/workflows/task-node.tsx, web/src/app/workflows/plan-edge.tsx,
  web/src/app/workflows/plan-page.tsx, web/src/app/workflows/plan-page.test.tsx,
  web/src/app/workflows/inspect-panel.tsx, web/src/app/workflows/contract.yaml,
  docs/frontend/stages.md
- Needs: F-70 done
- Work: take the stagger as a layout rule: `nodesep` 68, `ranksep` 172, fan-out step 22, and
  each single-task column after the start moves up, then down, by one named constant, applied to
  dagre's node before positions and edges are read (so the drawn box is still the placed box).
  No `!` assertion. Contract trap updated. `task-node` and `plan-edge` take the look. In
  `plan-page`, the progress header and per-task toggles sit inside `Planned`: the total
  task count is real, done shows "—", toggles are disabled, and `planTaskProgress`,
  `COMPLETED_STATES` and the export's progress test are not ported. In `inspect-panel`, the
  acceptance list shows as a list with disabled ticks inside `Planned`; copy buttons use
  `app/copy-button.tsx`; the section badges stay. Restore `plan-page.tsx`'s deleted comments.
- Done when: from `web/`, `npx vitest run src/app/workflows/layout.test.ts
  src/app/workflows/plan-page.test.tsx src/app/workflows/plan-graph.test.tsx
  src/app/workflows/plan-map.test.tsx src/app/workflows/plan-frames.test.tsx` passes (new
  layout cases: single-task columns alternate above and below, a column with two tasks and the
  start do not move, no two boxes overlap, every edge ends on its drawn box; a plan-page case:
  the progress block is marked planned, shows the real count and no percent), then
  `npm run typecheck`, `npm run lint`, `npm run build` and
  `npx vitest run src/codemap.test.ts src/architecture` pass; a plan of five or more tasks is
  screenshot in both themes and compared.

### F-76 · Agent Mode: the execution board beside the chat
- Read: `web/src/app/conversation/contract.yaml` (Messages: rows, live steps, todos "never
  both"); `web/src/app/conversation/activity-rows.ts`, `usage.ts`, `turn-state.ts`; export
  `conversation.tsx` lines 100-1037; evidence, Report A feature 3, Report B item 4, Report C tsc
  errors.
- Touch: web/src/app/conversation/conversation.tsx, web/src/app/conversation/agent-board.tsx
  (new), web/src/app/conversation/agent-console.tsx (new),
  web/src/app/conversation/agent-board.test.tsx (new), web/src/app/conversation/activity-rows.ts,
  web/src/app/conversation/activity-rows.test.ts, web/src/app/conversation/messages.tsx,
  web/src/app/conversation/index.ts, web/src/app/conversation/contract.yaml,
  docs/frontend/codebase/README.md, docs/frontend/stages.md
- Needs: F-71 and F-72 done
- Work: `conversation.tsx` keeps only the header with the Chat/Agent switch (`aria-pressed`) and
  chooses the view. `agent-board.tsx` holds the board (prompt, steps, todos, subagents, model,
  context); `agent-console.tsx` holds its Diff, Terminal, Files and Logs tabs. "Written rows plus
  live steps" and "the live todo list, else the last written one" become one function each in
  `activity-rows.ts`, called by both `Messages` and the board. Only real fields: no filler
  prompt, no 35% or 12% fallback, no "Active" limit text, no `context_used`, no
  `limits.fiveHour.percent`; terminal steps are `execute`. Copy uses `CopyButton`. The
  five-stage stepper and the Active Tools matrix sit inside `Planned` with no done marks.
- Done when: from `web/`, `npx vitest run src/app/conversation/agent-board.test.tsx
  src/app/conversation/activity-rows.test.ts src/app/conversation/conversation.test.tsx`
  passes (the switch shows the board and back; the board lists the turn's real steps and todos;
  with no steps it shows an empty state and no percent; the stepper is marked planned and inert;
  Messages draws the same rows as before), then `npm run typecheck`, `npm run lint`,
  `npm run build` and `npx vitest run src/codemap.test.ts src/architecture` pass; a running and a
  finished turn are screenshot in Agent Mode in both themes and compared with the export's
  Agent Mode pictures.

### F-77 · The whole web client, once, after the migration
- Read: this plan; `docs/frontend/AGENTS.md` rules.
- Touch: docs/frontend/stages.md (status only); any fix goes to `frontend-fixer` as its own task.
- Needs: F-69 to F-76 done
- Work: run the whole web suite once, and walk every screen in the browser against a running
  `shadows serve`, in dark, light and reduced motion. Grep the changed `.tsx` files for hex,
  `rgba`, raw palette classes and `text-[1` and report each hit.
- Done when: from `web/`, `npm test`, `npm run typecheck`, `npm run lint` and `npm run build`
  pass; the colour grep reports nothing; screenshots of home, sidebar, conversation (both
  modes), plan page, agents home and settings, in both themes, are attached.
