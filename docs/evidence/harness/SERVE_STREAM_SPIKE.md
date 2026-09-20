# Serve + Stream Spike

**Date:** 2026-09-21
**Status:** Complete. Questions answered; code is throwaway.
**Probe code:** `sandbox/serve-stream-spike/` — labelled throwaway, not a
workspace member, deleted once this report is accepted.

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

## Finding 4 — `claude.exe` spawns a process tree

Observed during a live turn: a single interactive `claude.exe` had seventeen
`claude.exe` children. The spike's own child had none during a pure-text turn,
but the interactive case settles the question.

This is direct confirmation of the existing spec decision: killing the direct
child PID is not termination. Windows Job Objects with breakaway prevented are
required, and the spec was right to refuse Unix process-group membership as
proof. No change needed — the decision is now backed by observation rather than
by reasoning alone.

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
7. Keep the Job Object work as designed — Finding 4 confirms it is necessary.

## Not answered by this probe

- Behaviour when the child is killed mid-turn, and what the `result` line looks
  like on a cancelled turn. Belongs with the process-tree containment work.
- Whether `thinking` blocks should be persisted as `ThreadEntry` rows. They
  appear as durable `assistant` lines with a `thinking` content block, so the
  mechanism exists; whether shadows keeps them is a product decision.
- What happens when the browser disconnects mid-turn. The spike stops reading;
  a real implementation must decide whether the turn continues.
- SQLite WAL under concurrent writers. Separate experiment, unchanged.
