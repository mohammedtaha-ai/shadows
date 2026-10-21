# Settings dialog: the backend's part

- Status: proposed by Hamoud, 2026-10-21; waits on the request's acceptance
- Request: [settings dialog](../../shared/requests/2026-10-20-settings-dialog.md)
- Frontend design: [settings dialog design](../../frontend/specs/2026-10-20-settings-dialog-design.md)
- Facts: [provider presets research](../../evidence/2026-10-21-provider-presets-research.md)
- Plan: [settings dialog backend plan](../plans/2026-10-21-settings-dialog-backend.md)

## What this decides

Which parts of the settings dialog Core serves, in which service, and the rules
each part keeps. Core is the daemon (`shadows serve`). The native shell
(`apps/desktop`) is not Core and is named only to say what is its own. The
account server and device management are a separate piece of work (section 8).

| Part | Lives in | Service |
|---|---|---|
| Provider presets, label, per-model context window | daemon | `profiles` |
| Testing a key, listing its models | daemon | `profiles` |
| Agent character colour | daemon | `profiles` |
| The person: name, what AYLUL calls them, language, theme | daemon | `profiles` |
| Usage per provider and model | daemon | `turns` records, `profiles` reports (after a probe) |
| Plugins and skills: one library, enabled per agent | daemon | `profiles` (after a probe) |
| Devices on the account | out of scope (section 8) | - |
| Start with Windows, hide to tray on close | native shell | not Core |

`profiles` owns "AYLUL agent configuration"; everything above is configuration
of AYLUL's agents or of the person they serve, so no new service is created
(no layer before its first user). If `profiles` passes 500 lines in one file,
the split goes by domain inside the folder (`presets.rs`, `probe.rs`,
`person.rs`), never into a second facade.

## 1. Provider presets

1.1 A preset is compiled into Core: `id`, `name`, `base_url`, and how its models
are listed (URL, auth header, response format). The list, in this order:

| id | name | region | base_url |
|---|---|---|---|
| `minimax` | MiniMax | `international` | `https://api.minimax.io/anthropic` |
| `minimax-china` | MiniMax | `china` | `https://api.minimaxi.com/anthropic` |
| `anthropic` | Anthropic | null | `https://api.anthropic.com` |
| `deepseek` | DeepSeek | null | `https://api.deepseek.com/anthropic` |
| `openrouter` | OpenRouter | null | `https://openrouter.ai/api` |
| `moonshot` | Moonshot (Kimi) | `international` | `https://api.moonshot.ai/anthropic` |
| `moonshot-china` | Moonshot (Kimi) | `china` | `https://api.moonshot.cn/anthropic` |

Mohammed's ruling (2026-10-21): MiniMax and Moonshot each offer International
or China, and the person picks one. A preset is one region; two presets share a
`name`, and the client shows the name once with a region choice. `region` is
`international`, `china` or `null` (the provider has one region). The
international URLs are the ones each provider's own Claude Code guide sets as
`ANTHROPIC_BASE_URL`; the China URLs are named by the providers' Chinese pages
and search results, not read on an English guide (evidence file).

1.2 A provider with a preset takes its `base_url` and `name` from the preset.
The input may omit both; a sent `base_url` or `name` that differs from the
preset's is refused (`ProfileInvalid`), so a preset's URL is never typed over.
`preset: null` is Custom: `name` and `base_url` are required and checked by
today's URL rule (https, or http to loopback).

1.3 `label` is optional, trimmed, at most 64 characters, no control
characters; empty reads as `null`. Two providers may share a preset; the label
tells them apart. The provider's slug stays generated and immutable.

1.4 Migration: existing providers whose `base_url` equals a preset's get that
`preset`; the rest stay Custom. `label` starts `null`.

## 2. Models carry their context window

2.1 `Provider.models` becomes `[{ id, context_window }]`; `context_window` is a
`u32`. In `ProviderInput`, `context_window` per model is optional and defaults
to 200,000. Today's id rules stay (1..=20 models, ids unique, 1..=128
characters, no whitespace or control characters, not `default`); a window must
be 1,000..=10,000,000. The provider-level `context_window` field goes from
`Provider` and `ProviderInput`.

2.2 Migration: each existing model id gets its provider's current
`context_window`.

2.3 At opening, `CLAUDE_CODE_MAX_CONTEXT_TOKENS` is the window of the agent's
default model. `CLAUDE_CODE_AUTO_COMPACT_WINDOW` is that window capped at
1,000,000, and is not set when the window is below 100,000: Claude Code accepts
only 100,000..=1,000,000 there (evidence file). `availableModels` in the home's
`settings.json` stays the list of ids.

