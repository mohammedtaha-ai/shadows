# Hermes settings source inventory for the workspace

2026-10-19. Source inspection only, from the installed Hermes checkout. No personal
configuration, credentials, memory or live administrative schema was read.
This inventory routes UI work; it is not a claim that our REST connection offers
these administrative capabilities.

| Inspected source family | Our destination | Current delivery status |
| --- | --- | --- |
| `SettingsView`: about, billing | App information / future account services | Information only; billing not part of personal pilot |
| `SettingsView`: connections, gateway | Integrations / server connection | Existing account origin only; Hermes administration unproved |
| `SettingsView`: keys, vault | Protected provider/credential settings | No renderer readback; existing backend credential service is separate from Hermes configuration |
| `SettingsView`: providers | Provider/model settings | Disabled until owned bot runtime/configuration path is proved |
| `SettingsView`: keybinds | App shortcuts | Preserve existing native shortcut; extra shortcut configuration later |
| `SettingsView`: notifications | Notifications | Current local notifications/presence remain owned locally; new bot activity later |
| `SettingsView`: plugins | Capabilities / plugin settings | Dynamic desktop and agent-plugin inventories not yet connected |
| `SettingsView`: sessions | Conversations | Existing 0.5.0 account history works; broader session administration later |
| `SettingsView`: `config:*` | Bot configuration sections | Live schema proof is required before writes |
| Manifest: appearance, chat, advanced | Appearance and conversation options | Workspace layout first; no unsupported save claims |
| Manifest: keybinds, notifications, sessions, gateway, about | Corresponding pages above | Same capability boundaries |
| Bot profile editor: SOUL, skills, toolsets, MCP, model/provider | Bot inspector / capabilities | Separate backend management contract required |
| Desktop messaging, jobs, artifacts, voice | Their named future feature pages | Navigation preserves scope; no fake activity/connection |

Inspected files under installed `apps/desktop/src/`:
`app/settings/{types.ts,settings-manifest.ts}`, `contrib/settings-pages.ts`,
`api/profiles.ts`, `plugins/hermes-bots/{profile-config.tsx,model-picker.tsx,profile-ops.ts}`.
The inspected desktop guide identifies a `tui_gateway` JSON-RPC backend. Our
selected `gateway/platforms/api_server.py` REST surface advertises
`admin_config_rw:false`, `jobs_admin:false`, `audio_api:false` in its static flags.

**OPEN - Dynamic setting rows.** Close before any schema-driven or plugin settings
editor ships with a pinned isolated live schema/catalog trial and a row-level
mapping into supported, deferred or platform-specific settings. No saved personal
config is needed for that trial; no missing row is silently treated as supported.
