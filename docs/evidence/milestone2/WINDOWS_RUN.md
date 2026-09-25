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

The log records no Approve, no edit of project instructions, and no Connect or
Revoke, so §13.14's steps 4–6 were not run.

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
