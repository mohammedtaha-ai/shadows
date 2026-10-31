# Collapsible sidebar and readable agent activity

Branch `feature/web-polish`. Two asks from the user, 2026-10-31, after using the app:

1. A button at the top of the sidebar hides it and shows it again.
2. What the agent does during a turn reads like Claude Code's own app: consecutive tool calls
   grouped into one line ("Read 3 files, ran 2 commands, edited a file +36", total time,
   chevron) that opens into numbered steps; a running step shows a spinner and its elapsed
   time; an edit opens into its diff with line numbers; the agent's task list is a checklist.

Part 1 is web only (F-52). Part 2 needs Core: the fields are asked in
[`docs/shared/requests/2026-10-31-tool-activity.md`](../../shared/requests/2026-10-31-tool-activity.md).
Shipped in PR #48: F-52 to F-56 are done, and their entries are in
`docs/frontend/stages.md` at `ce31036d`. The F-numbers below name those tasks.

## What the code does today

Read on 2026-10-31 at `4196c986`.

- `web/src/app/shell.tsx` (`Shell`, composition) always renders `<Sidebar />` beside
  `<main className="flex min-w-0 flex-1 flex-col">`. `web/src/app/sidebar/sidebar.tsx`
  (`Sidebar`, no props) is one `aside` of fixed `w-72` (288 px); its `header` holds only the
  `WorkspaceSwitcher`. Nothing hides it: at 375 px the main pane is left 87 px.
- `web/src/app/sidebar/index.ts` exposes `storedWorkspace` and `Sidebar`. The sidebar keeps
  per-browser choices in `localStorage` inside try/catch (`open-projects.ts`, `workspace.ts`).
