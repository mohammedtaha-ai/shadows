# Effort `default` and the adapter's `recommendedValue` — probe

- **Date:** 2026-10-01
- **Adapter:** `@agentclientprotocol/claude-agent-acp` 0.81.1 · **Claude Code:** 2.1.286
- **Node:** 22.22.0 · **OS:** Linux (a cloud container), no `effortLevel` in `~/.claude/settings.json`
- **Daemon:** `shadows serve` built from `main` at `c56e0c8`, on a scratch database, with
  the Shadows repository as the project.
- **Probe:** for part 2, a temporary edit to `crates/shadows-agent/src/acp.rs` made
  `initialize` advertise the capability below. It was reverted after the run and never
  committed.
- **How an effort was read:** from Claude Code's own session transcript
  (`~/.claude/projects/<cwd>/<session>.jsonl`, the `"effort"` field), not from Shadows.

Answers what the effort `default` is, and whether the adapter can drop it (`docs/status.md`
Next, "Effort at once, without `default`").

## 1. `default` is no level: Claude Code picks one per model

The adapter maps the effort `default` to `null` (`toSdkEffortLevel` in
`dist/session-effort.js`), so Claude Code resolves the effort itself: the model's entry in
`settings.modelSettings`, then `settings.effortLevel`, then its built-in value. No settings
file set one here, so these are the built-in values. One turn per model, effort `default`,
each in a new thread:

| Model id | Ran as | Effort used |
|---|---|---|
| `opus` | `claude-opus-5-5` | medium |
| `sonnet` | `claude-sonnet-5-5` | medium |
| `claude-fable-5-1` | `claude-fable-5-1` | high |
| `claude-sonnet-5` | `claude-sonnet-5` | high |
| `claude-opus-5` | `claude-opus-5` | high |
| `claude-fable-5` | `claude-fable-5` | high |
| `claude-opus-4-8` | `claude-opus-4-8` | high |
| `claude-opus-4-7` | `claude-opus-4-7` | xhigh |
| `claude-opus-4-6` | `claude-opus-4-6` | high |
| `claude-sonnet-4-6` | `claude-sonnet-4-6` | high |
| `haiku` | `claude-haiku-4-5-20251001` | none (no effort option) |

The session never reports this value: its effort option reads `default`. So a person who
leaves `default` cannot see the effort a turn runs at, and it differs between models.

Efforts offered, as `PUT …/session/model` answered them: `default, low, medium, high, xhigh,
max` for every model except `claude-opus-4-6` and `claude-sonnet-4-6` (no `xhigh`) and
`haiku` (none). The model menu also offers a model `default` ("Default (recommended)",
described as Opus 5.5).

## 2. `recommendedValue` removes `default`

The adapter reads a JetBrains AIR extension from `initialize`'s client capabilities:

```json
"clientCapabilities": { "_meta": { "jetbrains": { "air": {
  "version": 1, "capabilities": ["recommendedValue"] } } } }
```

With it advertised (`clientSupportsRecommendedConfigValue` in `dist/acp-agent.js`):

- the effort menu has no `default` row, for every model: `low, medium, high, xhigh, max`
  (without `xhigh` on Opus 4.6 and Sonnet 4.6);
- the model menu has no `default` row; the session opened on `opus`;
- every model's effort starts at `medium`: the adapter picks `medium` when the model offers
  it, else the model's first level (`buildEffortConfigOption`). It is the adapter's choice,
  not Claude Code's built-in value of part 1;
- a chosen effort reaches Claude Code: two turns on `opus`, sent with `high` then `low`,
  were recorded in the transcript as `"effort":"high"` then `"effort":"low"`. Both
  completed (`end_turn`).
