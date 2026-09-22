# Serve + Stream Spike

**Date:** 2026-09-21
**Status:** Complete. Questions answered; code is throwaway.
**Probe code:** was `sandbox/serve-stream-spike/` — labelled throwaway, and
deleted as promised once this report was accepted. It is in Git history:
`git show c47bf38:sandbox/serve-stream-spike/src/main.rs`, or
`git log --all -- sandbox/serve-stream-spike` for every revision of it.
Everything this report establishes is stated here; nothing requires reading
that code.

## Question

Milestone 0 had two parts with no design at all: how `shadows serve` hands a
page to a browser, and what the concrete Claude harness invocation and output
contract actually look like. This probe answered both as cheaply as
correctness allowed. It did not probe persistence, cancellation, process-tree
containment, idempotency, or the error taxonomy.

## Environment

| Component | Version |
|---|---|
| rustc / cargo | 1.96.0 |
| Claude Code CLI | 2.1.278 |
| axum | 0.8.9 |
| OS | Windows 11 Pro 26200 |

`claude` on this machine is a genuine PE64 executable at
`~/.local/bin/claude.exe`. There is no shell wrapper, so `process/` can spawn
it directly with no shell interposed.

## Finding 1 — Serving the web client is not a real problem

A single `axum` route returning `include_str!("index.html")` plus one SSE route
was enough for a working page with live streaming. Total: one file, ~230 lines,
five dependencies, compiled clean on the first attempt, 51s cold build.

**Recommendation.** Milestone 0 does not need a frontend framework, a bundler,
or a dev-server proxy. A hand-written page embedded with `include_str!` covers
the whole milestone: select a project, create/resume a thread, run a turn,
watch it stream, stop it. Choosing a framework now would be deciding a question
the milestone does not ask.

The daemon prints its URL and does not open a browser, matching the spec rule.

## Finding 2 — The stream contract, concretely

Invocation used:

```
claude --print
       --output-format stream-json
       --verbose
       --include-partial-messages
       --model <model>
       --permission-prompts none
       (--session-id <uuid> | --resume <uuid>)
       <prompt>
```

`--verbose` is required with `stream-json`. `--include-partial-messages` is
what turns the stream from message-granular into token-granular.

Output is newline-delimited JSON on stdout, one object per line. Every line
carries `session_id`, and every durable line also carries its own `uuid`.

### The durable / transient split

The stream divides cleanly into four groups. This division is the main result.

| Group | Line types | Volume per turn | Persist? |
|---|---|---|---|
| **Transient** | `stream_event` (`content_block_delta`, `content_block_start/stop`, `message_start/delta/stop`) | ~40–hundreds | **No.** Render only. |
| **Durable** | `assistant`, `user` | one per message | **Yes.** This is `ThreadEntry`. |
| **Turn end** | `result` | exactly one, always last | **Yes.** Closes the Operation. |
| **Operational** | `system/*`, `rate_limit_event` | a handful | Diagnostics/UI, not conversation. |

The decisive property: **a `stream_event` never carries information that the
following `assistant` line does not also carry.** The deltas for a content
block arrive first, then the complete `assistant` line containing the exact
same content assembled, with a stable `uuid` and `timestamp`. Shadows can
therefore forward `stream_event` straight to SSE without touching storage, and
write a `ThreadEntry` only when the `assistant` or `user` line arrives. There
is no reconstruct-from-deltas path to build, and no risk of persisting a
half-formed message.

Concrete shapes observed:

```jsonc
// transient
{"type":"stream_event","event":{"type":"content_block_delta","index":0,
  "delta":{"type":"text_delta","text":"The"}},"session_id":"...","uuid":"..."}

// durable — assistant, tool call
{"type":"assistant","message":{"id":"msg_...","role":"assistant",
  "content":[{"type":"tool_use","id":"toolu_...","name":"Read","input":{...}}],
  "usage":{...}},"parent_tool_use_id":null,"session_id":"...",
  "uuid":"248762c1-...","timestamp":"2026-09-20T22:58:38.633Z"}

// durable — tool result, arrives as role user
{"type":"user","message":{"role":"user","content":[
  {"tool_use_id":"toolu_...","type":"tool_result","content":"..."}]},
  "session_id":"...","uuid":"...","tool_use_result":{...}}

// turn end
{"type":"result","subtype":"success","stop_reason":"end_turn",
  "terminal_reason":"completed","total_cost_usd":0.037,"usage":{...},
  "permission_denials":[],"session_id":"..."}
```

### Turn-end detection is unambiguous

`type == "result"` is emitted exactly once and always last. It carries
`subtype`, `stop_reason`, `terminal_reason`, `permission_denials`, full token
usage and cost. The process then exits 0. Shadows does **not** need a heuristic
for turn end, and does not need to infer success from exit code alone — it has
both a structured verdict and a process status, and can cross-check them.

**What this measurement does not cover, added 2026-09-22.** Every turn measured
here ended `subtype: "success"`. No failing turn was provoked, so no other
`result` subtype was observed and none is recorded anywhere in this repository.
Code that must tell a successful turn from a failing one therefore tests for
`success` and treats everything else as failing, rather than matching a list of
failure subtypes that would be invented rather than measured — a blacklist
would silently record the first unmeasured failure as a completed turn.
`src/planner/mod.rs` names this file for that rule. Any subtype learned from
outside a measurement belongs here, with how it was learned, before code
branches on it.

### Entry identity is supplied, not invented

