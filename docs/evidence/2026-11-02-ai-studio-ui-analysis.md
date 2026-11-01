# AI Studio UI export: analysis against `web/src`

Dated 2026-11-02, branch `feature/aylul-agents-panel` at `42bd1621`. Facts, not decisions:
the migration plan [`docs/frontend/plans/2026-11-02-ai-studio-ui-migration.md`](../frontend/plans/2026-11-02-ai-studio-ui-migration.md)
decides what to do with them.

Subject: Google AI Studio's export at `E:\Globalprojects\ui-modernizer-nov` (outside the
repository). Its `src/` is our `web/src` at `dc2ae544`, restyled. Compared with
`diff -uw --strip-trailing-cr` against `git archive dc2ae544 web/src`: 37 files differ, no file
added or removed under `src/`. Outside `src/`: `server.ts` (a fake Express backend that
mirrors our `/api/*`), `public/assets/aistudio/` (screenshots), a `package.json` for AI Studio.

Four read-only analysts (two `frontend-builder`, two `frontend-fixer`), run by Masada in
parallel. None edited or committed anything; scratch overlays lived in the session's temp
folder. Their reports follow as returned, lightly reformatted.

---

## Report A · Inventory and features without backend (frontend-builder)

### Headline findings
- The export is almost entirely a restyle. The only large non-style additions are the Agent
  Mode board in `conversation.tsx`, the progress UI in `plan-page.tsx`, and the interactions
  in `inspect-panel.tsx`. The new data they show comes from existing hooks or is computed locally.
- No file under `src/` depends on `server.ts`, which mirrors our `/api/*` routes so AI Studio
  can run the UI.
- No `fetch` and no new URLs were added to components.
- Real problems: `daemon-origin.ts` changes how the API origin is chosen;
  `/projects/$projectId/design` is a route we do not have; many explanatory comments were
  deleted; `home.tsx` has an odd status check.
- Lucide: every icon name the export imports exists in our installed `lucide-react` ^1.47.0.
  `motion` and `@dagrejs/dagre` are already in `web/package.json`.
- `package.json` adds nothing we need: `@google/genai`, `dotenv`, `express`,
  `@types/express`, `autoprefixer`, `esbuild`, `tsx` all serve `server.ts` and AI Studio's build.

### 1. Inventory

Counts are +/− lines, whitespace ignored. S = style, M = markup/layout, B = behaviour,
F = new feature without backend.

