# Slash commands and `/context` readings over ACP: 2026-10-22

Dated facts. This file decides nothing; harness controls §12.8 and the
architecture plan's A-2 and A-7 own the decisions.

## Reproduction

- **Probe:** `docs/evidence/2026-10-22-slash-context-probe.mjs` at `4e5c717`,
  deleted by the commit that adds this file. It drives the adapter over ACP
  directly, the way Core does: `initialize`, `session/new`, `session/prompt`
  with one text block per entry. Every permission request is refused.
- **Versions:** `@agentclientprotocol/claude-agent-acp` 0.81.1 (the repository's
  `harness/claude`), Claude Code 2.1.306, Node 24.15.0, Windows 11.
- **Modes:**
  - `sub`: the Claude subscription, through the signed-in `~/.claude`, with
    `ANTHROPIC_MODEL=claude-haiku-5-5`.
  - `api`: OpenRouter, with model `nvidia/nemotron-3.5-super-vl:free`, base
    `https://openrouter.ai/api`, a fresh `CLAUDE_CONFIG_DIR`, and the key in
    `ANTHROPIC_AUTH_TOKEN`. The key is read from `.env` line by line and never
    printed. `CLAUDE_CODE_MAX_CONTEXT_TOKENS` and
    `CLAUDE_CODE_AUTO_COMPACT_WINDOW` are both `1000000`, as AYLUL sets them
    for a 1M model.
- **Command:** `node acp-probe.mjs <sub|api> <repo> <workdir>`.
- **Runs:** 2026-10-21 21:06 UTC (`sub`) and 21:08 UTC (`api`).
- **Raw logs:** [`2026-10-22-slash-context-probe/`](./2026-10-22-slash-context-probe/)
  (`subscription.json`, `openrouter.json`). Each holds every turn's blocks,
  time, update counts, `usage_update` sequence and the first 600 characters of
  the reply. Each session also has its per-call usage, as Claude Code wrote it
  in its own transcript (`input + cache_read + cache_write / output`; `LOCAL` is
  a local command with no model call).

Three sessions per mode, in one adapter process:

| Session | Turns |
|---|---|
| `readings` | `Reply with exactly: hello`, `/context`, `/context`, `Reply with exactly: ok` |
| `control` | `Reply with exactly: hello`, `Reply with exactly: ok` |
| `command-with-block` | `Reply with exactly: hello`, then `/context` and `[Shadows] Stage: idea.` as two blocks of one prompt, then "list every user message you have received" |

## Figures

Per-call usage from Claude Code's transcripts:

| Mode | Session | Calls |
|---|---|---|
| sub | readings | `2+0r+32037w/85`, LOCAL, LOCAL, `2+32037r+6051w/52` |
| sub | control | `2+19122r+13278w/4`, `2+19122r+15317w/4` |
| sub | command-with-block | `2+32400r+0w/4`, `2+19122r+15322w/335`, `2+34444r+392w/220` |
| api | readings | `12637/22`, LOCAL, LOCAL, `13548/14` |
| api | control | `3965+8672r/22`, `3984+8672r/14` |
| api | command-with-block | `3965+8672r/14`, `3989+8672r/152`, `4077+8672r/852` |

The final `usage_update.used` of each turn (`sub`, then `api`):
- readings: 32124 → (none) → (none) → 38142; 12659 → (none) → (none) → 13562
- control: 32406 → 34445; 12659 → 12670

`/context` readings in the `readings` session:
- `sub`: "Tokens: 32k / 1m" then "33.3k / 1m", with no turn between them.
- `api`: "18.3k / 1m" then "18.7k / 1m".

The first reading on a fresh session took 26.4 s on `sub` and 4.9 s on `api`.
The second took 0.5 s and 2.9 s.

## Answers

**1. Does a `/context` reading reach what the model is sent?** Yes, on both.
- **OpenRouter:** the first call is the same size in both sessions (12,637 input
  tokens). The `ok` call's input is 13,548 after two readings and 12,656
  without them, so the two readings added 892 tokens to the next request.
- **Subscription:** the `ok` call's input is 38,090 after two readings and
  34,441 without them.
- On both, the second reading's own total is higher than the first's, with no
  turn in between.

This is the control run that §12.8's OPEN block names as its trigger.

**2. Does a slash command survive a second block in the same prompt?** No, on
both.
- The command never runs: no `Context Usage` table comes back.
- A model call is made, and the model answers the stage line.
- Asked afterwards to list the messages it received, the model quotes the
  command as plain text: "/context[Shadows] Stage: idea." on `sub`, and
  "/context [Shadows] Stage: idea." on `api`.
- `/context` sent alone runs as a command on both.

**3. Does `used` grow with the conversation on one model?** Yes, in every
session on both modes, from turn to turn. No turn's final figure is below the
one before it.

**4. Does `/context` report the 1M window when it is set?** Yes, on both
("/ 1m"). The context-meter probe at `e0478f7` set
`CLAUDE_CODE_MAX_CONTEXT_TOKENS=200000`, which explains why its totals always
equalled 200,000.

**5. Does a command after leading whitespace run?** No (`sub`, run at
2026-10-22, the same probe with three one-block sessions).
- `" /context"` and `"\n/context"`: a model call is made. The model answers
  that it cannot run `/context` and should be asked to type it.
- `"/context"`: the `Context Usage` table comes back, and no model call is made.

So a command must be sent with its leading whitespace removed. Detecting it
after a trim is not enough.

## Not established

- Whether a model switch inside one session lowers `used`. No run here switches
  model. The drop from 24.9k to 24.8k seen in AYLUL on 2026-10-21 came after a
  switch from MiniMax to nemotron.
- Behaviour on MiniMax. It was not run: the account had hit its token-plan limit.
- Earlier runs of this probe, before 21:06 UTC, got `401 "API key expired."`
  from OpenRouter, with the `.env` key that was later replaced. The context-meter
  probe at `e0478f7` received answers from OpenRouter, so that error does not
  apply to it.
