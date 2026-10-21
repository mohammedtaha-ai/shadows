# Provider presets research (2026-10-21)

Facts read from each provider's official documentation on 2026-10-21, for the
settings dialog request (`docs/shared/requests/2026-10-20-settings-dialog.md`).
No API key was used or read; no endpoint was called. Every "could not confirm"
below is a question a real key answers at B-3's acceptance run.

Context: the AYLUL brain is Claude Code. Shadows sets `ANTHROPIC_BASE_URL` to the
provider's URL and passes the key as `ANTHROPIC_AUTH_TOKEN` (sent as
`Authorization: Bearer <key>`), with `ANTHROPIC_API_KEY` set empty
(`crates/shadows-agent/src/profile.rs` `spawn_env_with`,
`crates/shadows-core/src/harness/assistant.rs` `opening`).

## Summary

| Preset | `ANTHROPIC_BASE_URL` | List the models | Auth on the list | Context window in the list | Cheapest key test |
|---|---|---|---|---|---|
| MiniMax | `https://api.minimax.io/anthropic` | `GET https://api.minimax.io/anthropic/v1/models` | `X-Api-Key` | no | the model list |
| Anthropic | `https://api.anthropic.com` | `GET https://api.anthropic.com/v1/models` | `x-api-key` (or `Authorization: Bearer`) + `anthropic-version` | yes, `max_input_tokens` | the model list |
| DeepSeek | `https://api.deepseek.com/anthropic` | `GET https://api.deepseek.com/models` | `Authorization: Bearer` (assumed, see below) | yes, `context_window` | the model list |
| OpenRouter | `https://openrouter.ai/api` | `GET https://openrouter.ai/api/v1/models` (whole catalogue, not per key) | Bearer | yes, `context_length` | `GET https://openrouter.ai/api/v1/key` |
| Moonshot (Kimi) | `https://api.moonshot.ai/anthropic` | `GET https://api.moonshot.ai/v1/models` | `Authorization: Bearer` | yes, `context_length` | the model list |

Every provider's own Claude Code guide uses `ANTHROPIC_AUTH_TOKEN`, which is what
Shadows already sends.

## MiniMax

- Base URL: `https://api.minimax.io/anthropic`. The token-plan Claude Code page
  sets `ANTHROPIC_BASE_URL` to it and the key in `ANTHROPIC_AUTH_TOKEN`; it also
  sets `CLAUDE_CODE_AUTO_COMPACT_WINDOW=524288` and the model
  `MiniMax-M3.1-Flash[1m]` for all four `ANTHROPIC_*MODEL` variables.
  A China-region URL `https://api.minimaxi.com/anthropic` is named by MiniMax
  search results but not on the pages read; not a preset here.
- Models (Anthropic format): `GET https://api.minimax.io/anthropic/v1/models`,
  header `X-Api-Key`, query `limit`, `after_id`, `before_id`. Response
  `{ data: [{ id, created_at, display_name, type }], first_id, last_id, has_more }`.
  No context window field.
- Models (OpenAI format): `GET https://api.minimax.io/v1/models`, Bearer.
  Response `{ object: "list", data: [{ id, object, created, owned_by }] }`. No
  context window.
- Context windows from the Anthropic API page: `MiniMax-M3.1-Flash` 1,000,000;
  `MiniMax-M3` 1,000,000. The example list responses show `MiniMax-M3`,
  `MiniMax-M2.7`, `MiniMax-M2.5` and not `MiniMax-M3.1-Flash`: could not confirm
  whether the live list includes it.
- Usage: the page points to "response `usage`" for token counts; no cost field
  is documented.
- Sources: <https://platform.minimax.io/docs/token-plan/claude-code>,
  <https://platform.minimax.io/docs/api-reference/text-anthropic-api.md>,
  <https://platform.minimax.io/docs/api-reference/models/anthropic/list-models.md>,
  <https://platform.minimax.io/docs/api-reference/models/openai/list-models.md>

## Anthropic

- Base URL: `https://api.anthropic.com`.
- Auth: the API overview lists `Authorization: Bearer <token>` where the token
  "is your API key", required unless `x-api-key` is set; `x-api-key` is a "legacy
  fallback ... still supported". `anthropic-version` (for example `2023-06-01`)
  is required. So Shadows' `ANTHROPIC_AUTH_TOKEN` route works for an Anthropic key.
