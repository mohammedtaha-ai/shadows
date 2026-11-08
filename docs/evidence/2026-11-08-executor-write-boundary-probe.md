# Executor write boundary probe (B-89)

- Date: 2026-11-08, run by the controller.
- Agent: Claude Code 2.1.319 (`claude.exe`), model `haiku`, signed in natively.
- Adapter: `@agentclientprotocol/claude-agent-acp` 0.101.0, launched as Shadows launches it
  (cleared environment plus `PATH`, `HOME`, `TMPDIR`, `LANG`, the Windows essentials and
  `CLAUDE_CODE_EXECUTABLE`), with `tools`, `settingSources: []` and `strictMcpConfig: true`.
- Client: a throwaway ACP client outside the repository that recorded every
  `session/request_permission`, `tool_call` and `tool_call_update`. Nothing in the repository was
  changed to run it, and it is not kept. Each case ran in a fresh disposable git folder.
- Outcome: Mohammed ruled that the executor is built as the Planner is, with only its task's
  tools and permission rules; shell commands are not policed; the commit takes only paths in
  `writes` (design §7).

## Observations

| Case | What was done | What was seen |
|---|---|---|
| Mode | `session/new` with no mode set | The session starts in **`auto`**. In `auto`, Write and Bash ran with **no** permission request. |
| a | Edit inside `src/`, mode `default` | One request, `kind: edit`, `rawInput` = `file_path`, `old_string`, `new_string`; `locations` = the absolute path. The completed call's diff carries only the edited snippet, not the whole file. |
| b | Write outside `src/` | One request with `file_path` and the **whole** `content`; the diff's `newText` is the whole file. |
| c | `echo x > outside_shell.txt` | One `kind: execute` request with the command; `locations` named the redirect target. |
| d | `git status --short` | One request; `git status` lists every change, the agent's and the hand edit's alike. |
| refuse | b and c answered `reject_once` | Both tool calls `failed` ("User refused permission to run tool"); neither file exists on disk; the model reported the refusal and stopped. |
| acceptEdits | Edit in `src/`, Write in `web/` | **No** request for either; `web/x.txt` was created. |
| e | A line appended by hand after the agent's edit | `src/a.txt` holds the agent's change and the hand line; git shows one modified file. |
| f | `git checkout -- src/a.txt`, a `node -e` write, `git mv web/w.txt src/w.txt` | One request each, with no `locations`. The node write appears as `?? src/gen.txt`; the move is staged as `R  web/w.txt -> src/w.txt`. |
| g | Edit through `src/junction/` and `src/slink/`, both pointing to `web/` | The request names the link path, not the target. `realpath` resolves both to `web/w.txt`. Git lists the junction's contents as `?? src/junction/w.txt`, as if inside `src/`, and the symlink as `?? src/slink`. |
| h | `composer create-project laravel/laravel backend` | One request with the command. 105 s. 59 files not ignored; `.env`, `vendor/`, `bootstrap/cache/*.php` and `database/database.sqlite` ignored by Laravel's own `.gitignore`. A whole-folder `git status -uall` took 69 ms and hashing the 59 files 74 ms. |
| i | `docker compose up -d` | Not completed: stopped by Mohammed. Containers belong to the Docker daemon, not to the session, so they outlive it; Mohammed ruled that this is fine. |
| j | `settings.permissions` passed in `_meta.claudeCode.options` | Accepted. `deny: Write(web/**)` refused the Write **and** `echo y > web/y.txt` inside Claude Code, before any request reached the client. `allow: Edit(src/**)` let the edit run with **no** request reaching the client. |
| k | `env` names in a shell command | Only the launch set plus Claude Code's own (`CLAUDE_CODE_*`, `CLAUDE_PID`, ...) and Git Bash's. A canary variable set in the client's own environment did not arrive: the cleared launch holds. With a provider profile, its `ANTHROPIC_AUTH_TOKEN` would arrive the same way. |

| final | The chosen setup: allow `Read`, `Grep`, `Glob`, `TodoWrite`, `Bash`, `Edit/Write/MultiEdit(backend/**)`; every other request refused by the client | Edit and Write in `backend/` ran with no request. A Write to `web/x.txt` reached the client, was refused, and no file was written. `git status` and `echo y > web/y.txt` ran with no request; the second wrote `web/y.txt`, which the commit leaves out. |

## What the rule takes from it

- The mode must be set explicitly; the adapter's initial mode is not a default to rely on.
- Claude Code's own rules carry the task's permissions (cases j and final), so the executor is
  built like the Planner: allowed tools run unasked, anything else is refused. A generator (case
  h) and child processes (case f) are seen only as a command; per-file control of the shell
  would need snapshots around every command, and Mohammed chose not to police commands.
- At the commit, git's status is the whole truth: generator files appear, ignored files do not,
  a rename shows its origin. A junction inside `writes` would smuggle another folder's files
  into the commit, so a link in a committed path refuses it.
- The adapter's `agentFileChangeReport` capability (paths edited by editor tools in a turn, from
  Claude's checkpoints) does not see shell writes and is not used.
