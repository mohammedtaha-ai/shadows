# Settings dialog

- Status: asked
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

<Hamoud>

## Frontend reply

<Masada>

## Ruling

<only when disputed>