| File | +/− | Kind | What changed |
|---|---|---|---|
| api/daemon-origin.ts | +6/−1 | B | Origin becomes `VITE_SHADOWS_URL`, else `location.origin` when not in test mode and the page is http, else `127.0.0.1:4318`. |
| app/agents/home.tsx | +191/−9 | M (F, S) | Hero, four-stage pipeline strip, agent grid, capability lists. |
| conversation/composer-bar.tsx | +31/−8 | S (M) | Tooltip wrappers, an `Add` aria-label. |
| conversation/composer.tsx | +5/−5 | S | Class strings. |
| conversation/context-ring.tsx | +67/−13 | M (S) | New `DonutGauge` and a popover with a usage bar, from `shown`/`limits` only. |
| conversation/conversation.tsx | +782/−8 | F (B, M) | Chat/Agent mode switch, `viewMode` state, a 720-line `AgentExecutionBoard`. |
| conversation/draft.tsx | +24/−11 | S (M) | Icons. |
| conversation/entry.tsx | +7/−10 | S | Bubble classes; three why-comments deleted. |
| conversation/map-proposal-card.tsx | +8/−8 | S | Classes. |
| conversation/messages.tsx | +21/−27 | S | Empty state with Sparkles, `max-w-4xl`; about 20 lines of why-comments deleted. |
| conversation/plan-card.tsx | +10/−5 | S | Classes. |
| conversation/plan-side-panel.tsx | +2/−2 | S | Classes. |
| conversation/roadmap-proposal-card.tsx | +6/−6 | S | Classes. |
| conversation/status-badge.tsx | +26/−8 | S (M) | Status dot, per-state colours; logic unchanged. |
| conversation/subagent-card.tsx | +22/−11 | S (M) | Icons and tones. |
| conversation/subagent-panel.tsx | +26/−18 | S (M) | Bot icon header. |
| conversation/todo-card.tsx | +47/−8 | M (S) | Percent bar computed from `items`; per-status styles. |
| conversation/tool-diff.tsx | +16/−10 | S | Diff tints. |
| conversation/tool-group.tsx | +59/−24 | S (M) | Busy-row highlight. |
| conversation/vision-proposal-card.tsx | +4/−4 | S | Classes. |
| daemon-status.tsx | +6/−6 | S | Glow, mono text. |
| design/workspace-page.tsx | +60/−10 | M | Header and tab links restyled; `Compass` icon. |
| desktop/activity.tsx | +11/−3 | M | On `capabilities.localCore` false, shows "Local activity is available in the desktop app." |
| home.tsx | +90/−8 | M (B) | Project cards with Open Planner and "Design & Map" links; empty state restyled. |
| settings/settings-dialog.tsx | +10/−6 | S | Active nav pill, sizes. |
| sidebar/agent-list.tsx | +54/−19 | B (S) | `filter` prop; client filter by name/description; auto-open while filtering. |
| sidebar/project-list.tsx | +57/−20 | B (S) | `filter` prop (name/directory); workspace link gets `activeProps`. |
| sidebar/sidebar.tsx | +109/−13 | B (M) | Search toggle and input, `filter` state, tooltips, motion backdrop, narrow-screen sidebar. |
| sidebar/thread-list.tsx | +7/−7 | S | Classes. |
| sidebar/workspace-switcher.tsx | +37/−5 | M (S) | New exported `NylraLeafMark` SVG logo. |
| workflows/inspect-panel.tsx | +127/−23 | B (F, M) | Local acceptance checkboxes with a progress bar, copy-path buttons, section badges. |
| workflows/layout.ts | +24/−2 | B | Staggered columns, below. |
| workflows/plan-edge.tsx | +35/−4 | S (M) | Glow path, endpoint dots, rounded corners. |
| workflows/plan-page.test.tsx | +18/−0 | test | New test for the progress bar. |
| workflows/plan-page.tsx | +212/−21 | F (B, M) | `planTaskProgress`, per-task completion toggle, header progress bar. |
| workflows/task-node.tsx | +17/−15 | S | Gradient nodes, glow. |
| index.css | +73/−39 | S | Below. |

**layout.ts.** dagre still runs left to right. `nodesep` 48 → 68, `ranksep` 160 → 172. After
layout, nodes are grouped by rounded x (columns). Every column with exactly one node, except
the lone `start` node, moves its y by −46, then +46, alternating: box above, box below, box
above. The edge fan-out step cap goes from 20 to 22. `plan-page.test.tsx` only appends one test
(`1 / 2 tasks`, `50%`, `aria-valuenow=50`; clicking `Toggle T2 completion` gives `2 / 2`, `100%`).

**index.css.** Palette moves from grey-indigo to violet-black (background `#0b0a0f` →
`#07050f`, accent `#534ab7` → `#7c3aed`, accent-line `#7f77dd` → `#a855f7`); danger, success
and the light theme retuned. `--radius` 0.625rem → 0.75rem. Fonts become "Plus Jakarta Sans"
and "JetBrains Mono" with no font files. The body gets a fixed radial-gradient background. New
utilities: `.nylra-panel`, `.nylra-card-glow`, `.nylra-pill-active`, `.nylra-grid-bg`. Many
components use hard-coded hex (`bg-[#140e28]`) and `purple-500/..` instead of tokens.

### 2. Features without backend

No task search exists in the export. The sidebar search is a client-side filter over data we
already fetch.

1. **Task completion toggle and progress bar** (`plan-page.tsx:20-110`, `:172-260`, `:301-311`).
   "n / m tasks", a percent and a bar; each toggle changes a local `Set<number>`. "Completed"
   is guessed: `completed`/`status`/`state` fields our `PlanTask` lacks, or a Frozen plan, or no
   blocker mentioning `T<n>` plus a goal and acceptance items. A status rule copied into the
   client; shows 100% on frozen plans. Proposal: remove the toggle and the guess; keep the bar
   only when Core exposes task status; until then a disabled "coming soon" placeholder.
2. **Acceptance checkboxes in the Inspect panel** (`inspect-panel.tsx:15-40`, `:101-130`).
   Local ticks with a "3/5" badge and bar, lost on close. Proposal: read-only list, or label it
   "Local checklist (not saved)". Copy-path buttons are fine.
