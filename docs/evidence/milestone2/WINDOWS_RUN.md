# Milestone 2 on Windows — Mohammed's run

- **Date:** 2026-09-25
- **Build:** `shadows serve --debug`, a debug build of branch `milestone-2/plan-workflow` at `dfa91d4`
- **Harness:** adapter 0.81.1, Claude Code 2.1.281 (`harness.versions`)
- **Database:** the Milestone 1 database, migrated at start without error
- **OS:** Windows 11

## What Mohammed did

Read from the debug log `shadows-20260925T174327Z-20464.log`:

- He opened one Milestone 1 conversation and created a new one.
- He sent eight messages. Every turn ended `success` / `end_turn`.
- The Planner used Shadows' MCP server: 11 `POST /mcp`, all `200`. Every MCP
  connection negotiated `2026-07-28` (`server/discover`), not the older
  `initialize` fallback the probe saw (`MCP_PROBE.md` §4).

His verdict: the result is good as a first version; the larger idea behind
Shadows is not in it yet.

That log records no Approve, no edit of project instructions, and no Connect
or Revoke. Steps 4–6 were run after the fix below. Steps 1–3 were used, not
checked item by item. The plan has 12 tasks and 18 links, and the Planner showed
it with `plan_show`, but whether a cycle was refused was not checked.

## Step 4: an approved plan is not edited; the edit makes v2 — pass

On the fixed build (log `shadows-20260925T185231Z-11580.log`), for a 12-task,
18-link plan titled "موقع توصيل طعام - المرحلة 1 (MVP)":

1. Mohammed pressed **Approve**. The daemon logged
   `POST /api/workflows/818462e4…/approve 200`.
2. He asked for a change. That turn made five `POST /mcp` calls and ended `success`.

The API afterwards:

| | v1 `818462e4…` | v2 `e83a79f6…` |
|---|---|---|
| state | `Frozen`, `frozen_at` 18:53:40 | `Draft` |
| revision | 1: the original 30 changes (12 tasks added, 18 links) | 1: "4 changes: updated T2, updated T8, updated T9, updated T12" |
| lineage | `next` = v2 | `previous` = v1 |

v1 kept its 12 tasks and 18 links, and its last edit is still the one that
created it.

## Step 5: project instructions reach the Planner — pass for a new conversation

At 18:59:58 Mohammed saved project instructions number 1: "ابداء كل رد ب كلمه
حاضر" (start every reply with the word حاضر). He then opened a new conversation
`fa79fc78…` and sent two messages:

| Mohammed | The Planner's reply, first words |
|---|---|
| هاي | حاضر، أهلاً! 👋 … |
| كيفك | حاضر، أنا بخير، شكراً لسؤالك … |

A conversation opened after the change gets the instructions when its Claude
session is created (§13.8).

**A conversation opened before the change gets them too.** The plan
conversation `ffa4ba70…` was created before the instructions existed, so its
Claude session holds none; `MCP_PROBE.md` §3 found that a resume cannot add
any. At 19:05:24 Mohammed wrote "شكرا لك" in it, and the reply began
"حاضر، العفو 🙏". The instructions reached it as the context block of the next
turn (§13.8).

## Step 6: Connect — the external Claude Code connects

At 19:02:07 **Connect** made a project grant:
`POST /api/projects/…/mcp-grants 200`, then `kind: project, revoked_at: null`.

Mohammed ran the command it gave in a separate Claude Code session, which
reported:

- the server was added as `shadows` at `http://127.0.0.1:4318/mcp`, local
  scope, for the project `E:\Globalprojects\shadows`;
- `claude mcp get shadows` shows it `Connected`, and the daemon logged two
  `POST /mcp 200` at 19:02:53;
- a session that is already running cannot see a server added after it
  started, so the tools are available from the next session.

That Claude Code also warned that the bearer is stored in plain text in
`~\.claude.json`, and that `claude mcp get` prints it in full.

## Step 6: Revoke — the external Claude Code is refused

- 19:06:03: **Revoke**, `DELETE /api/mcp-grants/33e2a473… 200`; the grant now
  has `revoked_at` 19:06:03.
- 19:06:32: a new Claude Code session with the old bearer tried twice:
  `POST /mcp 401`, twice.

That session told Mohammed that the `shadows` server was not connected because
it had refused the `Authorization` token with HTTP 401. It guessed that the
grant was revoked or that the daemon had another database, and said it could
not call the tools. This is what `MCP_PROBE.md` §5 saw: a failed connection,
no OAuth.

**Not observed:** a session that was already using the tools when the grant
was revoked. That session had started after the Revoke. The daemon refuses a
revoked bearer on every request (§13.7), so such a session's next call gets
the same 401; what Claude Code shows the person then was not seen.

## Watch list

- **Tool lines.** The adapter titles Shadows' tools `mcp__shadows__<name>`,
  which is the form the client turns into "Plan started" and "Plan edited". The
  plan conversation holds `draft_start` ×2, `plan_edit` ×2, `workflow_get` ×5
  and `plan_show` ×2 under those titles.
- **The Planner inherits the person's Claude configuration**, as
  `MCP_PROBE.md` §1 found: in the plan conversation it wrote two files to
  Claude Code's auto-memory for `F:\testing`
  (`~\.claude\projects\F--testing\memory\`), outside the project directory.

## Defect: the harness could not start — fixed

Opening the Milestone 1 conversation failed four times in a row, each a `502`:

```text
17:45:21 ERROR http.failure error=the harness could not start cause="harness setup timed out"  latency_ms=5741
17:45:28 … latency_ms=5257
17:45:43 … latency_ms=5229
17:45:51 … latency_ms=5276
```

Fourteen minutes later the same opening took 2813 ms and succeeded.

**Not new.** The Milestone 1 Windows logs show the same failure on 2026-09-24
(`502` at 5108 ms and 5140 ms). Every opening in both runs took between 2606
and 5741 ms; the bound was 5 s.

**Where the time goes.** The debug log did not contain the adapter's stderr:
its lines are logged under the target `harness.stderr`, which the debug filter
`shadows=debug,info` left out, although spec §12.2 says every line reaches the
debug log. With `RUST_LOG=shadows=debug,harness.stderr=debug,info`, two cold
openings on a restarted daemon:

```text
18:42:05.644 process.spawn node.exe cwd=F:\testing
18:42:06.112 [session/models] phase=read-transcript durationMs=23 messages=53
18:42:06.196 [session/create] phase=settings       durationMs=82
18:42:06.210 [session/create] phase=prepare-query  durationMs=14
18:42:07.292 rmcp: Service initialized (Shadows' /mcp, protocol 2026-07-28)
18:42:10.392 [session/create] phase=sdk-initialize durationMs=4182 totalMs=4279
                                          → the opening returned 200 after 5071 ms
18:42:10.817 process.spawn node.exe (the second conversation)
18:42:15.836 process.terminate                     → 502 after 5358 ms
```

Nearly all of an opening is Claude Code's own `sdk-initialize`: it starts with
the person's configuration (plugins, hooks, MCP servers), so its cost depends
on the machine, not on Shadows. Shadows' own MCP server answered within about
1.1 s of the spawn.

**Fix.** The bound is `SessionsConfig::setup_wait`, 20 s (spec §12.2), and
`--debug` now logs `harness.stderr`. `tests/sessions.rs` covers both sides:

- a harness slower than `setup_wait` fails and leaves no adapter;
- the default outlasts a harness that takes 1.5 s to resume.

With the old 5 s bound hard-coded, the first test fails.