- Models: `GET /v1/models`, query `limit` (default 20, max 1000), `after_id`,
  `before_id`, `lifecycle`. Response `{ data: [ModelInfo], first_id, last_id,
  has_more }`; `ModelInfo` has `id`, `display_name`, `created_at`, `lifecycle`,
  `line`, `max_input_tokens` (number or null: "Maximum input context window
  size"), `max_tokens`, `capabilities`, `type`.
- Cheapest test: the model list (no tokens spent).
- Usage: not read from the Messages page (truncated before its response
  schema). Could not confirm from the source; the usual `usage` has
  `input_tokens`, `output_tokens`, cache fields and no cost.
- Sources: <https://platform.claude.com/docs/en/api/overview>,
  <https://platform.claude.com/docs/en/api/models-list>

## DeepSeek

- Base URL: `https://api.deepseek.com/anthropic`. The Claude Code page sets
  `ANTHROPIC_AUTH_TOKEN`, `ANTHROPIC_MODEL=deepseek-flash[1m]`,
  `ANTHROPIC_DEFAULT_HAIKU_MODEL=deepseek-flash`,
  `CLAUDE_CODE_AUTO_COMPACT_WINDOW=786432`. Models named: `deepseek-flash`,
  `deepseek-v4-pro`; legacy `deepseek-v4-flash` is still accepted.
- The Anthropic guide lists `x-api-key` as fully supported and ignores
  `anthropic-version`; it does not mention Bearer, though the Claude Code page's
  `ANTHROPIC_AUTH_TOKEN` sends Bearer. No models endpoint under `/anthropic` is
  documented.
- Models: `GET /models` with `{ object: "list", data: [{ id, object, owned_by,
  name, context_window, max_output_tokens, input_modalities, output_modalities,
  effort, api_capabilities }] }`. `context_window` is "the total token capacity
  ... counting both input and output tokens". The page names no host or auth
  header: `https://api.deepseek.com/models` with Bearer is assumed and must be
  confirmed with a key.
- Balance: `GET /user/balance` → `{ is_available, balance_infos: [{ currency,
  total_balance, granted_balance, topped_up_balance }] }` (host not shown).
- Usage in Anthropic-format responses: not stated.
- Sources: <https://api-docs.deepseek.com/quick_start/agent_integrations/claude_code>,
  <https://api-docs.deepseek.com/guides/anthropic_api>,
  <https://api-docs.deepseek.com/api/list-models>,
  <https://api-docs.deepseek.com/api/get-user-balance>

## OpenRouter

- Base URL: `https://openrouter.ai/api` ("including the scheme"), key in
  `ANTHROPIC_AUTH_TOKEN`, and `ANTHROPIC_API_KEY` "explicitly blank" (Shadows
  already does both).
- Models: `GET https://openrouter.ai/api/v1/models`, Bearer. Response `{ data,
  total_count, links: { next } }`; each model has `id`, `name`, `context_length`,
  `pricing` (`prompt`, `completion`), `top_provider` (`context_length`,
  `max_completion_tokens`), `supported_parameters`, and more. It is the whole
  catalogue ("List all models"), not the models a key may use; no key-scoped
  listing is documented.
- Key test: `GET https://openrouter.ai/api/v1/key`, Bearer → `{ data: { label,
  limit, limit_remaining, usage, is_free_tier, ... } }`; `401` for missing or
  invalid credentials.
- Usage: "automatically returns detailed usage information with every
  response": `usage` with `prompt_tokens`, `completion_tokens`, `cost`,
  `cost_details`. Stated for its chat API; could not confirm for the
  Anthropic-format `/api/v1/messages` that Claude Code calls.
- Sources: <https://openrouter.ai/docs/guides/guides/claude-code-integration>,
  <https://openrouter.ai/docs/api/api-reference/models/get-models>,
  <https://openrouter.ai/docs/api/api-reference/api-keys/get-current-key>,
  <https://openrouter.ai/docs/guides/administration/usage-accounting>

## Moonshot (Kimi)

- Base URL: `https://api.moonshot.ai/anthropic` (the API overview's
  "Anthropic-compatible" base). The Claude Code guide uses
  `ANTHROPIC_AUTH_TOKEN`, models `kimi-k3[1m]` and `kimi-k2.7-code` (Haiku tier;
  "requires thinking to be enabled ... or requests fail with a 400"), and
  `CLAUDE_CODE_AUTO_COMPACT_WINDOW=1000000`. A China-region
  `api.moonshot.cn` appears only on the Chinese page; not a preset here. A
  separate "Kimi Code" product uses `https://api.kimi.ai/coding/`: a different
  key and product, not this preset.
- Models: `GET https://api.moonshot.ai/v1/models`, `Authorization: Bearer`.
  Fields `id`, `object`, `created`, `owned_by`, `context_length`,
  `supports_image_in`, `supports_video_in`, `supports_reasoning`. The page shows
  no sample with real values.
- Balance: `GET /v1/users/me/balance` (fields not documented on the page read).
- Usage in responses: not stated on the pages read.
- Sources: <https://platform.kimi.ai/docs/guide/claude-code-kimi>,
  <https://platform.kimi.ai/docs/api/overview>,
  <https://platform.kimi.ai/docs/api/list-models>

## Claude Code facts that bear on the presets

- `ANTHROPIC_AUTH_TOKEN`: "Custom value for the `Authorization` header (the
  value you set here will be prefixed with `Bearer `)". `ANTHROPIC_API_KEY` is
  sent as `X-Api-Key`.
- `CLAUDE_CODE_MAX_CONTEXT_TOKENS`: overrides the context window Claude Code
  assumes for the active model, for models behind `ANTHROPIC_BASE_URL`.
- `CLAUDE_CODE_AUTO_COMPACT_WINDOW`: accepted from `100000` to `1000000`.
  Shadows today sets it to the provider's `context_window`, which its
  validation allows from 1,000 to 10,000,000: values outside Claude Code's
  range are possible.
- Source: <https://code.claude.com/docs/en/env-vars>

## What Shadows itself can see of usage

Shadows sees no provider response: Claude Code makes the requests. What it
receives is ACP (`agent-client-protocol-schema` 1.9.1, in the local Cargo
registry):

- `SessionUpdate::UsageUpdate { used, size, cost: Option<Cost { amount,
  currency }> }`. Shadows maps `used`/`size` (context) and `_meta` model and
  rate limit today (`crates/shadows-agent/src/acp.rs`); it drops `cost`.
- `PromptResponse.usage: Option<Usage { total_tokens, input_tokens,
  output_tokens, thought_tokens, cached_read_tokens, cached_write_tokens }>`,
  behind the crate's unstable feature `unstable_end_turn_token_usage`, which the
  workspace does not enable.

Not confirmed: whether the pinned Claude ACP adapter sends `cost` or the
end-of-turn `usage`, and whether a cost it sends is right for a non-Anthropic
model (Claude Code prices by model name).