3. **Agent Execution Board, "Agent Mode"** (`conversation.tsx:116`, `:155-215`, `:340-1037`).
   Its data is real (`entries`, `tools`, `todos`, `operations`, `session`, `context`, `limits`,
   subagents). Invented values: `:442` filler prompt; `:450` 35% progress while running with no
   steps; `:1015` context bar 12% with no figures; `:1026` "Active" as rate-limit text; the
   five-stage stepper (around `:410`) marked done from heuristics; the "Active Tools" matrix
   (`:955-990`) derived from step kinds, not the tool list. Terminal/Files/Logs tabs only
   filter real steps. Proposal: keep the board on real data, remove the fallbacks, mark the
   stepper and tool matrix "Preview".
4. **Agents home hero** (`agents/home.tsx:62-95`, `:180-210`). "Autonomous Workspace Ready",
   a "Ready" badge on every agent, a four-stage strip with Execute always active, static
   capability lists. Provider and agent counts and agent cards are real. Proposal: delete the
   readiness claims; present capabilities as a labelled static overview or drop them.
5. **Home project cards** (`home.tsx:95`, `:115`). "Workspace" label is static. The link
   `/projects/$projectId/design` is not a route; design views live under `/workspace`.
6. **Sidebar search** (`sidebar.tsx:22-35`, `:95-115`). Client-side filter over agents and
   projects (not threads). Fine.
7. **`home.tsx:18`** tests `status === 'Success' || (...)`; the first branch never matches.

---

## Report B · Structure against our architecture (frontend-fixer)

The export respects the transport boundary: no `fetch`, no URL building, no new react-query
keys or cache writes, no second stream. It adds client-invented rules and fake state in four
places, breaks the origin contract, and fails `typecheck` and `lint`.

### How it was checked (scratch overlay only)
- Overlay: base at `dc2ae544` with the export's `src` over it, our `node_modules` junctioned.
- `tsc -p tsconfig.app.json`: 9 real errors in 2 files.
- `oxlint --deny-warnings`: fails with 1 warning, `plan-page.tsx:61` `react(only-export-components)`.
- `vitest run src/codemap.test.ts src/architecture`: 82/82 pass.
- Vitest on api, conversation, sidebar, workflows, design, desktop, settings-dialog, aylul:
  433/433 pass (71 files).

### Blockers
1. **`api/daemon-origin.ts:16`** falls back to `location.origin` outside test mode. AI Studio
   needed it because `server.ts` serves Vite and a fake `/api` on :3000. It breaks the api
   contract lines `DAEMON_URL: "...; browser VITE_SHADOWS_URL or http://127.0.0.1:4318"` and
   `origin_setup`. In the browser, `npm run dev` on :5173 would call :5173 (no proxy). On the
   desktop, a Windows Tauri page (`http://tauri.localhost`) passes `startsWith('http')` before
   `setDaemonOrigin` runs, skipping the loopback validation. Fix: drop it; use `VITE_SHADOWS_URL`.
2. **`workflows/plan-page.tsx:27-81, 93, 306`**: `planTaskProgress`, `TaskStatusCandidate`,
   `COMPLETED_STATES` invent a task status machine (reads fields `PlanTask` lacks, treats
   Frozen as done, regex-matches blocker text); toggles are local only. Breaks AGENTS.md rule 0
   and the neon spec ("No artificial progress percentages"). Also the lint failure.
3. **`workflows/plan-page.test.tsx:403`** locks in item 2.
4. **`conversation/conversation.tsx:119, 323-1037`**, the `AgentExecutionBoard`: invented
   figures (35% at `:446-450`, 12% context at `:1015`, five-stage pipeline from heuristics at
   `:478`, "Interactive Agent Shell" at `:827`, filler prompt at `:442`); re-derives rules the
   contract gives to `Messages` (rows plus live steps; todos "never both", `:412`); calls
   `navigator.clipboard` directly (`:369`) instead of `app/copy-button.tsx`; type errors from
   fake shapes (`s.kind === 'command'` at `:397`, `op.context_used`/`op.context_window` at
   `:916-918`, `limits.fiveHour.percent` at `:1026`); the file grows 318 → 1037 lines with two jobs.
5. **`app/home.tsx:23` and `:115`**: `status === 'Success'` never matches;
   `/projects/$projectId/design` is not a route.
