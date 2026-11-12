# Task page: the Planner's review, and the Diff tab as a file pane

Mohammed, 2026-11-11. Two pieces of the task page (`web/src/app/tasks/`):

1. Slice 1b's Planner review, from [the planner review request](../../shared/requests/2026-11-11-planner-review.md)
   (accepted): "waiting for the Planner's judgment" now, an attempt's addition once B-126 ships.
2. The Diff tab reads like Claude's file pane: the chosen file reads in the main area, the
   list of changed files is a small box below it.

## Global constraints

- Every read goes through `web/src/api/runs.ts`; no `fetch`, no URL, no Core rule in a
  component (`docs/frontend/AGENTS.md` rule 0).
- Find symbols with Serena (`s.lsp` only). Open a file at the symbol you need.
- Run only: `npx vitest run src/app/tasks` plus `npm run typecheck`, `npm run lint`,
  `npm run build` and `npx vitest run src/codemap.test.ts src/architecture`, from `web/`.
  Never the whole suite. Never run vitest while a `cargo` process runs on this machine
  (check with `powershell -NoProfile -Command "Get-Process cargo -ErrorAction SilentlyContinue"` (empty output means none) before each vitest run; if one runs, wait and check again).
- No browser journey: Mohammed tries it in the browser himself.
- One commit per task, `git add <each file>`, nothing under `.claude/`, no push.

## F-106 · The task page says when the task waits for the Planner's judgment

The request states the reading: the task waits for the Planner's judgment when `run.latest`
is not null, `run.latest.state === 'committed'` and `run.done === false`.

- `web/src/api/runs.ts`: one exported function `awaitingJudgment(run: TaskRun): boolean` with
  that reading and a doc comment naming the request as its source. `taskRunQuery`'s
  `refetchInterval` reads again every 2000 ms while the latest attempt runs **or**
  `awaitingJudgment` holds (there is no event for a run, and the Planner marks or carries on
  without the page asking). Add the function to the `runs:` block of `web/src/api/contract.yaml`
  and amend the `taskRunQuery` line.
- `web/src/app/tasks/use-task-run.ts`: when the task run's `latest` attempt changes (its `id`
  or `state`), the attempts list is read again (invalidate or refetch `attemptsQuery`), so a
  new attempt the Planner starts shows on the Report tab.
- `web/src/app/tasks/task-header.tsx`: while `awaitingJudgment(run)` holds, beside the state
  badge, a second badge reading `Waiting for the Planner's judgment` (same badge style, muted).
  Nothing else changes: Start stays as `start.allowed`/`start.reason` say (Core answers
  `task_busy` with its reason), Mark done stays available to the person.
- `web/src/app/tasks/contract.yaml`: one obligation: "A committed task not marked done shows
  'Waiting for the Planner's judgment' and is read again every two seconds until it is marked
  or a new attempt starts (a_committed_task_waits_for_the_planners_judgment)."
- Tests in `web/src/app/tasks/task-page.test.tsx`:
  - `a_committed_task_waits_for_the_planners_judgment`: the read answers latest `committed`,
    `done: false`; the badge shows. The next read answers `done: true`; the badge goes and
    `Done` shows, with no click (the page read again by itself).
  - `a_task_marked_done_does_not_wait`: `committed` and `done: true` shows no badge.

## F-107 · The Diff tab: the chosen file above, the changed files in a small box below

Data: `attemptDiffQuery(attempt.id)` only: `FileDiff { path, added, removed, patch }`, where
`patch` is git's unified diff text for that file, cut by Core at 2,000 lines with a marker.

Layout (the tab panel is already `min-h-0 flex-1 overflow-y-auto`; the Diff tab fills it as a
column of full height):

- **File area** (top, `flex-1 min-h-0`, scrolls on its own): a one-line header with the file's
  `path` (mono), `+added` and `-removed`; under it the patch drawn by a new component
  `FilePatch` in `web/src/app/tasks/file-patch.tsx`, one row per line of `patch`:
  - a line starting `@@`: a hunk separator row (muted, `bg-muted/40`); read the old and new
    start numbers from `@@ -a[,b] +c[,d] @@` to number the lines after it;
  - `+`: added, tint `bg-success/15`, the new line number;
  - `-`: removed, tint `bg-destructive/15`, the old line number;
  - ` ` (space): context, both numbers;
  - anything else (the `diff --git`/`index`/`---`/`+++` header, `\ No newline…`, Core's cut
    marker): muted, no number, shown as is.
  Two number columns (old, new), mono `text-2xs`, the text in a `whitespace-pre` cell with
  horizontal scroll, `dir="ltr"`. This draws git's text; it decides nothing about it.
