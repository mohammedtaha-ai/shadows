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

Not run live: the skip while a project's first file listing is still running. A listing of this
project ends in milliseconds, so it cannot be held open from outside; the case is covered by
`the_check_is_skipped_when_the_file_list_cannot_answer` in `crates/shadows-core/tests/task_contract.rs`.