6. **`app/agents/home.tsx:69, 93-131, 153, 163, 183-225`**: fabricated runtime state and
   capabilities ("Autonomous Workspace Ready", Execute hard-set active, "Ready" on every agent,
   a made-up default description, static capability and memory lists with green dots). The
   restyled `Step` and the agent grid are real data.
7. **`workflows/inspect-panel.tsx:17, 101`**: clickable acceptance "checked" state and its bar,
   unpersisted, on an `li` with no keyboard role. Copy at `:34` should use `CopyButton`.

### Must-fix during migration
8. Colour literals in components (`bg-[#0d091a]`, `purple-500/…`, `emerald-…`, `rose-…`,
   `rgba` shadows) break `index.css` "the only place a colour value is written". `.nylra-*`
   (`index.css:217+`) hard-code `rgba`, which breaks "Light mode retains equivalent hierarchy".
9. `index.css:58-60` maps serif to "Plus Jakarta Sans" (removing "serif (the reply voice)") and
   names fonts it does not ship, against "Fonts never come from a CDN". HEAD already has the
   neon `index.css` (242984d8): merge 3-way against HEAD, never overwrite.
10. Removed why-comments: `messages.tsx` around `:106-127`, `entry.tsx:102`, `plan-page.tsx`
    around `:126` and `:148`.
11. `workflows/layout.ts:157-178`: the ±46 px stagger after dagre breaks the workflows contract
    trap "The box dagre spaced is the box drawn", with a `!` assertion and a magic `'start'` id.
    `nodesep`/`ranksep` changes are fine.
12. Sidebar filter (`sidebar.tsx:26`, `project-list.tsx:13`, `agent-list.tsx:15`) breaks the
    contract signature `ProjectList: "() => JSX.Element"`. Display-only filtering is fine;
    update signatures, empty text and tests together.
13. Duplicated list rules: `agents/home.tsx:48` filters `!a.deleted` (the sidebar does not);
    `home.tsx:54` repeats the sidebar's `assistant` filter.
14. `context-ring.tsx:78`: `DonutGauge` is a second ring beside `Ring`.

### Acceptable (class-only or display arithmetic over Core data)
`composer`, `composer-bar`, `draft` (except the invented label at `:117`), `entry`, the
map/roadmap/vision proposal cards, `plan-card`, `plan-side-panel`, `status-badge`,
`subagent-card`, `subagent-panel`, `tool-diff`, `daemon-status`, `settings-dialog`,
`thread-list`, `workspace-switcher`, `task-node`, `plan-edge`, `desktop/activity`,
`design/workspace-page`. `todo-card.tsx:29` computes a percent from real statuses (contract says
completed items are "text muted"; the export adds line-through). `tool-group.tsx:92` shows a
check for completed steps. Rows to update after migration: `AylulHome`, `home.tsx`, `TodoCard`,
`ToolGroup`.

### Tests
None of the existing tests fail on the overlay; queried strings survive. What fails is
`npm run typecheck` (9 errors) and `npm run lint` (1 warning). Re-check `entry.tsx` and the
`permission-card`/`agent-settings` neon work after the merge with HEAD.

---

## Report C · Code style and line formatting (frontend-builder)

Method: the export's `src/` copied to scratch with our `node_modules`; our `.oxlintrc.json` and
`tsc` run there.

**What we enforce.** `npm run lint` is `oxlint --deny-warnings` (react, typescript, oxc;
`react/rules-of-hooks` error, `react/only-export-components` warn, which fails under deny).
`npm run typecheck` is strict `tsc -b`. No prettier or editorconfig: line width is convention.

### (a) Measured (37 changed files)

| Metric | Base | Export |
|---|---|---|
| Lines | 4,658 | 6,643 (+43%) |
| Max line length | 431 | 406 |
| p95 / p99 line length | 95 / 133 | 103 / 163 |
| Lines over 100 / 120 chars | 172 / 61 | 367 / 195 |
| `className=` lines over 140 chars | 13 | 57 |
| `conversation.tsx` lines | 229 | 1,037 |
| Raw palette classes (`purple-*`, `emerald-*`, `text-white`) | few | hundreds (`purple-500/NN` about 150) |
| Arbitrary hex/rgba values | 16 | 194 |
| `text-[10px]` / `text-[11px]` | — | 12 / 41 |
| `aria-*` and `role=` | 95 | 171 |
| `nylra-panel` uses | 0 | 28 |

