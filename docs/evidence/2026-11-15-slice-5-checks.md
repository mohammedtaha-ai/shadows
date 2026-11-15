# Slice 5 acceptance: checks, returns and messages (B-151)

- Date: 2026-11-15, run by the controller on Windows 11 Pro.
- Build: `feature/slice-5-checks` at `6b4c4d49` (`cargo build -p shadows`), the dev daemon
  (`.claude/launch.json` `daemon`, run from `shadows-run.exe`) and the web client on 5173.
- Agent: Claude Code 2.1.325 through `@agentclientprotocol/claude-agent-acp` 0.101.0, signed in
  on the subscription. Planner `claude-sonnet-5-5`; executors `sonnet`.
- Project: `testing4` (a small Go notes service with git history), workflow `9edc1d92`.
- Logs: `shadows-dev/logs/shadows-20261115T185458Z-30672.log` (first run) and
  `shadows-20261115T185858Z-24044.log` (after the restart).

## Step 1: `checks_live`

Passed before this run (reviewer, commit `11261bd3`): one test, 98.67 s, Sonnet. Three
attempts, both checks confirmed, `task_done` refused `checks_not_passed` and then recorded with a
reason, two replies delivered.

## Step 2: the browser journey

The Planner was asked in the browser for one plan with two independent tasks, started together:

- T1, `Service.Shortest`: c1 `go test ./internal/service/...`; c2
  `powershell -Command "Start-Sleep -Seconds 90; go vet ..."` (timeout 300 s).
- T2, `GET /ping`: c1 `go test ./internal/handler/...`; c2 `cmd /c exit 1`, failing on purpose.

| Time (UTC) | What happened |
|---|---|
| 18:57:47, 18:57:53 | T1 and T2 attempts started, 6 s apart, both `origin = start`. |
| 18:58:19, 18:58:22 | T2, then T1 reported their commits (`32a7121`, `0f9aa0a`), each check reported. |
| 18:58:26–18:58:30 | The Planner's `message_agent` to T1 ran as its own turn on T1's thread; the reply ("`TestShortest`") was delivered to the Planner. |
| 18:58:30.97 | Shadows began T1's checks once both executor threads were quiet. T2's checks waited: one check at a time per project. |
| 18:58:32 | T1 c2 (the 90 s check) started; the daemon was stopped at 18:58:52, killing it, and started again. |
| 18:58:58 | Recovery: `runs_settled interrupted=0`. |
| 19:04:26–19:05:59 | T1 c1 and c2 ran again from the start; both `passed`; notice sent 19:06:03. |
| 19:06:04–19:06:07 | T2's checks ran after T1's: c1 `passed`, c2 `failed (exit 1)`. |
| 19:06:07, 19:06:21 | Two `origin = checks` returns to T2's executor, each in the same session. Both reported the same commit with "nothing needed changing"; both confirmed again with the same results. |
| 19:06:31 | After the second return the notice went to the Planner (`notice_sent_at` on attempt 3 only). |
| 19:06:13, 19:06:38 | The Planner marked T1 done with no reason (every check passed) and T2 done with a reason naming the deliberate c2 failure; `judged_attempt_id` recorded each. |

The task page showed T2's done reason under its title. The Report tab listed three attempts, each
with "Returned by checks" or "Began from Start", the executor's report, and per check the run
line, "Reported" and "Confirmed" outcomes, exit code, the executor's note, HEAD before and after,
and the output behind "Show output".

## Found

1. **Recovery re-notified old attempts.** Migration `0064` adds `notice_sent_at` as `NULL` for
   every attempt that existed before it, so the first recovery after the upgrade took 23 old
   committed attempts (from 2026-11-09 to 2026-11-14, several projects) as unsent. It queued
   their notices again between 19:03:47 and 19:04:26: three of them started new Planner turns in
   `testing1` and `testing2`, two failed with `command conflict` (`runs.confirm_failed`), and the
   rest replayed. The first daemon start of this build (18:54:58) logged no notice before it was
   stopped, so it had probably not reached them yet.
2. **Recovery took 4 min 49 s before T1's rerun.** Nothing is logged between 18:58:58 and
   19:03:47; the recovery confirmations run one after another, and T1's came after the 23 old
   ones above. Why the old ones took that long was not found.
3. T1's executor ran c2 without its `Start-Sleep` prefix and reported it `passed` with that
   note. Shadows' own run used the full line, so nothing passed wrongly.
4. T2 was returned twice although the executor's report already matched Shadows' result and
   the task forbade changing c2: the return rule does not weigh a matching report.

## Not tried

- Two executors running a check at the same moment (here the two turns ended before Shadows'
  confirmation began).
- A message sent while an executor's turn was still running (the message here arrived after T1
  had reported).
- A check whose output exceeds 65,536 bytes; a check `Inconclusive` from a moved HEAD.
