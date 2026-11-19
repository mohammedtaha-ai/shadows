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

## Seen

- The notice named a `not_run` check without its reason; the reason was only in `task_checks`.
  Fixed after the run: a `contract` check that does not pass is followed by its output in the
  notice (`no_language_server_is_said_and_not_returned`).
- A done task in an older plan version was rechecked after a newer version corrected its
  contract and marked it done: v3's T1 kept failing in the background, told once. Fixed after the
  run: a done task that a newer version of its plan also marks done is not rerun
  (`a_done_task_a_newer_version_marks_done_is_not_rechecked`).
- The Report tab's `contract` row (F-129) was checked in its component tests, not in a browser.

## In the browser, before the PR

Date: 2026-11-19. The branch at `35136db`, served by `shadows serve` and the Vite web app
(`http://127.0.0.1:5173`), on a fresh database and a fresh `notes-app`. The person's steps were
made in the web app, driven by Playwright: create the project, write to the Planner, approve each
card with its button (the map's with "Approve map and waivers"), approve the plan, start a task
on its page, answer the Planner's question with its button, and mark a task not done and done
again. The Planner ran on Sonnet 5.5.

| # | Step | Seen in the web app |
|---|---|---|
| 1 | v1's T1 exposes `list_all = _list_all`; the person starts it | Attempt 1 committed. The Report tab shows `T1 · Contract`, "Confirmed: failed", with no command line and no Reported field. Attempt 2, "Returned by checks", quotes `contract failed` / `T1 promises list_all in notes/store.py; the code does not have it.` |
| 2 | The executor reports the step against the contract; the Planner asks with three buttons | The person chose "Drop the step; define list_all directly with def", and v2 followed. |
| 3 | v2's T1 and T2 | Both checks passed on T1 (`by_shadows` true only on `contract`). The Planner marked T1 done and started T2 itself; T2's pytest check passed and the Planner marked it done. The recheck after T2 ran T1's `c1` and `contract`: both passed, so no notice. |
| 4 | The person removes `list_all` by hand and commits; marks T2 not done, then done with a reason | The conversation shows "Shadows · recheck after T2 (version 2)": `T1 · Contract`, "Confirmed: failed", "Their done marks stand". The Planner found the hand commit and offered three ways on. |

Seen, and left:

- `c1` (`python3 -c "import notes.store"`) was "inconclusive" with exit code 0, and the Report tab
  showed a Mismatch with nothing saying why. The check itself wrote `notes/__pycache__/`, and a
  check that changes the working tree is inconclusive. Adding `.gitignore` made it pass. The rule
  is older than this branch; the missing reason is the gap.
- The recheck card in the conversation shows `T1 · Contract  Confirmed: failed` without its
  output, as for every check in that card. The Planner's notice names the missing function; the
  person sees which one only by asking or by opening the task.

## Another language: Rust, with rust-analyzer

Same session, a fresh Rust library crate `notes-rs`. Its vision, map, roadmap and plan were made
and approved in the web app, and each task was started from its page.

| # | Step | Seen |
|---|---|---|
| 1 | T1 creates `src/store.rs`: `pub struct Store` with `impl Store { new, add, delete, list_all }`; its contract names `Store` (class) and `Store.new` … `Store.list_all` (method) | `contract not_run`: `the language server rust-analyzer failed:   14: <unknown>`. The `rust-analyzer` on PATH was rustup's stub without the component. The Planner marked T1 done with that reason. |
| 2 | rust-analyzer's own answer for that file, asked directly | `Store` kind 23 (struct); `impl Store` kind 19 holding `new` 12, `add`/`delete`/`list_all` 6; a `tests` module kind 2. The check counted neither kind 23 nor methods under an `impl` block, so once the server ran, every promised symbol would have read missing and the attempt would have been returned with correct code. |
| 3 | Fixed (`f9c6d0f`, `55c566b`): struct, interface and enum count as a class; an `impl X` / `impl Trait for X` block and a gopls `(*X).m` name give the method its type; a failed server reports its first stderr line that is not a backtrace frame | `live_a_rust_struct_and_its_impl_methods_are_found` passes on real rust-analyzer. |
| 4 | After `rustup component add rust-analyzer` and a daemon restart on the fixed build: v2's T2 adds `Store.len` and `Store.is_empty` | `c1` (`cargo test -q`) passed, **`contract passed`**. The Planner marked T2 done. The record: `Store`, `Store.len`, `Store.is_empty` all `found`; files `src/store.rs` modify only. |

TypeScript was not run: the machine has TypeScript 6.0.2, which has no `tsc --lsp`, the server
the language-server design chose for TypeScript 7. A TypeScript project there gets `contract
not_run` with the server's error line. Go is covered by recorded gopls answers only (gopls is
not installed here).