- A `ToolCall` entry carries the tool's last title in `body` and `card: null`
  (`crates/core-turns/src/entries.rs`, `Collector.tools: Vec<(String, String)>`).
  `crates/shadows-agent/src/acp/mapping.rs` passes on `title`, `status` and Claude's tool name,
  and drops ACP `kind`, `locations`, `rawInput` and `content` (where an edit's diff is). ACP
  `plan` updates fall into `other => None`.
- No live frame reports a tool call; a running tool is invisible until its entry is written.
  The subscribe route's frames: `durable`, `caught-up`, `delta`, `subagent`, `turn-end`,
  `usage`, `options`, `commands`, `plan-show`, `lagged`, `fatal` (`schema.d.ts`, the
  `/api/subscribe` description).
- `web/src/stream/thread-stream.ts` folds frames into `StreamState` (`streaming`, `labels`,
  `subagents`, ...); `frames.ts` reads each frame and `readSubagentCard` reads a `Subagent`
  entry's untyped `card`.
- `web/src/app/conversation/messages.tsx` (`Messages`) filters `silent` entries, then draws
  one memoised `EntryRow` per entry; `entry.tsx` (`Entry`) draws a `ToolCall` as a wrench and
  `toolText(title) ?? title`, one line per call. `tool-text.ts` owns `toolText`, `toolTitle`,
  `isTool`, `silent`, `subagentOf` and `copyText`. F-51's obligation stands: a delta redraws
  the reply, not the thread (each row memoised, props equal while the entries list is).

## Decisions

### 1. The sidebar hides and shows

- **One state, held by the shell.** `useSidebarState()` (new, `app/sidebar/sidebar-state.ts`)
  returns `{ shown, narrow, toggle, hide }`. `Shell` calls it once and passes it down; no
  context, no second copy.
- **Wide (768 px and up).** `shown` starts from `localStorage` key
  `shadows.sidebar.collapsed` (`"true"` means hidden; anything else, or no storage, shown).
  `toggle` flips it and writes the key; read and write in try/catch like `workspace.ts`, so
  without storage the choice lasts in memory. Hidden, the sidebar is not rendered and the main
  pane takes the whole width.
- **Narrow (below 768 px, `window.matchMedia('(max-width: 767px)')`).** `shown` starts
  `false` whatever is stored, and narrow toggles never write the key. Shown, the sidebar lies
  over the content: `aside` `fixed inset-y-0 left-0 z-40 w-72 max-w-[85vw]`, with a backdrop
  button (`fixed inset-0 z-30 bg-black/40`, `aria-label="Close sidebar"`) that hides it.
  Changing the route's pathname (choosing a conversation, a plan, a page) or pressing Escape
  hides it. Crossing the 768 px line re-reads the rule for the new width.
- **Keys.** `Ctrl+B` (or `Cmd+B`) anywhere calls `toggle` and prevents the browser's default.
- **The button.** `SidebarToggle` (new, `app/sidebar/sidebar-toggle.tsx`): a ghost icon button
  with lucide `PanelLeft`, `aria-label` "Hide sidebar" or "Show sidebar", `aria-expanded`,
  `aria-controls="app-sidebar"` (the sidebar's `aside` carries `id="app-sidebar"`), and the tooltip "Hide sidebar (Ctrl+B)" / "Show sidebar
  (Ctrl+B)" through `components/ui/tooltip.tsx`. In the sidebar it sits at the right of the
  header, after the `WorkspaceSwitcher`. When the sidebar is hidden, `Shell` draws the same
  button floating at the top left of the main pane (`absolute top-2.5 left-2.5 z-20`, main
  becomes `relative`).
- **Focus.** Showing by the button or the key moves focus to the sidebar's own toggle; hiding
  by Escape or the backdrop moves it to the main pane's toggle.
- **The slide.** Hiding and showing slide, as Claude's own app does: on wide screens the
  `aside`'s width animates between `w-72` and 0 (content clipped, `overflow-hidden`) while the
  main pane widens with it; on narrow screens the overlay slides in from the left and the
  backdrop fades. Both use Motion (`motion/react`, already wrapped by `MotionConfig
  reducedMotion="user"` in `Shell`), about 200 ms, ease-out. With reduced motion it appears
  and disappears at once. Hidden, the aside is removed from the tree once the slide ends
  (`AnimatePresence`), so the tests' "aside is gone" still holds.
- No icon rail and no resizing (not in this plan).

### 2. Agent activity

**What Core gives** (the request, field by field): a `ToolCall` entry's `card` is a
`ToolCallCard { id, tool, kind, status, target, lines, added, removed, diff, started_at,
finished_at }` with `diff: DiffHunk[] | null`; a live `tool` frame `{ op, id, tool, kind,
title, target, lines, status, started_at }`; a live `todos` frame `{ op, entries }`; and a
`TodoList` entry at the end of a turn that had a list. The web reads them and draws them; it
works out no diff, no status and no target.

**Reading (stream and api, F-53).**
- `api/threads.ts` re-exports the generated `ToolCallCard`, `DiffHunk`, `DiffLine`,
  `TodoListCard`, `TodoItem`, `ToolLive`, `CallKind`, `CallStatus`, `DiffOp` and `TodoStatus`
  as aliases (`Schemas['...']`), never copying fields. `card` stays a free-form object in the
  schema; the readers narrow it by the entry's `kind`.
- `stream/frames.ts` gains `ToolFrame` and `TodosFrame` (camelCase, as the other frames),
  `parseTool`, `parseTodos`, and `readToolCard(value: unknown): ToolCallCard | null` and
  `readTodoList(value: unknown): TodoListCard | null`, which return `null` for a value they
  cannot read (no throw), as `readSubagentCard` does.
- `StreamState` gains `tools: Record<callId, ToolFrame>` (the latest frame per call) and
  `todos: Record<opId, TodoItem[]>` (the latest list per turn). Both are cleared on a break
  and on `lagged`, and kept past `turn-end`, like `subagents`: the view, not the stream,
  decides when an entry has taken a frame's place.

**Grouping (F-54).**
- A **group** is a maximal run of consecutive shown `ToolCall` entries with the same
  `operation_id` (silent ones are already filtered out, so `plan_show` never splits a run).
- Each member is a **step** (`ToolStep`, a view model in `activity-rows.ts`):
  `{ key, ordinal | null, tool, kind | null, title, sentence | null, target, lines, status,
  added, removed, diff, error, startedAt, finishedAt }`. A durable step comes from its entry and
  `readToolCard` (`card: null` gives `kind: null`, `status: 'completed'`, the title only);
  `sentence` is `toolText(title)` when it differs from the title (Shadows' own tools).
  `error` is the card's `error` (user ruling 2026-10-31: Core sends it on a `failed` card
  only); a live step has `error: null`, since the `tool` frame carries none.
- **Live steps.** The latest turn's `tool` frames whose `id` is not the `card.id` of any
  entry are live steps (status `pending` / `in_progress` / `completed` / `failed`). A
  durable `ToolCall` entry is final over every frame of its `id` (Hamoud's rule: a frame
  can repeat or lag). Core holds an ended call's entry until the turn's next entry of
  another kind or the turn's end, so a `completed` step is drawn from its last frame until
  its entry lands. If the last shown row is a group of that turn and no reply text is drawn,
  live steps join it; otherwise they form one group drawn after the reply, where
  `runningCards` are drawn today. Once the turn has ended a live step no longer spins (Core
  writes every unended call as `stopped` at the turn's end, so its entry follows).
- **The line.** A group of one step draws that step's line directly (no summary). A group
  of two or more draws a summary button: a chevron, the words, the total time, and, when a
  step failed, "· N failed". It is collapsed by default; while any step is pending or in
  progress it is open, shows a spinner and the elapsed time, and ticks once a second. When it
  finishes it closes again unless the person opened or closed it themselves.
- **The words** (`activity-text.ts`), joined with ", ", first letter upper case, in the order
  the kinds first appear in the group:

  | `kind` | One / many | Counts |
  |---|---|---|
  | `read` | "read a file" / "read N files" | distinct non-null `target`s, else calls |
  | `edit` | "edited a file" / "edited N files", then " +A −R" when any count is set | same; A and R summed |
  | `delete` | "deleted a file" / "deleted N files" | same |
  | `move` | "moved a file" / "moved N files" | same |
  | `search` | "searched once" / "searched N times" | calls |
  | `execute` | "ran a command" / "ran N commands" | calls |
  | `fetch` | "fetched a page" / "fetched N pages" | calls |
  | `think` | "thought" | calls (no number) |
  | `switch_mode` | "switched mode" | calls (no number) |
  | `other`, or `null` | "used a tool" / "used N tools" | calls |

- **Time.** Done: `finishedAt` of the last step minus `startedAt` of the first, as `duration`
  in `subagent-text.ts` already words it. Running: now minus the first `startedAt`. Without
  times (old entries), no time is shown.
- **A step** (opened group, numbered `1.`, `2.`, ...; or a lone step): an icon for its
  status (spinner for pending or in progress, a red `X` with "failed" for failed, a muted
  stop for stopped, none for completed), then `sentence` when set, else the verb of its kind
  (Read, Edit, Delete, Move, Search, Run, Fetch, Think, Switch mode) and `target` with `lines` as
  `(47–102)`, else the title; then ` +A −R` for an edit; then its own time.
- **A failed step's error** (user ruling 2026-10-31). Under the line of a `failed` step with
  a non-empty `error`, the text shows as is: small, in the destructive colour, monospace,
  `whitespace-pre-wrap break-words`, indented to the step's text, `dir="auto"`. It is always
  visible, with no expand, while it is at most 6 lines and 600 characters; a longer error
  shows its first 6 lines (CSS `line-clamp-6`) with a "Show all" / "Show less" button. A
  lone failed step shows it too. A failed step without `error` (an old entry or a live frame)
  shows only its "failed" mark. The error is not part of Copy's line.
- **Copy** on a group copies each step's line, one per line; on a lone step, its line (the
  title for a non-Shadows call, so "Copy on a tool line copies its title" still holds).
- **Fork and links.** The group row carries the fork point when its last member is the fork
  anchor. A link to `#entry-<ordinal>` of a member opens its group and scrolls to that step.
- **Rendering cost.** Grouping is computed once per entries list (`useMemo`), and the group
  row is memoised on stable props, so F-51's "a delta redraws the reply, not the thread"
  still holds.

**The edit's diff (F-55).** An edit step with `diff` opens (its own chevron) into
`ToolDiff`: monospace lines, added on a green tint with `+`, removed on a red tint with `−`,
context plain; two gutters of line numbers (old, new) counted from `old_start` / `new_start`;
no gutters when they are `null`. A gap between hunks is a `⋯` row. A hunk whose `path`
differs from the one before it starts with that path as a small heading (one call can edit
more than one file). With `diff: null` and a
count set: "Diff too long to show". No syntax colouring.

**The checklist (F-56).** `TodoCard` draws a list's items: a check (`CircleCheck`, accent)
for completed, with the text muted; a spinner (`LoaderCircle`) for in progress, with the text
in the normal colour; an empty circle (`Circle`) for pending. A heading "Tasks · D of N done".
While a turn runs, its live `todos` list draws as a card after the reply; once the entries
hold a `TodoList` entry of that `operation_id`, that entry draws it instead (never both). A
`TodoList` entry draws through `Entry`; an unreadable card shows its body.

## Files

| File | New / changed | Holds | Code-map row (job) |
|---|---|---|---|
| `web/src/app/sidebar/sidebar-state.ts` | new | `useSidebarState()`; private `readStored`, `writeStored` | "Whether the sidebar shows, kept per browser" |
| `web/src/app/sidebar/sidebar-toggle.tsx` | new | `SidebarToggle` | "Button that hides or shows the sidebar" |
| `web/src/app/sidebar/sidebar.tsx` | changed | `Sidebar` takes the state's props | unchanged |
| `web/src/app/sidebar/index.ts` | changed | also exposes `useSidebarState`, `SidebarToggle` | unchanged |
| `web/src/app/shell.tsx` | changed | calls the hook, passes it, draws the main-pane toggle | unchanged |
| `web/src/api/threads.ts` | changed | `ToolCallCard`, `DiffHunk`, `TodoListCard` aliases | unchanged |
| `web/src/stream/frames.ts` | changed | `ToolFrame`, `TodosFrame`, `TodoItem`, `parseTool`, `parseTodos`, `readToolCard`, `readTodoList` | unchanged |
| `web/src/stream/thread-stream.ts` | changed | `tool` and `todos` handlers; `StreamState.tools`, `.todos` | unchanged |
| `web/src/stream/index.ts` | changed | exposes the new types and readers | unchanged |
| `web/src/app/conversation/activity-rows.ts` | new | `ToolStep`, `Row`, `activityRows`, `liveSteps` | "Groups consecutive tool calls into rows" |
| `web/src/app/conversation/activity-text.ts` | new | `groupSummary`, `stepLine` | "Words a tool group with its steps" |
| `web/src/app/conversation/tool-group.tsx` | new | `ToolGroup` (memo), private `StepLine`, `useElapsed` | "A run of tool calls drawn as one line" |
| `web/src/app/conversation/tool-diff.tsx` | new | `ToolDiff` | "An edit's hunks drawn with line numbers" |
| `web/src/app/conversation/todo-card.tsx` | new | `TodoCard` | "The agent's task list drawn as a checklist" |
| `web/src/app/conversation/messages.tsx` | changed | draws rows: `EntryRow` or `ToolGroup`; live steps and live todos | unchanged |
| `web/src/app/conversation/entry.tsx` | changed | loses the `ToolCall` branch; gains `TodoList` | unchanged |

`copyText` (`tool-text.ts`) already returns `body` for a kind it does not know, so a
`TodoList` entry copies its Markdown list with no change there.

Component and hook signatures:

```ts
// sidebar-state.ts
useSidebarState(): { shown: boolean; narrow: boolean; toggle: () => void; hide: () => void }
// sidebar-toggle.tsx
SidebarToggle(props: { shown: boolean; onToggle: () => void; className?: string }): JSX.Element
// sidebar.tsx
Sidebar(props: { narrow: boolean; onToggle: () => void; onHide: () => void }): JSX.Element
//   rendered only while shown; `narrow` makes it the overlay with its backdrop
// activity-rows.ts
type ToolStatus = 'pending' | 'in_progress' | 'completed' | 'failed' | 'stopped'
type Row = { kind: 'entry'; entry: ThreadEntry } | { kind: 'tools'; key: string; steps: ToolStep[] }
activityRows(shown: readonly ThreadEntry[]): Row[]
liveSteps(frames: readonly ToolFrame[], entries: readonly ThreadEntry[], op: string): ToolStep[]
// activity-text.ts
groupSummary(steps: readonly ToolStep[]): string
stepLine(step: ToolStep): { text: string; counts: string | null }
// tool-group.tsx
ToolGroup(props: { steps: readonly ToolStep[]; ordinals: string; forkProjectId?: string;
  forkThreadId?: string; forkEntryId?: string }): JSX.Element
//   `ordinals` is the members' ordinals joined, a stable string key for memo
// tool-diff.tsx
ToolDiff(props: { diff: readonly DiffHunk[] | null }): JSX.Element
// todo-card.tsx
TodoCard(props: { items: readonly TodoItem[] }): JSX.Element
```

## Tasks

All five tasks are done at `ce31036d`: F-52 the sidebar hides and shows; F-53 the stream and
the api read tool cards and `tool` / `todos` frames; F-54 tool calls drawn as numbered steps;
F-55 an edit step opens into its diff; F-56 the agent's checklist. The Core side is Hamoud's
(B-19 to B-22, in `docs/backend/stages.md` at `ce31036d`).

## Open questions

- **OPEN** Narrow screens keep no stored choice (the sidebar always starts hidden below
  768 px). Closes when the user confirms or asks otherwise.

## Not in this plan

- Showing a `Read`'s file contents or a command's output. Only an edit's diff is kept.
- Syntax colouring in a diff.
- An icon rail or a resizable sidebar.
- Grouping a subagent's own tool calls (they stay in its card's steps).
