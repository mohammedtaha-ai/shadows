# Native Codex Git recovery on Windows (B-196)

Date: 2026-11-22 (Asia/Aden). Built Shadows `6306198b0f362c236b4732c1e8925de7267e76fe`
with `cargo build --locked -p shadows --bin shadows --jobs 2`. Ran the measured fixture
on the original `daemon` entry from `.claude/launch.json` with its existing database,
binding `127.0.0.1:4318`. The launched copy matched `target/debug/shadows.exe` by
SHA-256. The API and web listener at `http://localhost:5173/` returned HTTP 200 after
restart, and the original database still listed `testing7`.

The setup attempt on a separate database was stopped before any fixture task started.
All results below come from the disposable `Git Recovery Fixture` project registered in
the original database. The fixture repository had an initial commit and no remote.
The Planner and executors used native Codex `gpt-6.1-sol`, medium effort, in `ask`
(`Approve for me`) mode. The controller sent and read turns through the local HTTP API.

## Outcomes

| Case | Observed |
|---|---|
| Scoped approval for denied Git writes | T1 wrote `fixture/permission.txt`. Ordinary `git add` and `git commit --only` each received `Unable to create .git/index.lock: Permission denied`. The executor requested its configured approval for each exact command. Both approved requests succeeded. The executor reported `2663e7fea69e43c4e8a07593460ce1c220c4544e`; Shadows confirmed `c1 passed` and the Planner marked T1 done. |
| A real index lock | Before Start, the tester staged `fixture/operator-staged.txt` outside the task's `writes` and excluded `/.fixture-continue` in the fixture's Git excludes. T2 wrote `fixture/contention.txt` and waited before Git. The tester exclusively created `.git/index.lock` and wrote the ignored marker at `00:38:57Z`. Staging failed with an existing lock, including the same-command retries after 1, 2, and 4 seconds. The executor reported no approval request and left the fixture file and lock in place. Its content check passed. The attempt ended `Failed` with no commit. |
| Release and readiness | The HTTP task read returned `{ "allowed": true, "code": null, "reason": null }` after the failed attempt released its scope. The tester removed only the lock it created and left the ignored marker. The Planner summarized the automatic failure notice and waited for a request. When asked whether the task could be retried, it called `task_start_status`, reported that Start was allowed and waited for an explicit retry request. |
| Retry and judgment | After the retry request, the Planner called `task_start_status` again and then `task_start`. Attempt 2 kept the matching leftover file and saw the marker. Ordinary Git writes were denied by the sandbox and the approved retries succeeded. It committed `84a4beeaf3c49c786023861199ad54370e8a44bb`. Shadows confirmed `c1 passed`. The Planner read `task_checks`, `task_diff` and `task_file`, then marked T2 done. |
| Personal changes still block | T3's `writes` names `fixture/personal.txt`, which the tester had created as an untracked file. The Planner called `task_start_status` and named `dirty_scope` and that path. A direct Start request returned HTTP `409` with the same code and reason. T3 had zero attempts, and the personal file's SHA-256 stayed equal before and after. |
| Commit scope | `git show --name-only` for the T2 retry named only `fixture/contention.txt`. `git diff --cached --name-only` still named only `fixture/operator-staged.txt`. The marker remained ignored and `.git/index.lock` was absent after the test. |

Each content check was a stable text assertion independent of the Git index and fixture
marker. The permission refusal and lock contention branches
were both observed. The native reviewer approved the scoped permission requests in this
run. An approval rejection was not observed.

## Focused automated verification

- `cargo test -p core-runs --lib the_first_prompt_says_how_an_attempt_ends` failed on
  the old prompt and passed once after B-194.
- `cargo test -p shadows --test mcp_tools task_start_status_matches_the_task_read`
  failed with `tool not found` before registration and passed once after B-195.
- `cargo test -p shadows --test mcp_server the_tool_list_depends_on_the_grant_kind`
  passed once. The OpenAPI artifact had no diff.

## Local evidence anchors

| Record | Identifier |
|---|---|
| Project | `58b860eb-a1cf-4e62-b0e3-34771d32395f` |
| Planner conversation | `ef0aa1b5-8e45-4959-842d-1b44608abc43` |
| Setup v1 workflow | `b8bcb72e-e090-4a63-9c50-2e92c038f9d5` |
| T1 permission attempt | `ac9ddf5a-a88e-4fe7-a9db-f9f1c7b352a4` |
| T2 lock attempt | `b1cc5045-7235-4699-8909-f248672359d6` |
| T2 released-scope readiness turn | `83024ee4-7fd1-4345-bf86-7f8d3d89a380` |
| T2 retry Planner turn | `6b268fe8-dd7a-43a9-b9ca-490426090330` |
| T2 successful retry | `fcabf45a-633d-4a5c-894d-659d71d914e6` |
| T3 personal-file readiness turn | `84925fb6-c933-4dd3-8758-4bd9257a5aa8` |

The Planner and executor evidence is in the native thread entries and workflow attempts.
Use `/api/threads/{id}/entries` and `/api/workflows/{id}/versions/1/tasks/{task}/attempts`
on the original database. The fixture commits are local to the disposable repository.
