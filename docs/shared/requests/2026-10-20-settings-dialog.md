# Settings dialog

- Status: answered
- Asked by: Masada, 2026-10-20
- Screen: [settings dialog design](../../frontend/specs/2026-10-20-settings-dialog-design.md)

## Need

The settings dialog shows, per section, what the person has and only the actions their state
allows. These are the parts it cannot draw from today's API. Shapes are what the screen needs;
names, routes and where each lives (daemon, account server, native shell) are the backend's
call in its answer.

## Fields

### Account: the person and their devices

| Call | Sends | Gets back |
|---|---|---|
| `GET /api/me` | – | `{ name: string \| null, aylul_calls_me: string \| null }` |
| `PUT /api/me` | `{ name, aylul_calls_me, command_id }` | the same object |
| list devices (signed in) | – | `[{ id, name, platform, last_active_at, this_device: bool }]` |
| revoke a device | `{ device_id }` | `{ revoked: true }` |

`me` is local and works without an account. The device calls go through the account server
(today's native `authAction` covers only this device).

### Providers: label, test, models

| Call | Sends | Gets back |
|---|---|---|
| `GET /api/aylul/provider-presets` | – | `[{ id, name, base_url }]` (MiniMax, Anthropic, DeepSeek, OpenRouter, Moonshot) |
| `Provider` gains | – | `preset: string \| null`, `label: string \| null`, `models: [{ id, context_window }]` |
| `ProviderInput` gains | `preset`, `label`, `models: [{ id, context_window? }]`; `base_url` only when `preset` is null | – |
| test a key before saving | `{ preset \| base_url, api_key }` | `{ ok: bool, models: [{ id, context_window }] }` or the provider's error |

The key sent to the test is not stored. Two providers may share a preset with different labels.

### Agents: character colour

| Call | Sends | Gets back |
|---|---|---|
| `Agent` / `AgentInput` gain | `character_color: "blue" \| "red" \| "green" \| "purple" \| "orange" \| "pink"` | same |

### Plugins & Skills

| Call | Sends | Gets back |
|---|---|---|
| list | – | `{ plugins: [Item], skills: [Item] }`, `Item = { id, name, description, enabled }` |
| toggle | `{ id, enabled, command_id }` | `Item` |
| add | the source the backend accepts (path or URL) | `Item` |

### Usage

| Call | Sends | Gets back |
|---|---|---|
| `GET /api/usage?from&to` | period | `{ by_provider: [{ provider_slug, provider_name, label, requests, tokens_in, tokens_out, cost }], by_model: [{ model, provider_slug, requests, tokens_in, tokens_out, cost }] }` |

Cost may be `null` when the provider's price is not known; the screen then shows "–".

### General

| Setting | Values |
|---|---|
| language | `ar` \| `en` |
| theme | `dark` \| `light` \| `system` |
| start with Windows | bool (native) |
| closing the window hides it to the tray | bool (native) |

## Errors

| When | The screen shows |
|---|---|
| Test connection: key refused | "The provider refused this key." and no model list |
| Test connection: provider unreachable | "Could not reach <provider>. Check your connection and retry." |
| Device list while the account server is down | "The account server is unavailable." in the Devices card |
| Usage with no records | "No usage recorded in this period." |

## Backend answer

Hamoud, 2026-10-21. Decisions: [backend spec](../../backend/specs/2026-10-21-settings-dialog-backend-design.md);
tasks: [backend plan](../../backend/plans/2026-10-21-settings-dialog-backend.md), B-1..B-9 in
[`docs/backend/stages.md`](../../backend/stages.md); facts: [provider research](../../evidence/2026-10-21-provider-presets-research.md).
Every Core part is the daemon's, in the `profiles` service
(`crates/shadows-core/src/profiles/contract.yaml`); B-2 and B-5 also touch
`crates/shadows-core/src/harness/contract.yaml`. All routes are tag `aylul`.

### Account: with a change; Devices out of scope

- `me`: **with a change.** `GET /api/me` → `{ name, aylul_calls_me, language, theme, updated_at }`;
  `PUT /api/me` `{ command_id, name, aylul_calls_me, language, theme }` → the same object.
  Names: optional, trimmed, ≤64 characters, no line breaks; `""` reads as `null`. Before the
  first write: all `null`, `theme: "system"`, `updated_at: null`. `aylul_calls_me` (else `name`)
  reaches AYLUL at the next session opening. `language` and `theme` join this record (see
  General). Task B-5.
- Devices and revoking another device: **out of scope** (Mohammed, 2026-10-21). The account
  server and device management are a separate piece of work. Keep the Devices card undrawn.
  For that later work: the same daemon runs locally (beside the desktop) and on the server
  (beside the server backend), and the frontend will call both. Nothing in this answer
  assumes the daemon is only local (spec §8.2). Ids are machine-independent and no answer
  carries a local path.

### Providers: as asked, with changes

- `GET /api/aylul/provider-presets` → `[{ id, name, region, base_url }]`, **with a change**:
  `region` is added (`"international" | "china" | null`). MiniMax and Moonshot each have two
  presets, and the person picks the region (Mohammed, 2026-10-21). Show the name once, with
  an International / China choice. In this order (this closes the design's OPEN):

  | id | name | region | base_url |
  |---|---|---|---|
  | `minimax` | MiniMax | `international` | `https://api.minimax.io/anthropic` |
  | `minimax-china` | MiniMax | `china` | `https://api.minimaxi.com/anthropic` |
  | `anthropic` | Anthropic | null | `https://api.anthropic.com` |
  | `deepseek` | DeepSeek | null | `https://api.deepseek.com/anthropic` |
  | `openrouter` | OpenRouter | null | `https://openrouter.ai/api` |
  | `moonshot` | Moonshot (Kimi) | `international` | `https://api.moonshot.ai/anthropic` |
  | `moonshot-china` | Moonshot (Kimi) | `china` | `https://api.moonshot.cn/anthropic` |

  The international URLs are verified against each provider's own Claude Code guide and
  match F-5's temporary list. The China URLs come from the providers' Chinese pages and
  have not been tested live.
- `Provider` gains `preset: string | null`, `label: string | null`; `models` becomes
  `[{ id, context_window: number }]`, **with a change**: the provider-level `context_window`
  field goes. Existing providers get their preset when their URL matches, and each model
  gets the old provider window. Tasks B-1, B-2.
- `ProviderInput`: `preset`, `label`, `models: [{ id, context_window? }]` (a missing window is
  200,000), **with a change**: `name` and `base_url` become optional. With a preset, send
  neither (or the preset's own values); a different value is a 400. With `preset: null`
  (Custom), both are required. `label`: ≤64 characters, `""` is `null`.
- Test a key, **with a change**: two routes, both answering `200` with an outcome, not an
  HTTP error:
  - `POST /api/aylul/providers/test` `{ preset | base_url, api_key }` (new provider);
  - `POST /api/aylul/providers/{slug}/test` `{ api_key? }` (Edit: without `api_key` it uses
    the stored key, so the key is never shown again).
  - Answer: `{ ok, models: [{ id, context_window: number | null }], listed, reason, status, message }`.
    `ok: false` has `reason: "key_refused"` (your "The provider refused this key.") or
    `"unreachable"` (your "Could not reach <provider>…") or `"unexpected"` (show `message`).
    `listed: false` with `ok: true` means a Custom provider that has no model list: the key
    was not refused; offer add-by-hand only. `context_window` is `null` when the provider
    does not say (MiniMax never does): let the person keep 200,000 or type one.
  - The key is not stored, logged or echoed. `message` is Core's sentence, never the
    provider's body. OpenRouter's list is its whole catalogue, filtered to models that take
    tools; it can be hundreds long, so the checklist needs its search box.
  - Task B-3. Live-tested with real keys for OpenRouter and MiniMax (international) only.
    Anthropic, DeepSeek, Moonshot and the China presets are checked against documented
    shapes until Mohammed has their keys.

### Agents: with a change

`Agent.character_color: string` (always present) and `AgentInput.character_color?: string`.
**Not an enum**: Core keeps no list of colours. It checks only the id's shape (1..=32 of
`a-z`, `0-9`, `-`, starting with a letter). Absent on create is `"purple"`; absent on
update keeps the stored colour; existing agents migrate to `"purple"`. The colour list
(today blue, green, purple, orange, pink) and the palettes are yours and the character
renderer's; each draws purple for an id it does not know. Adding a colour needs no Core
change. The field is added to the agent list the native presence already reads. That
parser must accept a new field, and `appearance.color` in `companion-state` and the
main-only `refresh_companion_appearance` command are **native, not Core's**. Task B-4.

### Plugins & Skills: with a change, after a probe

Mohammed's ruling (2026-10-21): **one shared library, enabled per agent**. Each agent uses only
the plugins and skills the person turned on for it. So the switch lives on the agent, not on
the item. Intended routes (B-9; fixed after the probe B-8):
- `GET /api/aylul/extensions` → `{ plugins: [Item], skills: [Item] }`, `Item = { id, name, description }`
  (the library, without `enabled`);
- `POST /api/aylul/extensions` `{ kind: "plugin" | "skill", path, command_id }` → `Item`. `path` is a
  folder on the daemon's machine. No Git or URL in the first version;
- `DELETE /api/aylul/extensions/{kind}/{id}?command_id=` → `204` (removes it from every agent);
- `GET /api/aylul/agents/{slug}/extensions` → `{ plugins: [AgentItem], skills: [AgentItem] }`,
  `AgentItem = Item + { enabled }`;
- `PUT /api/aylul/agents/{slug}/extensions/{kind}/{id}` `{ enabled, command_id }` → `AgentItem`.

This changes your screen: the on/off switches belong to an agent, for example inside the agent
form or with an agent picker in Plugins & Skills. That is your call. F-10 stays blocked until B-9.

### Usage: with a change, after a probe

- **With a change:** the route is `GET /api/aylul/usage?from&to` (RFC 3339), since only
  AYLUL agents run on providers. Each row adds `currency: string | null` beside `cost`.
  Otherwise your shape.
- Core sees no provider response, because Claude Code makes the requests. What Core gets is
  what the ACP adapter reports, and whether it reports tokens per turn and a cost is unknown.
  A probe (B-6, on OpenRouter and MiniMax) answers that first. `cost` is a number only where
  it can be trusted: Anthropic, and OpenRouter if the probe shows the number is OpenRouter's
  own charge. Otherwise it is `null` and your "–" shows. F-11 stays blocked until B-7.

### General: with a change, half native

- `language` and `theme` are stored in Core with the person (`/api/me`, above), the same in
  the browser and the desktop (Mohammed, 2026-10-21). `theme`: `"dark" | "light" | "system"`. `language` is a
  short tag (`"ar"`, `"en"`) whose shape Core checks without a list, so a third language
  needs no Core change.
- Start with Windows and hide-to-tray on close: **native, not Core's.** Ask the native side.

### Errors

| When | What Core answers |
|---|---|
| Key refused | `200 { ok: false, reason: "key_refused", status: 401 \| 403 }` |
| Provider unreachable | `200 { ok: false, reason: "unreachable", status: null \| 5xx }` |
| Device list, account server down | out of scope (separate account work) |
| Usage with no records | `200` with both lists empty |

### What you can build when

B-1 then B-2 then B-3 unblock F-8. B-4 unblocks F-9. B-5 unblocks F-7's "You" card and
F-12's language and theme. F-12's native switches wait on the native side. F-7's Devices
card is out of this work. F-10 and F-11 wait on B-6..B-9.

## Frontend reply

<Masada>

## Ruling

<only when disputed>