> **OPEN — window after a model switch.** The window is fixed at opening, so a
> person who switches the session to a model with another window keeps the
> default model's window until the session reopens. Closed when a person
> reports a wrong compaction after a switch, or when the harness gains a
> per-model reopen; then the switch reopens with that model's window.

## 3. Testing a key and listing its models

3.1 `test_provider` sends one read-only request per listing and spends no
tokens. Per preset:

| Preset | Request | Auth | Ids from | Window from |
|---|---|---|---|---|
| `minimax` | `GET https://api.minimax.io/anthropic/v1/models?limit=100` | `x-api-key` | `data[].id` | none |
| `anthropic` | `GET https://api.anthropic.com/v1/models?limit=1000` | `x-api-key`, `anthropic-version: 2023-06-01` | `data[].id` | `max_input_tokens` |
| `deepseek` | `GET https://api.deepseek.com/models` | Bearer | `data[].id` | `context_window` |
| `openrouter` | `GET https://openrouter.ai/api/v1/key`, then `GET https://openrouter.ai/api/v1/models` | Bearer | `data[].id` with `"tools"` in `supported_parameters` | `context_length` |
| `moonshot` | `GET https://api.moonshot.ai/v1/models` | Bearer | `data[].id` | `context_length` |
| `minimax-china` | as `minimax`, host `api.minimaxi.com` | `x-api-key` | `data[].id` | none |
| `moonshot-china` | as `moonshot`, host `api.moonshot.cn` | Bearer | `data[].id` | `context_length` |
| Custom | `GET <base_url>/v1/models` | `x-api-key` and Bearer, `anthropic-version` | `data[].id` | `max_input_tokens` if present |

OpenRouter's catalogue is not scoped to the key, so its key check is
`/api/v1/key`; the catalogue is filtered to models that take tools, since
Claude Code cannot work without them. Anthropic-format pagination
(`has_more`/`last_id`) is followed up to 5 pages.

3.2 The answer is an outcome, not a transport error (like `PLAN_BLOCKED`):

- `ok: true`, `models: [{ id, context_window: u32 | null }]` sorted by id,
  `listed: true`.
- Custom whose `/v1/models` answers 404: `ok: true`, `models: []`,
  `listed: false`: the key was not refused, the person adds models by hand.
- `ok: false`, `reason: "key_refused"` for 401 or 403;
  `reason: "unreachable"` for a connection failure, a timeout or a 5xx;
  `reason: "unexpected"` for any other status or a body that does not parse.
  `status` carries the provider's HTTP status when there was one. `message` is
  Core's own sentence, never the provider's body (it may echo the key).

3.3 Security rules, each held by a test: the key is never stored, logged,
traced or returned; redirects are not followed (a redirect is `unexpected`); a
10-second timeout; a response body over 8 MiB is `unexpected`; a Custom URL
passes the same URL rule as saving. Testing a saved provider without a new key
uses its stored key, read the way the harness reads it (`secrets`' scope
`provider/<slug>`, name `PROVIDER_API_KEY`).

3.4 The HTTP client is `reqwest` with rustls, already in the workspace for
tests: infrastructure, allowed. Parsing the five answers is product logic,
written here.

> **OPEN — the live answers.** OpenRouter and MiniMax (international) passed B-3's
> live model-list test on 2026-10-21 with the keys in `.env`; their documented
> shapes matched. The remaining shapes are accepted on fixtures only. Anthropic, DeepSeek (whose `/models` host and auth
> are assumed), Moonshot and both China presets stay unchecked: each closes when
> Mohammed has a key for it and one live run passes; a different shape amends 3.1.

## 4. Character colour

4.1 `Agent.character_color` is a string, always present. Core holds no list of
colours: an id is 1..=32 characters of `a-z`, `0-9`, `-`, starting with a
letter. A new or migrated agent is `purple`. `AgentInput.character_color` is
optional: absent on create is `purple`, absent on update keeps the stored
value; normalization happens before the replay fingerprint.

4.2 The colours and their palettes belong to the web client and the character
renderer; each draws purple for an id it does not know. Adding a colour needs no
Core change. This is the agreement with the frontend; Core never refuses a
well-formed id it has not heard of.

4.3 The native presence reads `character_color` from the agent list it already
fetches and publishes it as `appearance.color`; that, and the main-only
`refresh_companion_appearance` command, are the native side's.

## 5. The person

