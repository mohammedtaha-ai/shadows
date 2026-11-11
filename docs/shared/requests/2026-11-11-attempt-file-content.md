# A changed file's content at an attempt's commit

- Status: accepted
- Asked by: Masada, 2026-11-11
- Screen: the task page's Diff tab, [plan](../../frontend/plans/2026-11-11-task-page-review-and-diff.md#f-109-the-whole-file-with-its-changes)

## Need

Mohammed wants the Diff tab to read like Claude's file pane: the chosen file opens in the main
area and reads whole, its changed lines marked, with the list of changed files in a small box
below. Today `GET /api/attempts/{id}/diff` gives each file's counts and its patch as git text,
so the page can show only the hunks (F-107 builds that). To show the whole file it needs the
file's text at the attempt's commit, and the changes as structured hunks, so the web draws
changed lines without reading git's patch format.

## Fields

| Call | Sends | Gets back |
|---|---|---|
| `GET /api/attempts/{id}/files?path=<path>` (shape is Hamoud's call) | a `path` from that attempt's diff | `{ path, text: string \| null, cut: boolean }`: the file at the attempt's commit; `text` null for a deleted or binary file; `cut` true when Core shortened a long file |
| `GET /api/attempts/{id}/diff` | nothing new | `FileDiff` gains `hunks`, the same `DiffHunk` shape the conversation's edit cards read (`old_start`, `new_start`, `lines: [{ op, text }]`), so the page marks changed lines in the file with no parsing |

## Errors

| When | The screen shows |
|---|---|
| no such attempt, or a path not in its commit (404) | Core's message in the file area; the list stays |
| the attempt has not committed | the tab already shows "Nothing committed yet" and does not call |
| `text` null | "No text to show" for that file, with its hunks if any |

## Backend answer

As asked, with one widening. Hamoud, 2026-11-11. Service: Runs
(`crates/core-runs/contract.yaml`); route in `shadows-http`.

- **`GET /api/attempts/{id}/files?path=<path>`** answers `AttemptFile { path, text: string | null,
  cut: boolean }`: the file as the attempt's commit holds it, never the working tree. It calls
  `Runs::file`, the same method the Planner's new `task_file` tool calls (B-129), so the page
  and the Planner read one answer.
- **The widening:** `path` may be any path inside the task's `writes`, not only one in the
  attempt's diff. The page sends diff paths, so nothing changes for it; the Planner needs the
  rest after an addition, whose diff holds only the addition's change.
- `text` is null for a binary file and for a file the commit deleted. `cut` is true when the
  file passed 2,000 lines or 200,000 bytes; the text is shortened with no marker in it.
- **Errors:** 404 for an unknown attempt, an attempt with no commit, or a path not at its
  commit; 409 `{ code: "outside_writes", reason }` for a path outside the task's `writes` (the
  run refusal shape every run route uses).
- **`FileDiff` gains `hunks`**, the `DiffHunk` shape the edit cards read: `{ path, old_start,
  new_start, lines: [{ op, text }] }`, `op` one of `context`, `add`, `remove`. Read from the
  same git patch, so they match `patch`; they stop where a long patch is cut (2,000 lines); a
  binary file has none. `old_start` is 0 for a new file.
- Built by B-131 once this is `accepted`. B-129 (the Planner's side) builds `Runs::file` first
  and changes nothing the page sees.

## Frontend reply

Accepted as answered, the widening included (Masada, 2026-11-11). B-131 may be built. The
page sends only paths from the attempt's diff, so the widening changes nothing for it; any
refusal (404, 409 `outside_writes`) shows Core's message in the file area with the list kept.
F-109 is written on this shape in
[`docs/frontend/stages.md`](../../frontend/stages.md#f-109--the-whole-file-with-its-changes-in-the-diff-tab):
the page draws the file's `text` and marks lines from `FileDiff.hunks`, and no longer reads
`FileDiff.patch`. `patch` may stay for other readers.

## Ruling

<The teacher, only when disputed.>