Both use 2-space indent, no semicolons, single quotes; the length gap comes from class strings.

**Lint and typecheck on the export.** oxlint: 1 warning (`plan-page.tsx:61`). tsc: 9 real
errors (`conversation.tsx:916/918` `op.context_used`/`op.context_window`; `:1026`
`limits.fiveHour.percent`; `:397` `'command'` not a step kind; `home.tsx:23` `'Success'`;
`home.tsx:115-116` unregistered route), plus 4 scratch artifacts.

### (b) Better than ours, adopt
- Accessibility: `aria-*`/`role` up from 95 to 171.
- Palette still defined once as `--palette-*` tokens; the violet values adopt as token values in
  both themes (light `--palette-accent` `#6d28d9`, needs a screenshot check).
- Radius 0.75rem: one token.
- Fonts as `--typeface-*` entries: cheap, but files must ship (`@fontsource`) or keep Geist.

### (c) Worse, normalise
1. Literal colours in markup → semantic tokens (`bg-card`, `border-border`, `text-accent-text`,
   `bg-accent-soft`, `border-accent-line/NN`, `text-success`, `text-danger-text`); a missing tint
   becomes one token in `index.css` first.
2. `.nylra-*` utilities are dark-only literals and bypass `html[data-theme="light"]`. Drop the
   brand name; rebuild from `var(--palette-*)`/`color-mix`, or add `--surface-panel`/`--glow`
   tokens in both themes. Body radial-gradient and `.nylra-grid-bg` the same. The
   `NYLRA · AYLUL Agent Mode` text (`agents/home.tsx:65`) is copy, flag it to the owner.
3. Repeated class strings (28 `nylra-panel rounded-2xl p-*`, the `rounded-xl border
   border-purple-500/20 bg-[#0d091a]/70 p-3` pattern, 7 × `text-xs font-semibold tracking-wider
   uppercase`): a string used 3+ times becomes a `cva` variant in `components/ui` or a small component.
4. `text-[10px]`/`text-[11px]` (53 uses) → `text-xs` or a `--text-2xs` token; `shadow-[...]`
   glows → a named `shadow-glow` token.
5. Wrap any JSX tag over 100 chars one prop per line; conditional classes in `cn(...)` or a
   `cva` variant.
6. Port only what existing hooks and types support; a missing field is a Core request.
7. Move `planTaskProgress` out of the component file (clears lint).
8. Brand/history comments (`/* 1. The palette — NYLRA · AYLUL Obsidian-Violet Neon Edition. */`)
   → why-comments or delete.
9. Take none of the export's `package.json` (lucide `^0.546` vs ours `^1.47`; motion `^12` vs
   `^13`; `@google/genai`, express, `server.ts`).

### (d) Proposed migration style rules
1. Restyle value from tokens only; raw palette classes, hex and rgba banned in `.tsx`.
2. Every new look becomes a token, then a `cva` variant; a string repeated 3+ times is a primitive.
3. Changed lines keep lint and typecheck at zero.
4. Wrap JSX over 100 chars; `cn()` for conditional classes.
5. Keep the added aria labels, focus-visible rings and reduced-motion handling; `motion-reduce:`
   on any new animation.
6. Screenshot dark and light for every restyled screen.
7. Comments say why; no brand names, no history.
8. A restyle touches markup and classes; logic, routes, status strings and fields stay ours.

---

## Report D · Conflicts with the newer branch work (frontend-fixer)

Only five commits touch `web/` since `dc2ae544` (27 files, +595/−45): c5882a94, 1e46ed7c,
f747f296, 242984d8, 37f614e4. Only two files overlap the export: `conversation/entry.tsx`
(f747f296) and `index.css` (242984d8).

### c5882a94 · MCP tools in the extension library
Files: `api/{extensions.ts, extensions.test.ts, contract.yaml}`, `settings/{extension-list.tsx,
mcp-form.tsx, mcp-form.test.tsx, plugins.test.tsx, sections/plugins.tsx, contract.yaml}`.
No overlap; base tokens only, no `neon-*`. They will look plainer than the restyled
`settings-dialog.tsx`. Keep HEAD. Proof: `extensions.test.ts`, `mcp-form.test.tsx`, `plugins.test.tsx`.