Every durable line already has a stable `uuid` assigned by the harness. This is
a better `ThreadEntry` identity than anything shadows would generate, and it
makes replay and deduplication on resync trivial. Recommend storing it as the
harness-side identity alongside the shadows-side ordinal.

## Finding 3 — Session continuity is the harness's, not ours

`--session-id <uuid>` on the first turn and `--resume <uuid>` afterwards give
full conversational continuity **across separate OS processes**. Verified three
times, including through the browser: a third turn answered a question about a
file read in the first turn, with no tools and no context re-sent.

`--resume` does **not** re-emit history. It emits only the new turn's lines.

**Consequence that needs a decision.** There are now two records of the same
conversation: the harness's own session store, and the shadows `ThreadEntry`
log. They are not the same thing and can diverge — the harness store is the
model's actual context, the shadows log is the user-facing durable history.
Milestone 0 should state plainly that the harness store is the source of truth
for *what the model remembers*, and the shadows log is the source of truth for
*what the user sees and what survives a daemon restart*. Do not try to make one
reconstruct the other.

## Finding 4 — WITHDRAWN. The process-tree claim was not measured

An earlier version of this report claimed that a single `claude.exe` was
observed with seventeen `claude.exe` children, and presented that as
observational confirmation of the Job Object decision. **That claim is
withdrawn.** It was two mistakes stacked:

1. The process filter matched on the name `claude`, and this machine runs two
   unrelated executables that both match: the Claude desktop application
   (Electron, which runs a couple of dozen processes by design) and the Claude
   Code CLI. The seventeen were almost certainly the desktop application.
2. The apparent nesting came from Task Manager, whose Processes tab groups by
   *application*, not by parent. Reading that indentation as parentage is not
   evidence of anything. Only walking `ParentProcessId` is.

What was actually measured, scoped correctly by descending from the spike
server's own PID: the spawned child had **zero** children during a text-only
turn and during a turn using `Read`. Neither of those tools spawns anything, so
this neither proves nor disproves a tree.

**The open question is narrower than it looked.** Does `claude --print` spawn
child processes when it runs a tool that executes commands, such as `Bash`? A
turn that actually invokes `Bash`, with the tree walked by `ParentProcessId`
from the spike server's PID, would settle it in minutes.

**The spec decision does not depend on this.** Job Objects with breakaway
prevented remain correct, on the reasoning already recorded: the harness runs
tools that execute arbitrary commands, and persisted raw PIDs are unsafe after
restart because of PID reuse. What changed is only that this report no longer
claims to have measured it.

## Finding 5 — Three operational details that would have cost a day each

1. **Child stdin must be closed.** Without `Stdio::null()` the child waits
   three seconds for stdin that never arrives, then warns and proceeds. Every
   turn would silently pay 3s. `process/` must always null the child's stdin
   unless `--input-format stream-json` is in use.

2. **`system/api_retry` exists.** One turn stalled and emitted
   `{"type":"system","subtype":"api_retry"}` before recovering; that turn took
   52s against ~10s for the others. If the daemon does not surface this event,
   the UI looks frozen with no explanation. It must be forwarded.

3. **`rate_limit_event` carries five-hour and seven-day utilisation.** Free
   operational signal, worth surfacing rather than discarding.

## Finding 6 — The harness binary has no single identity on this machine

The probe invoked whatever `claude` resolved to on `PATH`:
`~/.local/bin/claude.exe`, version 2.1.278. That is **not** the binary the
Claude desktop application uses. The desktop application ships its own copy
under `%LOCALAPPDATA%\Packages\Claude_*\LocalCache\Roaming\Claude\claude-code`,
and keeps more than one version of it side by side — plausibly a new version
alongside the previous one while the new one settles.

So the stream contract documented above is the contract of *one* installation
at *one* version. A different install, or a silent auto-update, can change
`system/*` subtypes or add fields without any signal to us.

**Recommendation.** `shadows` must not resolve its harness through `PATH`. The
harness executable path is explicit configuration, and the harness version is
read when an Operation starts and recorded with it. Without that, the contract
can change underneath a running install and the first symptom is a blank page
rather than an error.

## Recommendations for the Milestone 0 plan

1. Serve the web client as an `include_str!` page from `protocol/`. No
   framework, no bundler.
2. Model the harness contract as four stream classes, not one. Only
   `assistant`, `user` and `result` reach `storage/`.
3. Take `uuid` from the harness as the durable entry's harness-side identity.
4. Use `--session-id` / `--resume` for continuity and record the session id on
   the thread. State the two-records consequence explicitly in the plan.
5. Detect turn end from the `result` line, cross-checked against process exit.
6. Null the child stdin; forward `system/api_retry` and `rate_limit_event`.
7. Keep the Job Object work as designed. Finding 4 does not support it by
   measurement, but the original reasoning stands on its own.
8. Resolve the harness executable from explicit configuration, never `PATH`,
   and record its version on every Operation.

## Not answered by this probe

- Whether `claude --print` spawns a process tree when a command-executing tool
  such as `Bash` runs. See Finding 4; cheap to settle, not settled here.
- Behaviour when the child is killed mid-turn, and what the `result` line looks
  like on a cancelled turn. Belongs with the process-tree containment work.
- Whether `thinking` blocks should be persisted as `ThreadEntry` rows. They
  appear as durable `assistant` lines with a `thinking` content block, so the
  mechanism exists; whether shadows keeps them is a product decision.
- What happens when the browser disconnects mid-turn. The spike stops reading;
  a real implementation must decide whether the turn continues.
- SQLite WAL under concurrent writers. Separate experiment, unchanged.