- **Changed files box** (bottom, a bordered box, `max-h-48`, scrolls on its own,
  `aria-label="Changed files"`): one row per file: path (truncated, mono), `+added`, `-removed`.
  Each row is a button with `aria-pressed` for the open file. Clicking a row opens that file in
  the file area. The first file is open when the tab opens; no file is ever closed to nothing.
- Unchanged: "Nothing committed yet." without a committed attempt, "Reading the diff…" while
  reading, `ErrorLine` on a refused read. A commit with no files shows "No files changed." in
  the file area and an empty box.
- `docs/frontend/codebase/README.md`: a row for `web/src/app/tasks/file-patch.tsx`, "One file's
  patch drawn with line numbers", contract `web/src/app/tasks/contract.yaml`; the `diff-tab.tsx`
  row becomes "The Diff tab: the open file above, the changed files below".
- `web/src/app/tasks/contract.yaml`: one obligation: "The Diff tab opens the first changed file
  above and lists every changed file in a box below; a click opens that file. The patch is shown
  as Core's text, line by line, never as markup (diff_opens_the_first_file_and_switches_on_click)."
- Tests in `web/src/app/tasks/task-page.test.tsx`: replace `diff_lists_files_and_opens_a_patch`
  with `diff_opens_the_first_file_and_switches_on_click`: two files; the first file's added line
  shows in the file area with its new line number; the box lists both with their counts; a click
  on the second shows its patch and not the first's. Add `diff_shows_a_cut_marker_as_plain_text`:
  a patch whose last line is a non-diff line (Core's marker) shows it, with no line number.

## F-108 · An attempt's addition on the Report tab (blocked on B-126)

Needs B-126 built and `api/openapi.json` showing `Attempt.addition`; first step `npm run gen:api`.

- `web/src/app/tasks/report-tab.tsx`: each attempt card, under its heading line, says
  `Began from Start` when `addition` is null, or `Carried on with the Planner's addition` and
  the addition's text (`dir="auto"`, `whitespace-pre-wrap`, `aria-label="Planner's addition"`,
  plain text, never markup) when it is not.
- If the request's backend answer names how a task stopped after its third addition shows,
  the task page shows that field or reason as Core says it; the web counts no additions.
  Masada amends this task with the exact field before it runs.
- `web/src/app/tasks/contract.yaml`: one obligation for the addition (the test below).
- Test in `web/src/app/tasks/task-page.test.tsx`:
  `the_report_shows_the_planners_addition_as_plain_text`: attempt 2 with an addition holding
  `<b>x</b>` shows the literal text and "Carried on…"; attempt 1 with `addition: null` shows
  "Began from Start".

## F-109 · The whole file with its changes

Needs [the attempt file content request](../../shared/requests/2026-11-11-attempt-file-content.md)
(accepted 2026-11-11), built by B-131. Run `npm run gen:api` first, once B-131 is committed
(`api/openapi.json` holds `AttemptFile`, the `/api/attempts/{id}/files` route and
`FileDiff.hunks`).

What Core gives:
- `GET /api/attempts/{id}/files?path=<path>` answers `AttemptFile { path, text: string | null,
  cut: boolean }`: the file as the attempt's commit holds it. `text` is null for a binary file
  and a file the commit deleted; `cut` is true when Core shortened a long file (no marker in
  the text). 404 for an unknown attempt, no commit, or a path not at the commit; 409
  `outside_writes` for a path outside the task's writes (the page sends diff paths only).
- `FileDiff.hunks: DiffHunk[]` (`{ path, old_start, new_start, lines: [{ op, text }] }`, `op`
  one of `context`, `add`, `remove`), read from the same patch; they stop where a long patch is
  cut; a binary file has none; `old_start` is 0 for a new file.

