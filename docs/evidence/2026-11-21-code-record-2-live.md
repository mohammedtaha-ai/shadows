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
| P7 | **`code_where`'s answer-level `gaps` stayed empty** when a promised symbol was live `gap` (the hand-deleted `SignIn`); the gap shows only on the symbol. To check against the 2b plan's shape. | `crates/code-record/src/code_where.rs` | open: to check |

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

## Logs

`daemon.log` over the run: 0 WARN, 0 ERROR (INFO and DEBUG only). Browser console: no errors on the
State view. Every Planner turn ended `Completed` on `claude-sonnet-5-5`.
