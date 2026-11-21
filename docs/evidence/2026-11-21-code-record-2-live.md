# The project's state, its diagrams and `code_where` on a real provider (B-181, B-186, in progress)

Date: 2026-11-21. Branch `ccr-9e1b02d8-b0ylm1` at `fef02ed` (all build tasks of plans 2a and 2b
done; full gate passed: 1636 tests, both clippy runs, openapi clean; web 1081 tests). `shadows serve
--debug` from that build in the cloud session, on a fresh database, with the web dev server (Vite).
Provider: Claude Code on Mohammed's account (his ruling for this run, in place of the OPENROUTER and
API_KEY_MINIMAX keys the plans name). Planner and executors on `sonnet`, observed
`claude-sonnet-5-5`, mode Auto, Planner effort medium. Language servers: pyright-langserver,
`tsc --lsp` (TypeScript 7.0.2). The person's clicks were made by the controller session through the
HTTP API, as the browser makes them; the browser checks used Playwright Chromium against the dev
server.

Project `Accounts`: a Git folder with `README.md`, `backend/__init__.py`, `backend/app.py`,
`web/src/main.ts`, `web/tsconfig.json` and `pyrightconfig.json`. The Planner proposed the vision, a
map of three parts (Backend `backend`, Database `database` under Backend, Frontend `frontend`, with
the api part waived and the waiver approved) and a one-outcome roadmap ("People can sign up and
sign in"); each was approved.

## Problems found

| # | Problem | Where | State |
|---|---|---|---|
| P1 | **The Planner does not ask `code_where`.** In the second plan's conversation it loaded the tool (its ToolSearch selected `mcp__shadows__code_where`) and called `project_state` and `design_elements` first, but never called `code_where`. It read 14 project files with `cat` in four shell calls before and around its `plan_edit`. The prompt's "Look before you write a path" says "Ask code_where first". This fails B-186's main expectation. | `crates/core-harness/src/prompt.txt` (B-184's lines) | open: root cause not yet traced |
| P2 | **Diagram nodes clipped their content.** The ER node for `people` cut off its 4th column; screen nodes cut off their purpose line. The layout height (`64 + 18 per row`) was smaller than the drawn node. | `web/src/app/project-state/diagram-layout.ts`, `element-canvas.tsx` (F-132, F-133) | fixed in `01f98d9`: node height is the sum of `NODE_LINES`, the same numbers the node draws with |
| P3 | **Screen-flow arrows were not visible.** For two screens that lead to each other, the back edge looped off the canvas on top of the forward edge, and the labels (default SVG labels under `colorMode="dark"`) were near-white on the light theme. | same files (F-133) | fixed in `01f98d9`: each edge has its own dagre label slot and a custom `diagram` edge with an arrow and an HTML label |
| P4 | **The Planner wrote check commands that cannot pass.** T1 to T3's checks ran `python3 -I -m unittest …`; `-I` drops the project folder from `sys.path`, so `backend` cannot be imported. T3's contract also left an existing test (`test_accounts.py`, asserting `routes() == {}`) outside T3's writes. Each was caught by the executor (a contract conflict) and corrected by the Planner in a new plan version (v2, v3). | Planner behaviour, not this slice | recorded; no change |
| P5 | **The Planner misread the plan's state once.** After v2 was approved (entry 68, "Plan v2 approved"), it told the person "Version 2 is still a draft, and nothing is approved yet" and asked for an approval already given. | Planner behaviour, not this slice | recorded; no change |
| P6 | **A done task carried into a new version keeps `at_done: unchecked`.** T2's symbols, carried to v3 as done with v2's commit, answer `at_done: "unchecked"` in `code_where`, while `live` confirms them. Expected by the design (a done mark with no attempt in that version has no verdict), but it reads as if they were never checked. | `CodeRecord` record rows | recorded; open whether to carry the earlier verdict |
| P7 | **`code_where`'s answer-level `gaps` stayed empty** when a promised symbol was live `gap` (the hand-deleted `SignIn`); the gap shows only on the symbol. To check against the 2b plan's shape. | `crates/code-record/src/code_where.rs` | closed, not a defect: the agreed `WhereAnswer` (shared request, `api/openapi.json`, `model.rs`) has no answer-level `gaps`; the design puts the gaps on the entries |
| P8 | **A long topic finds nothing, and the Planner then reads every file.** In the second run its one `code_where` call asked `"sign-in and sign-up logic, composition, repository, API handlers, web data access"`. The words tier needs every word in one file's purpose or one symbol's contract, so the answer was `entries: []`; the prompt then sends it to read what `code_where` did not answer, and it read 12 files with `cat` before `plan_edit`. One short topic (`sign_in`) gives 11 entries. Neither the tool description nor the prompt says to ask one short thing per call. | `crates/code-record/src/ranking.rs` (`words`), `crates/shadows-mcp/src/record_tools.rs`, `crates/core-harness/src/prompt.txt` | prompt and tool description changed in B-191 (`8673b50`); live, the Planner then did not call `code_where` at all (third run): open, owner decision |
| P9 | **`uses_live: answered` with no uses for Python symbols that have callers.** pyright started before the executors wrote `backend/accounts/`; `SignIn` (used twice in `app_core.py`) and `AppCore` (six uses) answered `uses: []`. pyright run by hand on the same folder returns them, and after a restart of `shadows serve` the same question returns them (its first ask right after the restart also answered `[]`, while pyright was still reading). An empty `uses` under `answered` claims no callers, so this answer is wrong, not unchecked. TypeScript uses were right throughout. | `crates/codebase` (a running pyright's view of files made after it started), `crates/code-record/src/code_where.rs` | fixed in B-192 (`ecbfb76`): new files are sent to the servers as created, not changed, and a started server is used only once it has gone quiet; confirmed live (third run) |

## B-181 · the first plan, the elements and the browser

| # | Step | Observed |
|---|---|---|
| 1 | Asked the Planner to plan "people sign up and sign in" | Before `draft_start` it called `design_elements`, then wrote E1 (table `people`: `id` INTEGER key, `email` TEXT NOT NULL UNIQUE, `password_hash`, `created_at`), E2 (screen Sign in, `/sign-in`) and E3 (screen Sign up, `/sign-up`), each leading to the other, with `design_element_put`. It read the six existing files with `cat` first (the record was empty, as the prompt allows). Its four tasks pin them: T1 `{E1 r1, create, backend/db/migrations/0001_create_people.sql}`; T4 `{E3 r1, …sign_up_screen.ts, s3}` and `{E2 r2, …sign_in_screen.ts, s4}`. |
| 2 | Approved v1; `project_state` | Each part: 4 tasks not started; Database 1 planned element, Frontend 2. |
| 3 | Opened the State view in the browser, then started T1 without reloading | The page changed by itself 12 s later to "0 of 4 done · 1 started · 3 not started" (`TaskProgressChanged` on the project stream). No console errors. |
| 4 | T1 to T4 ran (v1, then v2 and v3 after P4) | Commits `9e5ae51` (T1), `0325a2d` (T2), `3f6b5e7` (T3), `394228d` (T4). The Planner judged and marked each done; T1 and T2 were carried to v3 as done with their commits. |
| 5 | Element states after the plan | E1 `people`: built r1, confirmed `file_only` (a SQL migration; no language server). E2 Sign in: built r2, `file_and_symbol`. E3 Sign up: built r1, `file_and_symbol` (TypeScript server). |
| 6 | Second plan, new conversation: "password reset" | Its first calls: `project_state`, `workspace_get`, `workflow_list`, `project_files`, `design_elements`, `decision_list`. It asked the person how the email is sent (answered: a local file mailer), proposed vision, decision D1, map and roadmap cards (approved), then drafted v1: E4 (new table `password_reset_tokens`, foreign key to `people.id`), E5 Reset password and E6 Forgot password (new screens), and **E2 revision 3** (Sign in now leads to Forgot password). No element was duplicated: `people` was not written again and no second Sign in screen was made. (The plan expected `users` r2 with a token column; the Planner chose a separate token table, which also duplicates nothing.) |
| 7 | Browser, after the fix in `01f98d9` | Tables: `people` and `password_reset_tokens` with every column and the `person_id → people.id` arrow. Screens: Sign in, Sign up (built), Forgot password, Reset password (planned), separate labelled arrows both ways; Sign in shows "built r2, newer planned" (`behind`). |

Not yet done: running the password reset plan, so that E4 to E6 turn built and E2 stops being
behind.

## B-186 · `code_where`

| # | Step | Observed |
|---|---|---|
| 1 | The second plan names existing files without reading the folder | **Failed (P1).** It never called `code_where`; it read the files with `cat`. |
| 2 | `GET /api/projects/{id}/code-where?topic=sign_in` | 200 in 0.2 s. `backend/app/application/accounts.py` (Backend · application) with its tasks and commits; `PasswordHasher` line 39, `PersonRepository` line 31, `SignIn` line 56, each `live: confirmed` by pyright, `uses_live: answered`, `uses_total: 3`, uses grouped by part and layer (Backend · application, Backend · composition). |
| 3 | Deleted `class SignIn` from `accounts.py` by hand, asked `topic=SignIn`, then restored the file | `SignIn`: `live: {state: gap, why: "SignIn is not in backend/app/application/accounts.py now"}`, `uses_live: not_asked`. Answer-level `gaps` was empty (P7). |
| 4 | A stopped language server gives the record's half marked unchecked | Not yet run. |

## Second run · B-190, B-188 and B-186 step 1 again

Branch `ccr-9e1b02d8-b0ylm1` at `d3a5431` (B-190), `shadows serve --debug` in a new cloud session on
a fresh database, web dev server (Vite). Claude Code on Mohammed's account, Planner and executors
on `sonnet` (observed `claude-sonnet-5-5`), mode Auto. A new `Accounts` project of the same shape.

| # | Step | Observed |
|---|---|---|
| 1 | First plan: vision, map (TypeScript frontend in `web/`), roadmap, plan with E1 `people`, E2 Sign up, E3 Sign in | v1's T1 committed `8acedf8`; Shadows' rerun of its check found `__pycache__` written outside its writes and the executor reported a contract conflict. The Planner made v2 (a `.gitignore`, `python3 -B` in the checks), approved it on the person's word and ran T1 to T4: commits `69bff5e`, `3a23f67`, `a1c56df`, `2f7946c`, each judged and marked done. |
| 2 | `code_where` after v2 | Every symbol `at_done: found`, `live: confirmed` (pyright and `tsc --lsp`). |
| 3 | v3: T1 to T4 unchanged, T5 added (`signOut`) | Shadows did not carry the done marks; the Planner marked T1 to T4 done in v3 naming their earlier commits, with no attempt in v3. **B-190 holds:** their symbols answer `at_done: found` in v3 (P6 showed `unchecked`). |
| 4 | Second plan in a new conversation: password reset through a local file mailer | First turn: `project_state`, `workspace_get`, `workflow_list`, `design_elements`, `decision_list`, `roadmap_propose`, then one `code_where`. Drafting turn: `git ls-files`, then `cat` of 12 files before `plan_edit`. **B-186 step 1 fails again, for a new reason (P8).** It wrote E4 `password_reset_tokens`, E5, E6 and E3 revision 3, duplicating nothing. |
| 5 | Browser (Playwright) | State, Tables (`person_id → people.id`), Screens (labelled arrows both ways), Where (`sign_in`, 16 asks): no console errors. A Where answer for the Planner's long topic shows "Shadows has no record of this." |

## Third run · B-191 and B-192

Same project and database, `shadows serve` restarted on `ecbfb76`.

| # | Step | Observed |
|---|---|---|
| 1 | `code_where` `SignIn` right after the restart | `answered`, both uses in `app_core.py` (the second run's first ask gave `[]`). |
| 2 | A new file `backend/accounts/probe_use.py` importing `SignIn`, asked again | Within 5 s: `uses_total: 4`, the new file listed. Removed afterwards. **B-192 holds.** |
| 3 | New conversation: "people can change their email" (B-186 step 1, the plan's example) | First turn: `roadmap_propose` only. Drafting turn: `project_state`, `workflow_list`, `workspace_get`, ToolSearch loading `code_where`, then `git ls-files` and `cat` of 7 files, `design_elements`, then a `person_ask` (how a change of email proves who the person is). **No `code_where` call: B-186 step 1 fails a third time** with B-191's prompt in place. |

## Fourth run · B-193 (the record sent with the context)

`shadows serve` restarted on `55941d8`. New conversation: "people can change their email; they
re-enter their current password".

| # | Step | Observed |
|---|---|---|
| 1 | The first turn's planning context (`/api/operations/{id}/context`) | `planning_context: heard`. It holds "## Code record": every recorded file by part and layer with its purpose, functions with their signatures (`make_handler (function): core: AppCore -> type[...]`, `open_database (function): path: str -> sqlite3.Connection`), classes by name only (`SignIn (class)`): the contracts record no methods. |
| 2 | The Planner's calls before `plan_edit` | `workflow_list`, `workspace_get`, `project_state`, `design_elements`, then two shell reads: `cat` of 9 files it then plans to change (`openapi.yaml`, `http.py`, `accounts.py`, `app_core.py`, `app.py`, `person.py`, `sqlite_people.py`, the web data access and screens) with an `ls` of `web/src/*` and `backend/tests`. **No `git ls-files` listing and no search for where things are**; no `code_where` call. Then roadmap change, decision, E1 revision, `draft_start`, `plan_edit`: four tasks, no blockers. |

Reading: the Planner now plans from the record which files and parts to touch; it opens the files
it modifies, to see the methods inside the classes, which neither the record nor `code_where` holds.

## Fifth run · B-186 step 3 (a stopped language server)

The pyright process (`node …/langserver.index.js`) killed by hand while `shadows serve` ran.

| # | Step | Observed |
|---|---|---|
| 1 | `code_where` `SignIn`, right after | 0.1 s. The file entry and `SignIn` keep the record's half (`at_done: found`, tasks, commit); `live: {state: unchecked, why: "the language server pyright failed: it exited with no error line"}`, `uses_live: not_asked`. **Step 3 holds.** |
| 2 | The same question every 10 s for 60 s | Unchanged: the exited server is not started again until `shadows serve` restarts (no restart rule in `crates/codebase/contract.yaml`; recorded under "Known and left"). |

Killing only the `pyright-langserver` launcher did not stop the server: its node child kept
answering over the same pipes.

## Logs

`daemon.log` over the run: 0 WARN, 0 ERROR (INFO and DEBUG only). Browser console: no errors on the
State view. Every Planner turn ended `Completed` on `claude-sonnet-5-5`.
