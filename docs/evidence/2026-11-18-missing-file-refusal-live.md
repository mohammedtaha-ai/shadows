# A task may not modify a file the project does not have, on a real Planner (B-167)

Date: 2026-11-18. Branch `feature/missing-file-refusal` at `84efaf9`; `shadows serve` from that
build in the cloud session, a fresh database, Mohammed's Claude account. Planner on Claude Code,
`sonnet` (observed `claude-sonnet-5-5`), medium effort, mode Auto. The person's clicks were made
by the controller session through the HTTP API (`/api/.../approve`), as the browser makes them.

Project `notes-app`: an existing Git folder with `README.md`, `notes/__init__.py`, `notes/store.py`,
`notes/cli.py`, `tests/test_store.py`.

| # | Step | Observed |
|---|---|---|
| 1 | Vision, map, one-outcome roadmap | Five vision cards, one map (after two `MAP_REFUSED` the Planner fixed: an unknown rule id, and two parts claiming overlapping folders) and a roadmap with six acceptance items, each approved. |
| 2 | The Planner drafts a plan with contracts | T1 modifies `notes/store.py` and `tests/test_store.py`; T2 (needs T1) modifies `notes/cli.py` and creates `tests/test_cli.py`. Accepted at once: `modify` only on files the project has. |
| 3 | The person insists T3 modifies `notes/export.py` and `notes/formats.py` (absent) and `tests/test_cli.py` | The edit was refused: `WORKFLOW_VALIDATION_FAILED: T3 modifies notes/export.py, which the project does not have and no task it depends on creates; T3 modifies notes/formats.py, which the project does not have and no task it depends on creates`. The plan stayed at revision 1. |
| 4 | `tests/test_cli.py` in the same edit | Not refused: the Planner had made T3 need T2, which creates it. |
| 5 | The Planner's answer to the person | It quoted the refusal, listed the project's real files, and offered to create the two files or put the export in existing ones. |

| 6 | After a daemon restart on the same database: T3 (needs T2) deletes `README.md`, T4 (needs T3) modifies it, in one edit | Refused: `WORKFLOW_VALIDATION_FAILED: T4 modifies README.md, which a task it depends on deletes`. T3's delete of an existing file was not flagged. |
| 7 | The project's folder moved away, then T3 (needs T2) modifies the absent `notes/ghost.py` | Accepted (revision 2): the file list cannot answer for a missing folder, so the check is skipped. The folder was put back after the turn. |

Seen in step 7: the skip is silent. The Planner noticed it was accepted where a like edit had
been refused and could not tell why. Telling the Planner that the file check was skipped, and
why, is not built.

## The first listing still running (tried, not reachable)

To hold a project's first file listing open, a new project `huge-app` was given 2,000,000 empty
files under `data/` (plus the notes code). Its first listing ran from 23:29:40 to 23:33:26
(`code.scan … elapsed_ms=218119`). The journey was driven straight through it:

| # | Step | Observed |
|---|---|---|
| 8 | Five vision cards | Proposed and approved in 12 s, during the listing. |
| 9 | The map | The Planner's first `map_propose` was refused (a layer path with a note in it); its second one got no answer: `The operation timed out.` The daemon logged no `/mcp` response from 23:30:12 to 23:33:39, and the other projects' listings stalled with it (`notes-app` 177 s, `big-app` 142 s, both normally milliseconds to 22 s). |
| 10 | The plan | Not reached: no map was stored, so `draft_start` was refused with `STAGE_BLOCKED: no roadmap approved`. |

So the skip while a first listing runs cannot be reached through the Planner: a big first listing
holds back the daemon's other work until it ends, and a plan edit cannot be saved before the map
and roadmap anyway. The case stays covered by `the_check_is_skipped_when_the_file_list_cannot_answer`
in `crates/shadows-core/tests/task_contract.rs`.

Found on the way, not part of B-167:

- A project's first listing of 2,000,000 files took 218 s and stalled the Planner's tool calls on
  every project until it ended, so a tool call timed out.
- After the listing, each Planner turn on that project read the whole file list
  (`SELECT path FROM code_inventory WHERE project_id = ? AND kind = 'file'`, 2,000,005 rows, 15 s).

## After B-168: the same journey, rerun

Branch `feature/missing-file-refusal` at `3f31bce` (B-168 built), a fresh database, the same
2,000,000-file `huge-app`, Planner on `sonnet` (observed `claude-sonnet-5-5`).

| # | Step | Observed |
|---|---|---|
| 11 | Vision, map, roadmap and a plan, sent back to back from 23:45:39 | All four turns answered by 23:46:26 (47 s). No tool call timed out; the daemon logged no slow statement. |
| 12 | T1 modifies the absent `notes/ghost.py`, saved at about 23:46:20 | Accepted (revision 1): the first listing was still running (status `indexing`), so the file check was skipped. |
| 13 | The first listing ends | `code.scan … elapsed_ms=45269` at 23:46:37: 45 s, down from 218 s. |
| 14 | T2 modifies the absent `notes/ghost2.py`, after the listing | Refused: `WORKFLOW_VALIDATION_FAILED: T2 modifies notes/ghost2.py, which the project does not have and no task it depends on creates`. |

All five B-167 cases have now run live. Step 12 shows the silent skip again: the Planner
guessed that a plan's first task is not checked, since it was not told the check was skipped
(listed in `docs/backend/stages.md`, "Known and left").