### 1e46ed7c · agent capabilities
Files: `api/{extensions.ts, contract.yaml}`, `agents/{agent-extensions.tsx,
agent-extensions.test.tsx, contract.yaml}`. No overlap. `agent-extensions.tsx` uses
`neon-surface` (from 242984d8). Keep HEAD. Proof: `agent-extensions.test.tsx`.

### f747f296 · permission card
Files: `api/{permissions.ts, permissions.test.ts, contract.yaml}`, `conversation/{entry.tsx,
permission-card.tsx, permission-card.test.tsx, contract.yaml}`, `stream/{refresh.ts,
refresh.test.tsx, contract.yaml}`. Overlap `entry.tsx`, hunks adjacent but disjoint:
- HEAD adds `import { PermissionCard } from './permission-card'` after the `subagentOf` import.
- HEAD adds three branches after `if (entry.kind === 'AgentMessage') …`: `PermissionAsked` →
  `<PermissionCard …/>`; `PermissionAnswered` → muted "… · by you"; `PermissionRefused` with
  `entry.card?.reason` → muted "… · {reason}".
- The export restyles the user bubble (about line 63), PlanApproved (emerald pill) and the later
  PermissionRefused block (amber), and deletes its why-comment ("Refused in the mode the turn ran in…").
Recipe: start from the export's `entry.tsx`, re-apply HEAD's import and three branches, restore
the comment, restyle the two lines to the export's emerald/amber pill treatment.
Proof: `permission-card.test.tsx`, `stream/refresh.test.tsx`, `api/permissions.test.ts`,
`conversation.test.tsx`.

### 242984d8 · Codex's neon hierarchy (replaced by AI Studio)
Files: `index.css`, `components/ui/button.tsx`, `agents/{agent-settings.tsx,
agent-form.test.tsx, outfit-picker.tsx, agent-extensions.tsx, contract.yaml}`,
`conversation/{permission-card.tsx, contract.yaml}`, and the neon-clarity spec, plan and F-68.
Overlap `index.css`: HEAD added `--palette-glow/-sheen/-warning/-primary-text`, `--warning`,
`--color-warning`, `--primary-foreground: var(--palette-primary-text)` and an `@layer components`
neon block; the export added the typefaces, `--radius: .75rem`, body gradients and `.nylra-*`.
Keep (not visual): `agent-settings.tsx` `initialValue`/`dirty` and the `Unsaved changes` status
line; its test "unsaved identity changes are visible and reverting clears the indicator"; the
`agents/contract.yaml` obligation for it (drop "The shared neon palette owns visual emphasis.");
the permission card's text "Waiting for your approval"; `button.tsx` `data-variant` (harmless).
Drop: HEAD's palette values, the neon `@layer components` block, the `presentation:` line in
`conversation/contract.yaml` (or reword). Recipe: export's `index.css` plus `--palette-warning`
(dark `#ffd384`, light `#845000`), `--warning`, `--color-warning`; `permission-card.tsx`
`neon-surface` → glow card, `neon-status` → Tailwind pill with `text-warning`;
`agent-extensions.tsx` `neon-surface` → panel; `outfit-picker.tsx` drop `neon-choice` or a
`has-checked:` glow. Proof: `agent-form.test.tsx`, `outfit-picker.test.tsx`,
`permission-card.test.tsx`, `agent-extensions.test.tsx`. The neon spec bans "artificial progress
percentages, model state, memory metrics" — relevant to the export's features.

### 37f614e4 · panel navigation
`web/src/test/native-presence.json` only, read by `app/desktop/companion-binding.test.tsx`,
`desktop/native-parity.test.ts` and `apps/desktop/src-tauri/src/presence/binding_tests.rs:351`.
No overlap. Keep HEAD; never take it from the export.

### Token and class gap (HEAD uses, export `index.css` lacks)

| Missing | Used by | Action |
|---|---|---|
| `--color-warning` / `--warning` / `--palette-warning` | `agent-settings.tsx`, `permission-card.tsx` | Re-add in both themes |
| `.neon-surface` | `permission-card.tsx`, `agent-extensions.tsx` | New panel/glow surface |
| `.neon-status` | `permission-card.tsx` | Tailwind pill + `text-warning` |
| `.neon-choice` | `outfit-picker.tsx` | Drop, or `has-checked:` glow |
| `--palette-glow`, `--palette-sheen`, `--palette-primary-text` | HEAD's neon CSS only | Drop |
| `[data-slot=button][data-variant]` selector | `button.tsx` attribute | Drop the CSS |

