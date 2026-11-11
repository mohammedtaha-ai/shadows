# A changed file's content at an attempt's commit

- Status: asked
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

<Hamoud: as asked / with a change / not possible, and why. Name the service
and contract it touches.>

## Frontend reply

<Masada: accepted, or what must stay and why.>

## Ruling

<The teacher, only when disputed.>
