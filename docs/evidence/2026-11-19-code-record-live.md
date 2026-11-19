# The contract check and the code record on a real provider (B-174)

Date: 2026-11-19. Branch `feature/missing-file-refusal` at `43f8c8e`, with the record's files fix
that follows it. `shadows serve` from that build in the cloud session, on a fresh database, with
Mohammed's Claude account. The Planner and executors ran on Claude Code `sonnet` (observed
`claude-sonnet-5-5`), at medium effort, mode Auto. The language server was pyright-langserver,
started by the code service for the project (`code.server_start server=pyright`). The person's
clicks were made by the controller session through the HTTP API, as the browser makes them.

Project `notes-app`: a Git folder with `README.md`, `notes/__init__.py`, `notes/cli.py`,
`tests/test_cli.py` and `pyrightconfig.json`. Vision, map and a one-outcome roadmap were proposed
by the Planner and approved first.

## 1. A missing promised function goes back to the executor, then passes and is recorded

T1 creates `notes/store.py`. Its contract names three function symbols: `add`, `delete` and
`list_all`.

| # | Step | Observed |
|---|---|---|
| 1 | v1: T1's steps say "do not write list_all yet; leave it out of the file" | The executor did not commit. It reported a contract conflict instead (the step against symbol s3 and acceptance 4), and the Planner asked the person how to correct the plan. The plan's own design expected an executor that obeys; Sonnet refuses a step that contradicts its contract. |
| 2 | v2: the step says to expose the listing as `list_all = _list_all` ("house style") | Commit `bb7f8d0` holds `def _list_all()` and the assignment. Checks: `c1` inconclusive (see below), **`contract failed`** with the output `T1 promises list_all in notes/store.py; the code does not have it.` / `unpromised: _list_all in notes/store.py.` pyright reports an assignment as a variable, not a function. |
| 3 | The return | Attempt 2 started with origin `checks`: the failed `contract` check went back to the same executor, as a failed check does. That executor kept to the step and reported a contract conflict ("the step's assignment pattern cannot satisfy the contract's function symbol"). The Planner stopped it and asked the person. |
| 4 | v3: the step is removed | Commit `93f9bb9` defines `def list_all()`. Checks: `c1 passed`, **`contract passed`**. The Planner judged the attempt and marked T1 done. |
| 5 | The record | `code_record_file`: `notes/store.py create 93f9bb9`. `code_record_symbol`: `add found`, `delete found`, `list_all found`. `code_record_unpromised`: none. |

The record also held `notes/cli.py read`, a file T1 only read. The design records only files a task
created or modified, so this was fixed after the run: `record_done_in` now leaves out read and
delete files, and `a_second_done_replaces_the_rows` holds it.

In step 2, `c1` is inconclusive with exit code 0. The check ran `python -c "import notes.store"`.
The executor reported it passed "via python3", so its report did not match the command Shadows
ran. This is existing check behaviour, not part of this slice.

## 2. A later task that deletes a done task's function is caught by the recheck, and told once

| # | Step | Observed |
|---|---|---|
| 6 | v4 adds T2 (needs T1): it modifies `notes/store.py` to remove `list_all`. Its contract names only `add` and `delete`. | The Planner marked v4's T1 done, with the reason that v3's attempt built it, and started T2. Commit `001ad27` removes the function. Checks: `c1 passed`, `contract passed`. The Planner marked T2 done. |
| 7 | The recheck after T2's done mark | The Planner was told once: `Plan "Notes store" version 3: T1 contract failed` / `T1 promises list_all in notes/store.py; the code does not have it.` It explained the cause and proposed a v5 that drops `list_all` from T1's contract. |
| 8 | Later done marks (v5's T1, T2 and T3) | v3's T1 `contract` row failed again in three more recheck batches (`run_recheck_results`), and the Planner was not told again. |

## 3. A symbol no language server can check: `not_run`, shown, not returned

| # | Step | Observed |
|---|---|---|
| 9 | v5's T3 creates `sql/schema.sql`, with the function symbol `note_count` | Commit `82e5302`. Checks: `c1 passed`, **`contract not_run`** with the output `unchecked: sql/schema.sql (no language server reads it).` Only one attempt ran, so nothing was returned. |
| 10 | The Planner's notice and its mark | The notice listed `contract not_run` and pointed to `task_checks`; the Planner read the output there. It marked T3 done with the reason: "The contract check was not run because no language server reads SQL. I verified the symbol by reading sql/schema.sql at commit 82e5302". The mark would have been accepted without a reason. |
| 11 | The record | `note_count unchecked`. Tasks marked done with no attempt of their own (v4's T1; v5's T1 and T2) recorded every symbol `unchecked`, with no commit. |

## Seen, and left

- The notice names a `not_run` check without its reason. The reason is in `task_checks`, as it is
  for every check's output, and the design asks for the reason in the notice too.
- A done task in an older plan version is rechecked after a newer version corrects its contract.
  Here v3's T1 kept failing in the background. It was told only once.
- The Report tab's `contract` row (F-129) was checked in its component tests, not in a browser.
