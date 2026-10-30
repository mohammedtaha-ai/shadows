# Frontend stages

Where frontend work is now. Masada keeps this file; builders read their task
here and set its status. Task shape: [`AGENTS.md`](./AGENTS.md#a-task-in-stagesmd).

## Now

Open: F-52 (todo, web only). F-53 to F-56 are blocked on the shared request
`docs/shared/requests/2026-10-31-tool-activity.md` (asked 2026-10-31, Hamoud's side). Still
open outside any task: the light palette and the real desktop login-item behaviour need
their own app and OS acceptance.

Finished work: F-45 to F-49 (W-11, with B-14 to B-18, PR #47) are in this file at `4f23c717`;
F-28 (each settings form saves only the fields it changed, with the glass
theme selector, PR #46) is in this file at `e95b79c3`; F-22 (the context ring shows the summary only, PR #45) is in this file at
`29ddfd88`; F-20, F-21 and F-33 to F-44 (W-9, W-10, W-12 and W-13, PR #44) are in this
file at `4d5c022d`; F-16, F-17, F-26, F-27 and F-29 to F-32 (W-1 to W-8, PR #43) at
`ca618f89`; F-1 and F-23 to F-25 (PR #41) at `c7e7bac4`; F-2 to F-12, F-14 and F-15 (the
settings dialog, PR #40) at `1a2b601`.

## Tasks

Branch `feature/web-polish`: fixes and small improvements the user finds while using the app.

### F-50 · The message box keeps the caret after Send
- Status: done
- Plan: none; a bug found in use, 2026-10-30
- Read: `web/src/app/conversation/contract.yaml` (Composer, useSend)
- Touch: `web/src/app/conversation/composer.tsx`, its test file, `web/src/app/conversation/contract.yaml`
- Needs: none
- Found: `composer.tsx` sets `disabled={inFlight}` on the textarea. A disabled field loses focus
  to `BODY` and gets no focus back when it is enabled again, so after every Send or queue the
  person must click the box to type. Measured in the browser: right after Enter,
  `document.activeElement` is `BODY` and the box is disabled; after the send, the box is
  enabled and focus is still `BODY`.
- Done when: a test sends with Enter while the box has focus and finds the box still focused
  (and not editable while the send is in flight) once the send settles, for both a send and a
  queued message; `npx vitest run src/app/conversation` passes.

### F-51 · A streamed reply shows as it arrives, without freezing the page
- Status: done
- Plan: none; a bug found in use, 2026-10-30
- Read: `web/src/stream/contract.yaml` (delta), `web/src/app/conversation/contract.yaml`
  (Messages, Entry, PlanCard)
- Touch: files under `web/src/app/conversation/` and `web/src/stream/` that the cause lies in,
  their tests and their `contract.yaml`; report before touching anything else
- Needs: none
- Found: Core is not the cause. Recorded on `/api/subscribe` for one Planner reply: 100 `delta`
  frames over 8.8 s, the largest gap 0.14 s. In the browser on the same thread (a long
  conversation holding a plan card), the visible text went 14 → 54 characters, then one long
  task blocked the main thread for 4549 ms, then the text jumped to 786. Other long tasks:
  317 ms at send, 126 ms, 175 ms at the end. Find what one render does for 4.5 s while
  deltas arrive (profile it; do not guess), and fix that cause.
- Done when: a test drives many deltas into a thread that holds a plan card and long markdown
  and proves the cause is gone (for example the expensive part renders once, not per delta);
  `npx vitest run src/app/conversation src/stream` passes; and in the browser a reply on that
  thread grows steadily with no long task over 200 ms.

### F-52 · The sidebar hides and shows: a button, Ctrl+B, an overlay below 768 px
- Status: todo
- Plan: docs/frontend/plans/2026-10-31-shell-and-activity.md#1-the-sidebar-hides-and-shows
  (and its "Files" table for signatures and code-map rows)
- Read: `web/src/app/sidebar/contract.yaml` (Sidebar, WorkspaceStore, the try/catch rule);
  `docs/frontend/codebase/README.md` rows for `app/shell.tsx` and `app/sidebar/*` and the
  `composition` / `sidebar` lines of "Architecture modules"; `web/src/app/sidebar/workspace.ts`
  (storage in try/catch, the pattern to copy); `web/src/components/ui/tooltip.tsx`;
  `web/src/app/settings/person-theme.test.tsx` (how a test stubs `window.matchMedia`)
- Touch: new `web/src/app/sidebar/sidebar-state.ts`, `web/src/app/sidebar/sidebar-toggle.tsx`,
  `web/src/app/sidebar/sidebar.test.tsx`; changed `web/src/app/sidebar/sidebar.tsx`,
  `web/src/app/sidebar/index.ts`, `web/src/app/sidebar/contract.yaml`, `web/src/app/shell.tsx`,
  `docs/frontend/codebase/README.md` (two new rows: `sidebar-state.ts` "Whether the sidebar
  shows, kept per browser", `sidebar-toggle.tsx` "Button that hides or shows the sidebar")
- Needs: none
- Visual check: no mockup. Before committing, run the app (`npm run dev` against a running
  `shadows serve`) and take screenshots at 1280×800 and 375×812 of a conversation page with the
  sidebar shown and hidden (narrow: hidden, and shown as the overlay), and confirm the slide
  runs both ways (the plan's "The slide"). Confirm the main pane's
  toggle covers no header control, and the conversation fills the width when hidden. Save
  them under the session scratchpad and name their paths in the report.
- Done when: `npx vitest run src/app/sidebar src/app/agents` passes, with these new tests in
  `sidebar.test.tsx`:
  - "the sidebar's button hides it, the main pane's button and Ctrl+B show it again" (the aside
    with `id="app-sidebar"` is gone, then back; `aria-expanded` follows)
  - "a hidden sidebar stays hidden after a reload" (`shadows.sidebar.collapsed` is `"true"`;
    remount at the same URL; no aside)
  - "without storage the choice lasts until reload" (a throwing `localStorage` does not break
    the toggle)
  - "below 768 px it starts hidden, shows as an overlay, and closes on a chosen conversation,
    on Escape and on the backdrop" (stored `"false"` is ignored when narrow; nothing is written
    by a narrow toggle; focus lands on the main pane's toggle after Escape)
  and every existing sidebar and AYLUL test still passes unchanged; plus the checks in
  `AGENTS.md` rule 5.

### F-53 · The stream and the api read tool cards, `tool` frames and `todos` frames
- Status: blocked (the request `docs/shared/requests/2026-10-31-tool-activity.md` is `asked`)
- Plan: docs/frontend/plans/2026-10-31-shell-and-activity.md#2-agent-activity ("Reading")
- Read: the request (as accepted: its "Fields" and "Backend answer" win over the plan where they
  differ); `web/src/stream/contract.yaml` (StreamState, frame_readers, the transient-frame
  traps); `web/src/api/contract.yaml` (`externally_owned.aliases`, `threads_operations`);
  `web/src/stream/frames.ts` at `readSubagentCard` and `parseSubagent` (the pattern to copy);
  `web/src/stream/thread-stream.ts` at `#open` and the `lagged` handler
- Touch: `web/src/api/schema.d.ts` (only through `npm run gen:api`, run first),
  `web/src/api/threads.ts`, `web/src/api/contract.yaml`, `web/src/stream/frames.ts`,
  `web/src/stream/frames.test.ts`, `web/src/stream/thread-stream.ts`,
  `web/src/stream/thread-stream.test.ts`, `web/src/stream/index.ts`,
  `web/src/stream/contract.yaml`
- Needs: `docs/shared/requests/2026-10-31-tool-activity.md` accepted, and its Core side built so
  `api/openapi.json` names `ToolCallCard`, `DiffHunk`, `TodoListCard` and the `TodoList` kind
- Done when: `npx vitest run src/stream src/api` passes, with new tests:
  - frames: "reads a tool frame, and refuses one with a status it does not know";
    "reads a todos frame, and refuses an item without content"; "readToolCard reads a card with
    its diff hunks, and returns null for a card it cannot read"; "readTodoList returns null
    for a value that is not a list card"
  - thread-stream: "keeps the latest tool frame per call and the latest list per turn, past
    turn-end"; "clears tools and todos on a break and on lagged"
  and the stream contract lists `tool` and `todos` under shapes, frame_readers and obligations.

### F-54 · Tool calls drawn as groups of numbered steps, running and timed
- Status: blocked (needs F-53)
- Plan: docs/frontend/plans/2026-10-31-shell-and-activity.md#2-agent-activity ("Grouping",
  the words table, "A step", "Copy", "Fork and links", "Rendering cost"; signatures in "Files")
- Read: `web/src/app/conversation/contract.yaml` (Messages, Entry, the F-51 obligation "A delta
  redraws the reply, not the thread", copyText agreement, fork rules, "A link to a requested
  entry lands on it"); `web/src/app/conversation/messages.tsx` at `Messages`, `EntryRow`,
  `runningCards`, `forkAnchor`; `web/src/app/conversation/tool-text.ts`;
  `web/src/app/conversation/subagent-text.ts` at `duration`
- Touch: new `web/src/app/conversation/activity-rows.ts`, `activity-rows.test.ts`,
  `activity-text.ts`, `activity-text.test.ts`, `tool-group.tsx`, `tool-group.test.tsx`;
  changed `web/src/app/conversation/messages.tsx`, `web/src/app/conversation/entry.tsx` (drop the
  `ToolCall` branch), `web/src/app/conversation/contract.yaml`, `web/src/test/contract-fixtures.ts`
  (only to add a sample `ToolCallCard`), `docs/frontend/codebase/README.md` (rows for the three
  new source files, jobs as in the plan's "Files" table)
- Needs: F-53 done
- Visual check: as F-52's, at 1280×800, of a conversation holding a finished group (closed and
  opened) and a running one; paths in the report.
- Done when: `npx vitest run src/app/conversation` passes, with new tests:
  - activity-rows: "groups consecutive tool calls of one turn, and splits at another kind or
    another turn"; "an entry with no card is a completed step with its title";
    "a live frame already written as an entry is not a live step"
  - activity-text: one `it.each` over the words table ("words %s"), plus "an edit's counts
    are summed", "a sentence wins over the verb", "lines read as (47–102)"
  - tool-group: "a finished group is one closed line with its time, and opens into numbered
    steps"; "a running group is open, spins, and counts its seconds"; "a failed step is marked
    failed and the line says how many"; "Copy on a group copies every step's line"; "a link to
    a member's entry opens its group"; "draws a group once while many deltas arrive"
  and the existing "a tool call reads as a tool line, not as bracketed text", "Copy on a tool
  line copies its title (§23.8)", "plan tool lines read as sentences" and "draws the plan card
  and the long markdown once while many deltas arrive" pass unchanged.

### F-55 · An edit step opens into its diff
- Status: blocked (needs F-54)
- Plan: docs/frontend/plans/2026-10-31-shell-and-activity.md#2-agent-activity ("The edit's diff")
- Read: `web/src/app/conversation/contract.yaml` (ToolGroup as F-54 left it);
  `web/src/app/conversation/tool-group.tsx` at `StepLine`; `DiffHunk` in `web/src/api/threads.ts`
- Touch: new `web/src/app/conversation/tool-diff.tsx`, `tool-diff.test.tsx`; changed
  `web/src/app/conversation/tool-group.tsx`, `web/src/app/conversation/contract.yaml`,
  `docs/frontend/codebase/README.md` (row: `tool-diff.tsx` "An edit's hunks drawn with line
  numbers")
- Needs: F-54 done
- Visual check: as F-52's, at 1280×800, of an opened edit step in light and dark themes.
- Done when: `npx vitest run src/app/conversation` passes, with new tests: "draws added and
  removed lines with their old and new numbers"; "draws no numbers when the hunk has no
  start"; "marks the gap between two hunks"; "a diff too long to show says so and keeps its
  counts"; "an edit step without a diff has no chevron".

### F-56 · The agent's checklist, live and after reload
- Status: blocked (needs F-53; run after F-54, which changes the same files)
- Plan: docs/frontend/plans/2026-10-31-shell-and-activity.md#2-agent-activity ("The checklist")
- Read: `web/src/app/conversation/contract.yaml` (Messages, Entry, "A streamed reply stays
  ... a message is never shown twice"); `web/src/app/conversation/messages.tsx` at `Messages`;
  `TodoItem` and `readTodoList` in `web/src/stream/frames.ts`
- Touch: new `web/src/app/conversation/todo-card.tsx`, `todo-card.test.tsx`; changed
  `web/src/app/conversation/messages.tsx`, `web/src/app/conversation/entry.tsx`,
  `web/src/app/conversation/contract.yaml`, `docs/frontend/codebase/README.md` (row:
  `todo-card.tsx` "The agent's task list drawn as a checklist")
- Needs: F-53 done
- Visual check: as F-52's, at 1280×800, of a live list with all three states.
- Done when: `npx vitest run src/app/conversation` passes, with new tests: "draws done, in
  progress and pending items with their marks and the count"; "a live list updates with each
  todos frame"; "once the TodoList entry is in the list, the live card goes and the entry shows
  it, never both"; "after a reload the TodoList entry draws the checklist"; "an unreadable card
  shows its body"; "Copy on a checklist copies its Markdown list".