`font-serif` changes meaning: the export maps it to Plus Jakarta Sans, so `reply-text.tsx` and
`entry.tsx` lose the serif face. A design choice to confirm.

### Export risks outside the five commits
1. `api/daemon-origin.ts`: reject, keep HEAD.
2. `plan-page.tsx` completion heuristic and its test: strip.
3. `inspect-panel.tsx` local checkboxes; `conversation.tsx` agent view: review against Core.
4. Fonts: the export's `index.html` loads Plus Jakarta Sans and JetBrains Mono from the Google
   Fonts CDN. Local-first: use `@fontsource` (a new library dependency). Keep HEAD's
   `index.html` (it has the `dark` class and the Loading fallback the export dropped).
5. Never take the export's `package.json`, `vite.config.ts`, `tsconfig*` or `server.ts`.

### Safe merge order
1. `index.css` (export + warning tokens + local fonts). 2. Neon consumers (`permission-card`,
`agent-extensions`, `outfit-picker`), keeping the unsaved indicator. 3. Restyled components the
five commits never touched. 4. `entry.tsx` hand merge. 5. Workflows, styling only. 6.
`conversation.tsx` last. 7. Contracts, docs, gate. Keep HEAD's `daemon-origin.ts` and
`native-presence.json` throughout.

### U-8 and N-25
Neither Touch list names a `web/` file; nothing under `apps/desktop` uses `--palette` or
`neon-*`. Desktop reads `web/src/assets/outfits/`, `web/src/desktop`, `web/src/main.tsx` and
`web/src/test/native-presence.json`; the export changes none of them. Native tasks rewrite
`native-presence.json`: the migration must never touch it.

---

## Masada's own view

Masada read the export's `daemon-origin.ts`, `layout.ts`, `task-node.tsx`, `sidebar.tsx`,
`conversation.tsx` (header and board), `plan-page.tsx` (progress logic) and `index.css`.

**Agreed with the analysts.**
- `daemon-origin.ts` is AI Studio plumbing for `server.ts`: never ported.
- `planTaskProgress` is a client status machine: it reads fields `PlanTask` lacks, counts a
  Frozen plan as done, and regex-matches blocker text. It is not ported.
- The board in `conversation.tsx` uses real data, but it adds invented fallbacks (filler
  prompt, 35%) and reads fields Core does not send.
- Colours are literals almost everywhere. `.nylra-*` and the body gradient are dark-only.

**Disagreed.**
- **`layout.ts` (Report B, item 11).** The stagger moves dagre's own node
  (`graph.node(id).y`) before `node.position`, the handles and the edge map are read. So the
  drawn box is still the placed box, and edges follow it. Only single-task columns move, so no
  two boxes can overlap. Mohammed asked for this layout, and it is sound. It is kept as a named
  layout rule with tests and a reworded trap (F-75), not dropped. The `!` assertion is
  redundant (the column length is checked just before).
- **The Agent Mode board (Report B, item 4).** Mohammed wants it, so it is not removed. It is
  split into its own files, and the rules it copies from `Messages` move into one shared
  function in `activity-rows.ts`, called by both (F-76).
- **Brand copy (Report C, (c) 2).** "NYLRA · AYLUL" in visible text follows the export:
  Mohammed likes it, and his own reference screenshots carry the name. Only comments lose brand
  banners.
- **`lucide-react`.** It is already a dependency at ^1.47.0, so the approval changes nothing.

**What the analysts missed.**
- The Chat/Agent switch is two plain buttons with no `aria-pressed` and no radio-group role.
  The selected mode is shown by colour alone.
- `.nylra-pill-active` hard-codes `color: #f5f3ff`. In the light theme that puts near-white text
  on a pale violet fill, which is unreadable.
- The board's `copyText` starts a `setTimeout` that is never cleared on unmount. `CopyButton`
  already handles this.
- The `index.css` header still says "serif (the reply voice)", while the export maps serif to a
  sans face. The comment and the code disagree.
- The export's sidebar wraps long JSX one prop per line, which reads better than our long
  single lines. Adopted as a rule.
- No "task search" exists in the export. The search Mohammed asked for is the sidebar filter
  over projects and agents. Searching tasks in a plan is not built, so it is not in this plan.
