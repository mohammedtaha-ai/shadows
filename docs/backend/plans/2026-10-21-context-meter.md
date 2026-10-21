# Context meter plan

> **For builders:** each task is a `B-<n>` in [`../stages.md`](../stages.md).
> B-1 is a probe: not product code. A recorded run, then the probe is deleted
> (CLAUDE.md). B-2 is blocked until B-1's answer and a shared request.

**Owner spec:** harness controls §12.8,
[`../../superpowers/specs/2026-09-24-harness-controls-design.md`](../../superpowers/specs/2026-09-24-harness-controls-design.md#128-context-and-account-limits).
Mohammed's ruling (2026-10-21): the ring shows the size of the **conversation**,
as Claude's own Context panel does. It grows with each send, does not change with
the model or with a provider's cache reporting, and falls only on a compaction.
Option A: Core reads the adapter's `/context` after each turn. B-1 decides
whether A is safe.

## Constraints

- Live keys: only `OPENROUTER` and `API_KEY_MINIMAX`, in the repository's `.env`.
  Read only its variable names (`cut -d= -f1 .env`). Load it into one command's
  environment only, with a read loop. Never use `. ./.env`: the file has no final
  newline, and sourcing it once echoed part of a key. Never print,
  log, or commit a value. No prompt text and no generated text go into the
  evidence, except the one number asked for in step 6.
- `shadows.exe` and the AYLUL desktop are running for Mohammed and lock
  `target/debug/shadows.exe`. Build only `-p shadows-agent`. If a build needs
  that file, stop and report; do not stop either process.
- One cargo command at a time.

---

## B-1 · Probe: `/context` as the conversation's size, and live turn tokens

Write the probe as one ignored test in `crates/shadows-agent/src/acp.rs`'s
`tests` module: `b1_context_meter_probe`. Use the B-6 probe as the template. It
is in Git at `0bf351f`, in `crates/shadows-agent/src/acp.rs`
(`b6_live_usage_probe`), and has the same `ClaudeAdapter` paths, profile env,
fresh `CLAUDE_CONFIG_DIR` home with `.claude.json` onboarding, `setting_sources:
["user"]`, `CLAUDE_CODE_MAX_OUTPUT_TOKENS=1024` and `terminate_tree`. Add
`CLAUDE_CODE_MAX_CONTEXT_TOKENS=200000`. Write the home's `settings.json` with
`availableModels` listing every model the phase uses and `model` set to the
first.

The events sink of `Connection::open` records each `HarnessEvent` with its
milliseconds since the current prompt was sent. Print only:
- for each `Usage`: `ms`, `used`, `size`, and whether `cost` is set;
- for `Chunk`: the count and the ms of the first and last one;
- the `PromptResult.usage` (`TurnUsage`) of each prompt.

A `/context` reading is `connection.prompt(session, "/context", &[])`. Collect
its `Chunk` text, `shadows_agent::breakdown::parse` it, and print each
category's name and tokens, nothing else. Give each reading 60 s.

Run it once with both providers, in this order and in one home:

1. **OpenRouter process**, model `openai/gpt-4.1-mini`, base
   `https://openrouter.ai/api`. `SessionStart::New`. Turn 1: `say hi`. Then
   reading **C1**.
2. Reading **C1b** right after C1, with no turn between. If the `/context`
   exchange enters the context, Messages grows here.
3. Turn 2: `Write about 300 words about rivers.` (the live-token question:
   print the full Usage and Chunk timeline). Then reading **C2**.
4. `set_option` the model to `nvidia/nemotron-3.5-super-vl:free`, which is also in
   `availableModels`; if it is refused or the turn fails, print the error kind
   and use `openai/gpt-4o-mini` instead. Turn 3: `say ok`. Then reading **C3**.
   Terminate the process.
5. **Transcript check.** Under the home's `projects/`, for each `*.jsonl`,
   print the line count and how many lines contain `/context`,
   `local-command-stdout` and `Context Usage`. Print counts only.
6. **MiniMax process**, same home, model `MiniMax-M3`, base
   `https://api.minimax.io/anthropic`. `SessionStart::Resume(<session id from
   step 1>)`. Reading **C4a** before any turn (print its time taken). Turn 4:
   `How many messages have I sent you before this one, counting only my
   messages? Answer with one number.` Print the number in the reply (expected 3
   if `/context` is invisible to the model, more if it is seen). Then reading
   **C4**. Repeat the transcript check of step 5. Terminate.

If one step fails, print its error kind and go on with the next. Run:
`set -a; . ./.env; set +a; cargo test -p shadows-agent --lib b1_context_meter_probe -- --ignored --nocapture`.

Then write `docs/evidence/2026-10-21-context-meter-probe.md` in the shape of
`docs/evidence/2026-10-21-acp-usage-probe.md`: reproduction (the probe commit,
versions, command), the raw figures per step, and plain answers to:
- **Q1.** Does a `/context` exchange enter the context or the transcript the
  model sees again after a resume (C1 against C1b, the transcript counts, and
  the step 6 number)?
- **Q2.** Is the conversation's `/context` total stable across a model change
  (C2 against C3) and across a provider change on resume (C3 against C4a), and
  does it grow by about one message per turn (C1, C2, C3, C4)? Compare it with
  each turn's `Usage.used`.
- **Q3.** During a turn, does the adapter send token figures repeatedly while
  the reply streams, or only at the start and the end? Give the Usage timeline
  of turn 2 against its Chunk timeline.
- **Q4.** How long each reading took, and whether C4a (a fresh process before
  its first turn) stayed within 60 s.

Revert the probe in the same evidence commit, so that `acp.rs` matches its
pre-probe content, and name the probe commit in the evidence. The evidence
decides nothing; Hamoud amends §12.8.

## B-2 · The ring shows the conversation's size

Shaped after B-1. If B-1 shows that `/context` stays out of the model's
context and its total is stable across models: after a turn ends on an idle
session, Core reads the breakdown once (the `Sessions::context` path), and
stores its total and categories on the turn's invocation with
`context_source = "conversation"`. When there is no reading, the adapter's
last `used` is kept with `context_source = "last_request"`. The task's exact
`Touch:` list is written when B-1 closes and the shared request is `accepted`.
