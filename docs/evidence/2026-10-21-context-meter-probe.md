# Context meter probe - 2026-10-21

## Reproduction and boundaries

Temporary probe commit: `ce8fa7a` (ignored test `b1_context_meter_probe` in
`crates/shadows-agent/src/acp.rs`, from the B-6 probe at `0bf351f`). The evidence
commit restores `acp.rs` to its pre-probe content (`git diff 1a2b601 --
crates/shadows-agent/src/acp.rs` is empty). Windows, Claude Code 2.1.303, pinned
@agentclientprotocol/claude-agent-acp 0.81.1. Command: `cargo test -p shadows-agent
--lib b1_context_meter_probe -- --ignored --nocapture`, with only OPENROUTER and
API_KEY_MINIMAX loaded from `.env` into that command's environment. One fresh
CLAUDE_CONFIG_DIR home, `settingSources ["user"]`, `CLAUDE_CODE_MAX_OUTPUT_TOKENS=1024`,
`CLAUDE_CODE_MAX_CONTEXT_TOKENS=200000`, `settings.json` with `availableModels`.
No key, prompt text or generated text is recorded except step 6's number.
Step 6's turn did not complete (below). The run took 183 s.

## Raw figures

Usage events are `ms since prompt sent: used/size`. `Usage.used` is the
adapter's context-usage notification; TurnUsage is `PromptResult.usage`.

| Step | Usage timeline | TurnUsage in / out / cache-read / total | Chunks (count, first-last ms) |
|---|---|---|---|
| T1 openai/gpt-4.1-mini, `say hi` | 3693: 0; 4177: 15407; 4200: 15407 (cost set) | 15396 / 11 / 0 / 15407 | 9, 3695-3904 |
| T2 same, 300-word reply | 3045: 0; 5712: 16572; 5721: 16572 (cost set) | 830 / 382 / 15360 / 16572 | 380, 3046-5398 |
| T3 after model change, `say ok` | 5352: 0; 5361: 21248; 5380: 21248 (cost set) | 21230 / 18 / 0 / 21248 | 1, 5357 |
| T4 MiniMax-M3, resumed | none: timed out at 120 s, no events | none | 0 |

`set_option` to `nvidia/nemotron-3.5-super-vl:free` was accepted (no fallback used).

`/context` readings (category tokens; "sum" adds the parsed rows, free space and
autocompact buffer included):

| Reading | Took | System prompt | System tools | Skills | Messages | Free space | Autocompact | Sum |
|---|---|---|---|---|---|---|---|---|
| C1 (after T1) | 18450 ms | 1900 | 28300 | 2000 | absent | 153800 | 14000 | 200000 |
| C1b (right after C1) | 3858 ms | 1900 | 28300 | 2000 | 417 | 153400 | 14000 | 200017 |
| C2 (after T2) | 3905 ms | 1900 | 28300 | 2000 | absent | 153800 | 14000 | 200000 |
| C3 (after T3, other model) | 11305 ms | 1900 | 28300 | 2000 | 1 | 153800 | 14000 | 200001 |
| C4a (MiniMax, resumed, before any turn) | 4481 ms | 2900 | 13100 | 2000 | 3800 | 164300 | 14000 | 200100 |
| C4 (after T4) | 0 ms | rpc error, no reading | | | | | | |

Each `/context` reading emitted 2 Chunks and no Usage event with a changed `used`
(the events list in the probe output shows chunks only).

Transcript counts (files under the home's `projects/`; lines containing the
strings), one transcript file each time:

| Check | Lines | `/context` | `local-command-stdout` | `Context Usage` |
|---|---|---|---|---|
| Step 5 (after phase 1; 4 readings) | 68 | 6 | 7 | 6 |
| Step 6 (after phase 2; 5 readings) | 97 | 8 | 9 | 8 |

## Answers

**Q1. Does a `/context` exchange enter the context or the transcript?** It enters
the transcript: the `/context` command and its `local-command-stdout` output are
written as lines, and the counts grow with readings (`/context` 6 to 8 and
`Context Usage` 6 to 8 between the two checks, which add C4a and the failed C4).
Whether the
model sees them again is not established: C1 to C1b shows Messages going from
absent to 417 with no turn between, then back to absent in C2, so a single
reading's Messages value is not reliable either way. The step 6 question (how
many messages the model counts) got no answer: T4 on MiniMax timed out at
120 s with no event. The number is not available.

**Q2. Is the conversation's total stable?** No. The Messages row is absent in
C1 and C2, 417 in C1b, 1 in C3, and 3800 in C4a. After three turns C3 reads Messages=1, less than C1b's 417, so `/context` does
not grow by one message per turn and does not measure the conversation. The
non-message rows also change with the process: System tools 28300 against 13100,
system prompt 1900 against 2900 between the OpenRouter and MiniMax processes.
Across the model change (C2 to C3) the fixed rows are equal but Messages is not
comparable. Against `Usage.used` (15407, 16572, 21248), which does grow with the
conversation (T3 is large because the new model's request was not cached), the
`/context` totals are a constant 200000 by construction (the free-space row
fills to the window), so the sum says nothing; only the Messages row could carry
a conversation size, and it is unreliable. Option A (read `/context` after each
turn as the ring's value) is not supported by this run.

**Q3. Live token figures during a turn?** No. In every turn the adapter sends
`used` twice and only at the edges: once at about the first chunk (`used=0`),
then once after the last chunk (the real figure), then the cost-bearing copy a
few ms later. In T2 the 380 chunks ran from 3046 to 5398 ms; the figures came
at 3045 (0) and 5712 (16572), nothing between.

**Q4. Time per reading.** 3.9 s typical, 11.3 s (C3), 18.5 s (C1, the first on a
fresh process). C4a, a fresh MiniMax process before its first turn, took 4.5 s,
well inside 60 s. The C4 reading after the timed-out T4 failed at once with an
RPC error (the turn was still running or the connection was gone).

## Not established

T4 timed out, so the resume-visibility question and the post-resume Usage have no
data. A rerun of step 6 alone would answer it. This evidence decides nothing;
Hamoud amends harness controls section 12.8.