5.1 One local record, without an account: `name`, `aylul_calls_me`
(each optional, trimmed, at most 64 characters, no control characters or line
breaks, empty is `null`), `language` (optional; a tag of 2..=3 lowercase
letters, optionally `-` and 2..=8 letters or digits; Core keeps no list) and
`theme` (`dark` | `light` | `system`, default `system`). `PUT` replaces the
record under a `command_id` with replay rules; `GET` answers the defaults
before the first write.

5.2 At an AYLUL opening, when `aylul_calls_me` (else `name`) is set, the
session's system prompt append carries one line naming it; the agent home's
files are not changed. A change applies at the next opening.

5.3 Start with Windows and hide-to-tray are native settings, stored by the
native shell.

## 6. Usage

6.1 Core sees no provider response; Claude Code makes the requests. What
reaches Core is ACP: `UsageUpdate.cost` (dropped today) and, behind an unstable
feature, the turn's `PromptResponse.usage` token counts. Usage is recorded per
turn, in the turn's ending transaction, as `{ thread, agent, provider_slug,
model, tokens_in, tokens_out, cost, currency, recorded_at }`, and reported by
period grouped by provider and by model. `cost` is `null` unless the adapter
sent one and it is trusted for that preset: `anthropic` (Claude Code prices by
Anthropic model name), and `openrouter` if B-6 shows the number is
OpenRouter's own charge. Mohammed (2026-10-21) set no spending limit on
probes and live tests.

> **CLOSED by B-6, 2026-10-21.** The pinned adapter sends end-turn
> input/output/cache counts for OpenRouter and MiniMax. Its cost notification
> is Claude SDK total_cost_usd, not demonstrated OpenRouter billing: both
> providers retain null cost/currency. tokens_in includes inputTokens plus
> cachedReadTokens plus cachedWriteTokens; tokens_out is outputTokens.
> Record one end-turn result, never sum context usage notifications. Missing
> usage must remain unknown internally rather than fabricated. See
> [raw usage evidence](../../evidence/2026-10-21-acp-usage-probe.md).

## 7. Plugins and skills

7.1 Mohammed's ruling (2026-10-21): one library shared by every AYLUL agent;
each agent uses only the items the person enabled for that agent. An item is
added from a folder on the daemon's machine (a skill folder holding `SKILL.md`,
or a plugin folder in Claude Code's standard layout) and copied into Core's
`home_root/library/`; Git and URL sources are not in the first version. At
opening, only the items enabled for that agent are made visible in its home; an
item no longer enabled leaves the home at the next opening. Enablement rows are
keyed by agent slug and item id; deleting an agent removes its rows in the same
write; removing an item from the library removes every agent's enablement of it.

> **CLOSED by B-8, 2026-10-21.** With the pinned runtime and user settings,
> home A exposed its local skill and plugin command in ACP AvailableCommandsUpdate;
> home B exposed neither. A directory marketplace declared in extraKnownMarketplaces
> plus enabledPlugins was sufficient; no install CLI or extra flag was needed.
> This proves discovery, not arbitrary plugin execution. At the next opening,
> projection must also disable previously managed plugin ids and remove stale
> managed files, preserving unrelated user settings. See
> [raw extension evidence](../../evidence/2026-10-21-agent-home-extensions-probe.md).

## 8. Out of scope, and what this plan must not break

8.1 The account server and device management (listing the account's devices,
revoking another device) are a separate piece of work, not planned here.

8.2 The same daemon runs in two places that work together: locally, beside the
Tauri desktop, and on the server, beside the rest of the server backend. The
frontend will send some requests to the local daemon and others to the server
or the daemon there. So nothing in this plan may assume the daemon is only local:

- No route decides who the caller is from the connection (loopback, the OS user,
  the host name). Today's guard stays the only gate; an identity rule, when the
  server needs one, is added in one place for every route.
- No stored id is derived from the machine (host name, MAC, OS user, absolute
  path). Provider and agent slugs, preset ids, colour ids and library item ids
  are machine-independent strings, so records can be shared or synced later.
- No answer carries an absolute path of the daemon's machine. Library items
  answer an id and a name, not their folder; a folder is input only.
- The person record (section 5) is the daemon's record of its person. Whether
  the server's daemon and the local one share it is the account work's
  decision; this plan keeps it one row behind one `profiles` method pair, so
  that work can route or sync it without a second store.
- The key test (section 3) runs on whichever daemon receives it and needs only
  outbound HTTPS. The Custom URL rule's loopback exception means the loopback of
  the daemon running the test, which on the server is the server.
- Nothing here reads or writes native-only state (tray, autostart, companion
  window).

8.3 Not Core's:

- Start with Windows, hide to tray: the native shell.
- Companion `appearance.color` and `refresh_companion_appearance`: the native
  shell.