**The read.** `web/src/api/runs.ts`: `runKeys.file(attemptId, path)`;
`attemptFileQuery(attemptId: string, path: string)` over a private `getAttemptFile` that calls
`client.GET('/api/attempts/{id}/files', { params: { path: { id }, query: { path } } })`; export
`type AttemptFile`. Lines in the `runs:` block of `web/src/api/contract.yaml`.

**The file area.** `web/src/app/tasks/file-patch.tsx` is deleted (it read git's patch text);
new file `web/src/app/tasks/file-view.tsx`:
- `fileRows(text: string, hunks: readonly DiffHunk[])`: one row per line of `text` (a final
  empty piece after a trailing newline is not a line), numbered from 1. Walk each hunk from
  `new_start` (n): `context` moves n on; `add` marks line n added and moves n on; `remove`
  puts that removed text before line n, and does not move n. Removed text before line
  `lines + 1` shows after the last line. A mark that falls past the text's end (a cut file) is
  dropped. A hunk with `new_start` null is skipped. This joins Core's two answers for drawing;
  it decides nothing about the change.
- `FileView({ file, hunks })` where `file` is the `AttemptFile`:
  - `text` set: the rows above, one line each, mono `text-2xs`, `dir="ltr"`,
    `whitespace-pre` with horizontal scroll: a line number column (empty for a removed row), a
    mark column (`+` added, `-` removed, blank otherwise), the text. Added rows tint
    `bg-success/15`, removed rows `bg-destructive/15`, as F-107 did. `data-diff="add"` /
    `"remove"` on the marked rows.
  - `cut` true: a last muted row, "Core shortened this file; the rest is not shown."
  - `text` null: "No text to show" (muted), then, when `hunks` is not empty, each hunk drawn
    alone: a muted separator row, then its lines with their mark and tint and the new line
    number (old number for a removed line). A deleted file reads as all its lines removed.
- `web/src/app/tasks/diff-tab.tsx`, `Files`: the open file's area reads
  `useQuery(attemptFileQuery(attemptId, open.path))`. While it reads: "Reading the file…"
  (muted). On a refused read: `ErrorLine` with Core's message **inside the file area**; the
  header line and the changed files box stay. Then `<FileView file={...} hunks={open.hunks} />`.
  The header line (path, `+added`, `-removed`) and the box are unchanged. `open.patch` is no
  longer read by the page.
- `docs/frontend/codebase/README.md`: the `file-patch.tsx` row becomes
  `web/src/app/tasks/file-view.tsx`, "One file at the attempt's commit with its changes marked".
- `web/src/app/tasks/contract.yaml`: the Diff tab obligation now reads: "The Diff tab opens the
  first changed file above, whole as the attempt's commit holds it, with Core's added and
  removed lines marked, and lists every changed file in a box below; a click opens that file. A
  refused file read shows Core's message in the file area and keeps the list
  (diff_opens_the_first_file_and_switches_on_click, diff_shows_the_whole_file_with_its_changes_marked,
  diff_shows_cores_refusal_in_the_file_area)." Remove the obligation that names
  `diff_shows_a_cut_marker_as_plain_text`.

**Tests** in `web/src/app/tasks/task-page.test.tsx` (the fake daemon answers the file route):
- `diff_opens_the_first_file_and_switches_on_click` stays, with the file route answered for
  both files; the second file's text shows after the click.
- `diff_shows_the_whole_file_with_its_changes_marked`: text of 5 lines; one hunk from
  `new_start` 2: context line 2, remove `old three`, add line 3 (`new three`), context line 4.
  All 5 lines show with numbers 1 to 5; line 3 is marked added; `old three` shows just above it,
  marked removed, with no number.
- `diff_shows_no_text_for_a_deleted_file_with_its_hunks`: `text` null, one hunk of two removed
  lines: "No text to show" and both lines marked removed.
- `diff_says_when_core_shortened_the_file`: `cut: true`: the last row says so.
- `diff_shows_cores_refusal_in_the_file_area`: the file route answers 404 with Core's message:
  the message shows, and the changed files box still lists every file.
- `diff_shows_a_cut_marker_as_plain_text` is deleted: the page no longer reads patch text.
