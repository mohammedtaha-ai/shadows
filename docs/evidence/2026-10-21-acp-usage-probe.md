# ACP usage probe - 2026-10-21

## Reproduction and boundaries

Temporary probe commits: `c6fa634` (feature and instrumentation), `8a5bca5`
(diagnostic run), `0bf351f` (1024 output limit and OpenRouter-only retry).
The evidence commit restores Cargo.toml and acp.rs to their pre-probe contents.
Windows, Node from the installed runtime, Claude Code 2.1.303, pinned
@agentclientprotocol/claude-agent-acp 0.81.1, Rust ACP 2.2.0/schema 1.9.1.
The temporary ignored test ran `cargo test -p shadows-agent --lib
b6_live_usage_probe -- --ignored --nocapture`, with only OPENROUTER and
API_KEY_MINIMAX loaded from .env into that command environment. No credential
was written into a fixture, argument, evidence or commit. Each session used a
fresh CLAUDE_CONFIG_DIR, provider base URL, explicit model and settingSources
["user"]. The only prompt was `say hi`; generated text was not recorded.
A process-tree handle terminated each owned adapter after its turn.

The first OpenRouter call was refused with HTTP 402 before a completed prompt:
the adapter requested 32000 output tokens and the key limit allowed less.
No usage result was inferred from that refusal. The retry set
CLAUDE_CODE_MAX_OUTPUT_TOKENS=1024 and completed. MiniMax completed on the
first diagnostic run. Test harness exit status alone was not the acceptance:
the recorded per-provider outcomes below were both Ended.

## Raw observations

OpenRouter, model openai/gpt-4.1-mini, base https://openrouter.ai/api:

```json
{"used":0,"size":200000,"cost":null}
{"used":15410,"size":200000,"cost":null}
{"used":15410,"size":200000,"cost":{"amount":0.07726999999999999,"currency":"USD"}}
{"totalTokens":15410,"inputTokens":15399,"outputTokens":11,"cachedReadTokens":0,"cachedWriteTokens":0}
```

MiniMax, model MiniMax-M3, base https://api.minimax.io/anthropic:

```json
{"used":0,"size":200000,"cost":null}
{"used":18101,"size":200000,"cost":null}
{"used":18101,"size":200000,"cost":{"amount":0.0905375,"currency":"USD"}}
{"totalTokens":18101,"inputTokens":17885,"outputTokens":41,"cachedReadTokens":175,"cachedWriteTokens":0}
```

The first three objects in each block are UsageUpdate's used, size and raw
cost; the last is raw PromptResponse.usage serialized by the Rust schema.
Both prompt outcomes were Ended. No thoughtTokens field was sent.

## What this establishes

The unstable end-turn feature exposes per-turn input/output and cache counts
on this pinned stack for both providers. UsageUpdate.used is context usage,
not a separate billable input counter. MiniMax totalTokens here equals input
plus output plus cached read/write tokens. Preserve raw components internally;
for the two-column usage report count input plus cachedReadTokens plus
cachedWriteTokens as tokens_in, outputTokens as tokens_out. Do not sum the
context notifications as turn usage.

Cost notifications exist, but are not demonstrated provider charges. Inspection
of the installed adapter's dist/acp-agent.js result handler (around line 3786)
shows amount comes from Claude SDK message.total_cost_usd, currency USD. The
probe did not obtain OpenRouter generation billing or establish equality with
its actual charge. Therefore OpenRouter and MiniMax cost/currency remain null
in the product report; this is the accepted request's existing fallback.
Only Anthropic's own pricing remains eligible under spec section 6, and was
not live-tested here. The adapter's cost is a session report: future recording
must use a per-turn delta if displaying a trusted cumulative cost.

This is one completed fresh-session turn per provider, not evidence of every
cancellation, resume, model-switch or multi-request/tool scenario. Those need
synthetic boundary tests in B-7. No provider integration or product code is
retained from the probe.
